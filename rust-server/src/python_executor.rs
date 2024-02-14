use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use tokio::net::UnixListener;
use memmap2:: MmapOptions;
use pyo3::types::PyList;
use tokio::sync::Mutex;
use pyo3::prelude::*;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[pyclass]
pub struct CandleStick {
    #[pyo3(get, set, name = "t")]
    pub timestamp: u64,
    #[pyo3(get, set, name = "o")]
    pub open: f64,
    #[pyo3(get, set, name = "c")]
    pub close: f64,
    #[pyo3(get, set, name = "h")]
    pub high: f64,
    #[pyo3(get, set, name = "l")]
    pub low: f64,
    #[pyo3(get, set, name = "v")]
    pub volume: f64,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: std::vec::Vec<String> = std::env::args().collect();
    let algorithm_id = args[1].to_string();
    let run_every_sec = args[2].parse::<u64>()?;

    // Retrieve data from shared memory.
    let shmem_path = &*format!("tmp/shmem/{}.bin", algorithm_id);
    let memfile = std::fs::File::open(shmem_path).expect("Failed to open memfile.");
    let memfile_metadata = memfile.metadata().expect("Failed to get metadata memfile.");
    let mapped_data = unsafe {
        MmapOptions::new()
            .len(memfile_metadata.len() as usize)
            .map(&memfile)
            .expect("Failed to map file to memory.")
    };
    let serialized_data = &mapped_data[..memfile_metadata.len() as usize];
   
    // Create vec of klines from data in shared memory.
    let data : std::vec::Vec<CandleStick> = serde_json::from_slice(serialized_data).expect("Failed to deserialize.");

    // Create UnixSocket to receive data from algorithm and to send result back.
    let unix_socket_path = &*format!("tmp/sockets/{}.sock", algorithm_id);
    std::fs::remove_file(unix_socket_path).unwrap_or_default();
    let listener = match UnixListener::bind(unix_socket_path) {
        Ok(lis) => lis,
        Err(e) => {
            return Err(Error::StreamError(format!("Could not connect to UnixSocket: {}", e)));
        }
     };

    // Make a separate thread to countdown run_every_sec.
    // If the counter is not null the algorithm will not execute.
    let counter = Arc::new(Mutex::new(run_every_sec));
    let counter_clone = counter.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            
            let mut counter_guard = counter_clone.lock().await;
            if *counter_guard > 0 {
                *counter_guard -= 1;
            }
            drop(counter_guard);
        }
    });

    // Accept connection.
    while let Ok((mut stream, _)) = listener.accept().await {
        let mut data_clone = data.clone();
        let algorithm_id_clone = algorithm_id.clone();
        let counter_clone = counter.clone();
       
        // Don't close connection. Keep reading.
        tokio::spawn(async move {
            loop {
                let mut buffer = [0; 1024];
                match stream.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        // Data structure we are sending back.
                        /*
                        #[derive(Serialize)]
                        struct Data {
                            result: f64,
                            last_candlestick: CandleStick,
                        }*/
                        
                        // kline received from UnixSocket.
                        let received_data = String::from_utf8_lossy(&buffer[..n]);
                        let candlestick : CandleStick = match serde_json::from_str(&received_data) {
                            Ok(c) => c,
                            Err(_) => continue,
                        };

                        // Add received kline to all data.
                        data_clone.push(candlestick.clone());
                        
                        // Execute the Python code with the given data.
                        let result = match execute(data_clone.clone(), algorithm_id_clone.to_string(), (counter_clone.clone(), run_every_sec)).await {
                            Ok(r) => r,
                            Err(e) => {
                                match e {
                                    Error::CounterError => continue,
                                    _ => panic!("Could not execute PythonCode: {}", e),
                                }
                            }
                        };

                        // Write result of Python code back to UnixSocket.
                        stream.write_all(result.to_string().as_bytes()).await.expect("Failed to send");

                    },
                    Err(_) => break,
                }
            }
        });
    }

    Ok(())
}

// Execute the Python code.
async fn execute(data: std::vec::Vec<CandleStick>, python_file: String, counter: (Arc<Mutex<u64>>, u64)) -> Result<f64, Error> {

    let mut counter_guard = counter.0.lock().await;
    if counter.1 > 5 && *counter_guard > 0 {
        return Err(Error::CounterError);
    } else {
        *counter_guard = counter.1;
    }
    drop(counter_guard);

    pyo3::prepare_freethreaded_python();

    // Retrieve Python code from file.
    let mut python_code = match std::fs::read_to_string(format!("trading_algos/{}.py", python_file)) {
        Ok(code) => code,
        Err(e) => {
            return Err(Error::PyExecutorError(format!("Error reading Python code: {}", e)));
        }
    };

    // Check if Python code contains blacklisted keywords.
    if !arbitrary_code_is_secure(&python_code) {
        return Err(Error::PythonCodeError("Code contained unsafe elements.".into()));
    }

    // Import allowed libraries into Python code.
    import_allowed_libraries(&mut python_code);

    // Call Python function func.
    let result = Python::with_gil(|py| {

        // Convert data of klines to a PyList so it can be passed to Python code.
        let py_candlesticks: Vec<Py<CandleStick>> = data.into_iter().map(|candlestick| {
            Py::new(py, candlestick.clone()).unwrap()
        }).collect();
        let py_list = PyList::new(py, py_candlesticks);
        
        // Create PyModule.
        let fun: Py<PyAny> = match PyModule::from_code(
            py,
            &*python_code,
            "",
            "",
        ) {
            Ok(f) => {
                match f.getattr("func") {
                    Ok(a) => a.into(),
                    Err(e) => {
                        return Err(Error::PythonCodeError(format!("Error finding func-binding in Python code: {}", e)));
                    }
                }
            },
            Err(e) => {
                return Err(Error::PythonCodeError(format!("Error generating PyModule: {}", e)));
            }
        };

        // Call Python function.
        let f = match fun.call1(py, (py_list,)) {
            Ok(f) => f,
            Err(e) => {
                return Err(Error::PythonCodeError(format!("Error calling Python func: {}", e)));
            }
        };
        let result = match f.extract::<f64>(py) {
            Ok(r) => r,
            Err(e) => {
                return Err(Error::PythonCodeError(format!("Error extracting result from Python func: {}", e)));
            }
        };
        
        Ok(result)
    });

    let result = match result {
        Ok(r) => r,
        Err(e) => {
            return Err(Error::PythonCodeError(format!("Error getting result from Python code: {}", e)));
        }
    };

    Ok(result)
}
    
// Import allowed libraries into the Python code.
fn import_allowed_libraries(code: &mut String) {
    let allowed_libraries = vec![
        "math",
        "numpy",
        "pandas",
    ];

    for lib in allowed_libraries {
        code.insert_str(0, &*format!("import {}\n", lib));
    }
}

// We execute arbitrary Python scripts. Even if this is supposed to be a internal
// application we need to secure this somehow. We blacklist specific Python keywords like
// "import", "exec", "os",... so no dangerous code can be executed.
// We import popular mathematical libraries so those can be used.
fn arbitrary_code_is_secure(code: &String) -> bool {
    let blacklist = vec![
        "import",
        "read",
        "write",
        "file",
        "exec",
        "eval",
        "socket",
        "http",
        "requests",
        "urllib",
        "sys",
        "traceback",
        "__",
    ];

    let re_find_commented_lines = regex::RegexBuilder::new(r"^\s*#.*\n?").multi_line(true).build().unwrap();
    //let code_without_comments = re_find_commented_lines.replace_all(code, "");

    let code_without_comments = re_find_commented_lines.replace_all(code, |caps: &regex::Captures| {
        if caps.get(0).unwrap().as_str().contains('\'') {
            caps.get(0).unwrap().as_str().to_string()
        } else {
            "".to_string()
        }
    });

    for keyword in blacklist {
        match code_without_comments.find(keyword) {
            Some(_) => { return false; },
            None => (),
        }
    }

    true
}


// Error type for PyExecutor.
#[derive(Debug, PartialEq)]
pub enum Error {
    CounterError,
    PyExecutorError(String),
    ParseError(String),
    PythonCodeError(String),
    StreamError(String),
}

impl std::error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::ParseError(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::PyExecutorError(e.to_string())
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(e: std::num::ParseFloatError) -> Self {
        Error::ParseError(format!("line: {} {}", line!(), e.to_string()))
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Self {
        Error::ParseError(format!("line: {} {}", line!(), e.to_string()))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::CounterError => write!(f, ""),
            Error::PyExecutorError(error_msg) => write!(f, "\x1b[31m[Error] PyExecuto - PyExecutorError: {}\x1b[0m", error_msg),
            Error::ParseError(error_msg) => write!(f, "\x1b[31m[Error] PyExecutor - ParseError: {}\x1b[0m", error_msg),
            Error::PythonCodeError(error_msg) => write!(f, "\x1b[31m[Error] PyExecutor - PythonCodeError: {}\x1b[0m", error_msg),
            Error::StreamError(error_msg) => write!(f, "\x1b[31m[Error] PyExecutor - StreamError: {}\x1b[0m", error_msg),
        }
    }
}


// Testing the PyExecutor.
#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_arbitrary_code_is_secure() {
        let safe_code = r#"
def func(data):
    return 0
            "#;

        let unsafe_code = r#"
import os
def func(data):
    return 0
            "#;

        let test1 = arbitrary_code_is_secure(&safe_code.to_string());
        let test2 = arbitrary_code_is_secure(&unsafe_code.to_string());
        assert_eq!(test1, true);
        assert_eq!(test2, false);
    }

    #[tokio::test]
    async fn test_execute() {
        let data = vec![
            CandleStick {
                timestamp: 1707524880000,
                open: 3.0,
                close: 6.0,
                high: 6.0,
                low: 2.5,
                volume: 0.18006000,
            },
            CandleStick {
                timestamp: 1707524940000,
                open: 6.0,
                close: 2.0,
                high: 10.0,
                low: 1.6,
                volume: 0.04270000,
            },
            CandleStick {
                timestamp: 1707525000000,
                open: 2.0,
                close: 5.0,
                high: 5.0,
                low: 2.0,
                volume: 0.29519000,
            },
        ];

        let counter_param = (Arc::new(Mutex::new(0)), 0);
        
        let test1 = execute(data.clone(), "test/test1".into(), counter_param.clone()).await;
        let test2 = execute(data.clone(), "test/test2".into(), counter_param.clone()).await;
        
        assert_eq!(test1, Ok(0f64));
        assert_eq!(test2, Err(Error::PythonCodeError("Code contained unsafe elements.".into())));
    }
}
