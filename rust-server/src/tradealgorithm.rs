// TradeAlgorithm is used to interpet the Python files in which you
// can create your custom trading algorithm. Each algorithm takes a candlestick
// as parameter.

use super::*;

lazy_static! {
    pub static ref PROCESS_HANDLES: Mutex<std::collections::HashMap<String, std::process::Child>> = Mutex::new(std::collections::HashMap::new()); 
}

// All the trading algorithms rely on candlestick charts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CandleStick {
    pub timestamp: u64,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: f64,
}

// X-axis timespan of the chart of the algorithm.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum ChartInterval {
    All,
    Hourly,
    Daily,
}
impl std::str::FromStr for ChartInterval {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ALL" => Ok(ChartInterval::All),
            "HOURLY" => Ok(ChartInterval::Hourly),
            "DAILY" => Ok(ChartInterval::Daily),
            _ => Err("Parse error for ChartInterval."),
        }
    }
}


// A TradeAlgorithm has a Python-script containing the algorithm we want to test.
// Each algorithm gets a specific amount of funds assigned to play with.
#[derive(Clone)]
pub struct TradeAlgorithm {
    pub description: String,
    pub id: String,
    pub start_funds: f64,
    pub interval: String,
    pub run_every_sec: i32,
    pub prepend_data: i32,
}

impl TradeAlgorithm {
    //Create a new trading algorithm and insert into database.
    pub async fn new(id: String, description: String, funds: f64, interval: String, run_every_sec: i32, prepend_data: i32, user_id: i32, psql: Psql) -> Result<Self, tradealgorithm::Error> {

        // Check if interval is valid.
        if !vec!["1s", "1m", "5m", "15m", "30m", "1h", "2h", "12h", "1d", "3d"].contains(&&*interval) {
           return Err(tradealgorithm::Error::AlgorithmError("Invalid interval given".into()));
        }

        let query = psql.lock().await
           .query("
                INSERT INTO algorithms 
                    (id, description, start_funds_usdt, interval, run_every_sec, user_id, prepend_data)
                VALUES
                    ($1, $2, $3, $4, $5, $6, $7)
            ", &[&id, &description, super::sqlf64!(funds), &interval, &run_every_sec, &user_id, &prepend_data]).await;

       match query {
           Ok(_) => (),
           Err(e) => {
                return Err(tradealgorithm::Error::DatabaseError(e.to_string()));
           }
       };

        Ok(TradeAlgorithm {
            id: id.into(),
            description: description.into(),
            start_funds: funds,
            interval: interval,
            run_every_sec: run_every_sec,
            prepend_data: prepend_data,
        })
    }

    // Delete algorithm from database.
    pub async fn delete(&self, psql: Psql) -> Result<(), tradealgorithm::Error> {
        let query = psql.lock().await
           .query("
                DELETE FROM algorithms
                WHERE
                    id = $1
            ", &[&self.id]).await;

        match query {
            Ok(_) => {
                std::fs::remove_file(format!("trading_algos/{}.py", self.id))?;
                Ok(())
            },
            Err(e) => {
                Err(tradealgorithm::Error::DatabaseError(e.to_string()))
            }
        }
    }

    // Retrieve trading algorithm with 'id' from database.'
    pub async fn get(id: String, psql: Psql) -> Result<Self, tradealgorithm::Error> {
       let query = psql.lock().await
           .query("
                SELECT
                    id, description, start_funds_usdt, interval, run_every_sec, prepend_data
                FROM
                    algorithms
                WHERE
                    id = $1::TEXT
            ", &[&id]).await;

       match query {
            Ok(q) => {
                if q.len() == 0 {
                    return Err(tradealgorithm::Error::AlgorithmError("Algorithm not found".into()));
                }

                Ok(TradeAlgorithm{
                    id: q[0].get("id"),
                    description: q[0].get("description"),
                    start_funds: sqldec!(q[0].get::<&str, Decimal>("start_funds_usdt")),
                    interval: q[0].get("interval"),
                    run_every_sec: q[0].get("run_every_sec"),
                    prepend_data: q[0].get("prepend_data"),
                })
            },
            Err(e) => {
                Err(tradealgorithm::Error::DatabaseError(e.to_string()))
            }
       }
    }

    // Get current funds from this algorithm. We sum the total amount from the history.
    // On success returns a tuple (f64, f64) -> (USDT, BTC)
    pub async fn get_current_funds(&self, psql: Psql) -> Result<(f64, f64), tradealgorithm::Error> {
        let query = psql.lock().await
           .query("
                SELECT
                    start_funds_usdt + COALESCE(SUM(usdt), 0) AS current_funds_usdt,
                    COALESCE(SUM(btc), 0) AS current_funds_btc
                FROM
                    algorithms
                LEFT JOIN
                    history ON history.algorithm_id = algorithms.id
                WHERE
                    algorithms.id = $1
                GROUP BY
                    start_funds_usdt
            ", &[&self.id]).await;

       match query {
            Ok(q) => {
                if q.len() == 0 {
                    return Err(tradealgorithm::Error::AlgorithmError("Algorithm not found".into()));
                }
                
                let current_funds_usdt = sqldec!(q[0].get::<&str, Decimal>("current_funds_usdt"));
                let current_funds_btc = sqldec!(q[0].get::<&str, Decimal>("current_funds_btc"));
                Ok((current_funds_usdt, current_funds_btc))
            },
            Err(e) => {
                Err(tradealgorithm::Error::DatabaseError(e.to_string()))
            }
       }
    }

    // Get current balance. The difference between funds and balance is that the balance keeps
    // in account the volatility of BTC.
    pub async fn get_current_balance(&self, psql: Psql, api: Api) -> Result<f64, tradealgorithm::Error> {
      
        // Get current USDT and BTC funds.
        let (current_funds_usdt, current_funds_btc) = self.get_current_funds(psql.clone()).await?;

        // Get BTC price.
        let btc_price = api.get_btc_price().await?;
        let btc_in_usdt = current_funds_btc * btc_price;

        Ok(current_funds_usdt + btc_in_usdt)
    }
    
    // Start the trading algorithm. The algorithm keeps running on a seperate thread until it is aborted.
    pub async fn start(self, psql: Psql, datastream: Arc<Mutex<mpsc::Receiver<CandleStick>>>, api: Api, ws_send: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>, starttime: u128) -> Result<(std::process::Child, tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>), tradealgorithm::Error> {

        // Variable to save initial prepended kline data. We serialize this later
        // to share it through shared memory.
        let mut data = std::vec::Vec::<CandleStick>::new();

        // Prepend data if required.
        if self.prepend_data > 0 {
            let endtime = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
            
            // Create request parameters.
            let mut params = std::collections::HashMap::<String, String>::new();
            params.insert("symbol".into(), "BTCUSDT".into());
            params.insert("interval".into(), self.interval.to_string());
            params.insert("startTime".into(), starttime.to_string());
            params.insert("endTime".into(), endtime.to_string());

            // Execute request.
            match api.klines(&mut params).await {
                Ok(d) => {
                    for kl in d {
                        data.push(CandleStick {
                            timestamp: serde_json::from_value::<u64>(kl[0].clone())?,
                            open: serde_json::from_value::<String>(kl[1].clone())?.parse::<f64>()?,
                            high: serde_json::from_value::<String>(kl[2].clone())?.parse::<f64>()?,
                            low: serde_json::from_value::<String>(kl[3].clone())?.parse::<f64>()?,
                            close: serde_json::from_value::<String>(kl[4].clone())?.parse::<f64>()?,
                            volume: serde_json::from_value::<String>(kl[5].clone())?.parse::<f64>()?,
                        });
                    }
                },
                Err(_) => {
                    return Err(tradealgorithm::Error::APIError("Could not prepend data".into()));
                }
            };
        }

        // Write prepended data to shared memory.
        let shmem_path = format!("tmp/shmem/{}.bin", self.id);
        std::fs::remove_file(shmem_path).unwrap_or_default();
        let serialized_data = serde_json::to_string(&data).expect("Failed to serialize data.");
        let data_size = serialized_data.len();
        let memfile = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("tmp/shmem/{}.bin", self.id))
            .expect("Failed to open file");
        memfile.set_len(data_size as u64)
            .expect("Failed to set file size");
        let mut mapped_data = unsafe {
            MmapOptions::new()
                .len(data_size)
                .map_mut(&memfile)
                .expect("Failed to map file into memory")
        };
        mapped_data[..data_size].copy_from_slice(serialized_data.as_bytes());


        // Check if PyExecutor binary is legit.
        let pyexecutor_path = "target/debug/python-executor";
        let pyexecutor_bytes = std::fs::read(pyexecutor_path).unwrap();
        let pyexecutor_sha256 = sha256::digest(&pyexecutor_bytes);

        // Check if hash of PyExecutor binary matches predefined SHA256 hash.
        if pyexecutor_sha256 != config::PY_EXECUTOR_HASH.as_str() {
            return Err(tradealgorithm::Error::AlgorithmError("PythonExecutor hash does not match.".into())); 
        }

        // Create a new process to execute algorithm.
        let process_handle = std::process::Command::new(pyexecutor_path)
        .args(&[self.id.to_string(), self.run_every_sec.to_string()])
        .spawn()?;

        // Create a stream so we can write data to the PyExecutor and receive the
        // result back.
        let unix_socket_path = &*format!("tmp/sockets/{}.sock", self.id);

        // Function to connect to UnixSocket and retry 3 times if failure.
        async fn connect_to_unix_socket(unix_socket_path: &str) -> Result<tokio::net::UnixStream, tradealgorithm::Error> {
            for i in 0..3 {
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                match UnixStream::connect(unix_socket_path).await {
                    Ok(uxs) => return Ok(uxs),
                    Err(e) => {
                        if i >= 2 {
                            return Err(tradealgorithm::Error::StreamError(format!("Could not connect to UnixSocket: {}", e)));
                        }
                    }
                }
            }
            
            Err(tradealgorithm::Error::StreamError("Could not connect to UnixSocket after 3 tries.".into()))
        }

        // Connect to UnixSocket and split into receiver and transmitter so we can
        // send data from the API to PyExecutor and receive the result.
        let unix_stream = connect_to_unix_socket(unix_socket_path).await?;
        let (mut rx, mut tx) = unix_stream.into_split();

        // Current BTC price to add to history of algorithms.
        let current_btc_price = api.get_btc_price().await?;
        let current_btc_price = Arc::new(Mutex::new(current_btc_price));

        // Thread to update the current_btc_price every 10 seconds.
        let api_clone = api.clone();
        let current_btc_price_clone = current_btc_price.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(10000)).await;

                let mut current_btc_price_guard = current_btc_price_clone.lock().await;
                *current_btc_price_guard = match api_clone.clone().get_btc_price().await {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                drop(current_btc_price_guard);
            }
        });

        // Thread to receive data from API (websocket) and send it to UnixSocket.
        let thread_recv_websocket_data_handle = tokio::spawn(async move {
            while let Some(n) = datastream.lock().await.recv().await {   
                let ready = tx.ready(Interest::READABLE | Interest::WRITABLE).await.expect("Could not check if UnixSocket is ready to write.");
                if ready.is_writable() {
                    if let Err(e) = tx.write_all(serde_json::json!(n).to_string().as_bytes()).await  {
                        if e.kind() != std::io::ErrorKind::BrokenPipe {
                            eprintln!("\x1b[31m[Error] {}\x1b[0m",e);
                        } else {
                            panic!("Error writing to stream");
                        }
                    }
                }
            }
        });
     
        // Thread to read result from PyExecutor.
        let thread_process_websocket_data = tokio::spawn(async move {
            loop {

                let mut buffer = [0; 1024];
                match rx.read(&mut buffer).await {
                    Ok(0) => {
                        break;
                    },
                    Ok(n) => {

                        let received_result = match String::from_utf8_lossy(&buffer[..n]).parse::<f64>() {
                            Ok(r) => r,
                            Err(_) => {
                                panic!("Could not parse result to f64.");
                            }
                        };


                        // Process result and execute order if necessary.
                        match self.process(psql.clone(), received_result, current_btc_price.clone(), api.clone(), ws_send.clone()).await {
                            Ok(_) => (),
                            Err(e) => {
                                match e {
                                   tradealgorithm::Error::APIError(ee) => {
                                        eprintln!("\x1b[31m[Error] Error in API while processing: {}\x1b[0m", ee);
                                        panic!("Error in API while processing: {}", ee);
                                   },
                                   _ => {
                                        eprintln!("\x1b[31m[Error] {}\x1b[0m", e);
                                   }
                                }
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("\x1b[31m[Error] {}\x1b[0m", e);
                    }
                }
            }
        });

        Ok((process_handle, thread_recv_websocket_data_handle, thread_process_websocket_data))
    }

    // Stop the trading algorithm.
    pub async fn stop(&self) -> Result<(), tradealgorithm::Error> {
        let mut shared_process_handles = PROCESS_HANDLES.lock().await;
        
        // Retrieve process handle and kill if found.
        match shared_process_handles.remove(&self.id) {
            Some(mut ph) => {
                ph.kill()?;
            },
            None => {
                return Err(tradealgorithm::Error::AlgorithmError("Algorithm is not running".into()));
            }
        };

        Ok(())
    }

    // Check if the algorithm is currently running or not.
    pub async fn active(&self) -> bool {
        let shared_process_handles = PROCESS_HANDLES.lock().await;
            
        match shared_process_handles.get(&self.id) {
            Some(_) => true,
            None => false,
        }
    }
    
    // Here we process the result of the Python function. If r > 0 it means we want to buy r
    // amount. If r < 0 it means we want to sell r amount. r == 0 means do nothing.
    async fn process(&self, psql: Psql, r: f64, current_btc_price: Arc<Mutex<f64>>, api: Api, ws_send: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>) -> Result<(), tradealgorithm::Error> {
        if r > 0f64 {
            return self.buy(psql, r, current_btc_price, api, ws_send).await;
        }
        if r < 0f64 {
            return self.sell(psql, r, current_btc_price, api, ws_send).await;
        }
       
        Ok(())
    }

    // Buy.
    async fn buy(&self, psql: Psql, r: f64, current_btc_price: Arc<Mutex<f64>>, api: Api, ws_send: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>) -> Result<(), tradealgorithm::Error> {
      
        // Generate OrderID.
        let order_id : String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();

        // Create order parameters.
        let mut params = std::collections::HashMap::<String, String>::new();
        params.insert("symbol".into(), "BTCUSDT".into());
        params.insert("side".into(), "BUY".into());
        params.insert("type".into(), "MARKET".into());
        params.insert("quantity".into(), r.to_string());
        params.insert("newClientOrderId".into(), order_id.to_string());
      
        let current_btc_price_guard = current_btc_price.lock().await;
        let current_btc_price = *current_btc_price_guard;
        drop(current_btc_price_guard);

        // Order amount in USDT.
        let usdt = r * current_btc_price;

        // Get current USDT funds.
        let (current_funds_usdt, _) = self.get_current_funds(psql.clone()).await?;

        // Check if algorithm has enough USDT assigned.
        if current_funds_usdt - usdt < 0f64 {
            return Err(tradealgorithm::Error::AlgorithmError(format!("{} - Insufficient algorithm funds.", self.id)));
        }

        // Check if account has enough funds.
        if !self.check_funds(api.clone(), "BUY", r, usdt).await? {
            return Err(tradealgorithm::Error::AlgorithmError(format!("{} - Insufficient account funds.", self.id)));
        }

        // Execute order.
        match api.ws_order(&mut params, ws_send).await {
            Ok(_) => {
                println!("\x1b[32m[order] {} - Buying USDT {}\x1b[0m", self.id, usdt);
            },
            Err(e) => {
                return Err(e.into());
            }
        };

        // Register order in database.
        let query = psql.lock().await
           .query("
                INSERT INTO history 
                    (algorithm_id, order_id, action, btc, usdt, btc_price)
                VALUES
                    ($1, $2, 'BUY', $3, $4, $5)
            ", &[&self.id, &order_id, super::sqlf64!(r), super::sqlf64!(usdt * -1f64), super::sqlf64!(current_btc_price)]).await;

        match query {
            Ok(_) => (),
            Err(e) => {
                return Err(tradealgorithm::Error::DatabaseError(format!("{}", e)));
            }
        }

       Ok(())
    }
    
    // Sell.
    async fn sell(&self, psql: Psql, r: f64, current_btc_price: Arc<Mutex<f64>>, api: Api, ws_send: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>) -> Result<(), tradealgorithm::Error> {
        
        // Generate OrderID.
        let order_id : String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();
        
        // Create order parameters.
        let mut params = std::collections::HashMap::<String, String>::new();
        params.insert("symbol".into(), "BTCUSDT".into());
        params.insert("side".into(), "SELL".into());
        params.insert("type".into(), "MARKET".into());
        params.insert("quantity".into(), (r * -1f64).to_string());
        params.insert("newClientOrderId".into(), order_id.to_string());
        
        let current_btc_price_guard = current_btc_price.lock().await;
        let current_btc_price = *current_btc_price_guard;
        drop(current_btc_price_guard);
        
        // Order amount in USDT.
        let usdt = r * current_btc_price;
        
        // Get current BTC funds.
        let (_, current_funds_btc) = self.get_current_funds(psql.clone()).await?;
   
        // Check if algorithm has enough BTC.
        if current_funds_btc - (r * -1f64) < 0f64 {
            return Err(tradealgorithm::Error::AlgorithmError(format!("{} - Insufficient algorithm funds.", self.id)));
        }
        
        // Check if account has enough funds.
        if !self.check_funds(api.clone(), "SELL", r * -1f64, usdt).await? {
            return Err(tradealgorithm::Error::AlgorithmError(format!("{} - Insufficient account funds.", self.id)));
        }
        
        // Execute order.
        match api.ws_order(&mut params, ws_send).await {
            Ok(_) => {
                println!("\x1b[32m[order] {} - Selling USDT {}\x1b[0m", self.id, usdt);
            },
            Err(e) => {
                return Err(e.into());
            }
        };
       
        // Register order in database.
        let query = psql.lock().await
           .query("
                INSERT INTO history 
                    (algorithm_id, order_id, action, btc, usdt, btc_price)
                VALUES
                    ($1, $2, 'SELL', $3, $4, $5)
            ", &[&self.id, &order_id, super::sqlf64!(r), super::sqlf64!(usdt * -1f64), super::sqlf64!(current_btc_price)]).await;

       match query {
           Ok(_) => { return Ok(())  },
           Err(e) => {
               return Err(tradealgorithm::Error::DatabaseError(e.to_string()));
           }
       }
    }

    // Make first BTC order.
    pub async fn first_btc_order(&self, usdt: f64, psql: Psql, api: Api) -> Result<(), tradealgorithm::Error> {
        
        // Get BTC price.
        let btc_price = api.get_btc_price().await?;
        let usdt_in_btc = usdt / btc_price;
        
        // Generate OrderID.
        let order_id : String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();
        
        // Create order parameters.
        let mut params = std::collections::HashMap::<String, String>::new();
        params.insert("symbol".into(), "BTCUSDT".into());
        params.insert("side".into(), "BUY".into());
        params.insert("type".into(), "MARKET".into());
        params.insert("quantity".into(), format!("{:.5}", usdt_in_btc));
        params.insert("newClientOrderId".into(), order_id.to_string());
        
        // Get current USDT funds.
        let (current_funds_usdt, _) = self.get_current_funds(psql.clone()).await?;

        // Check if algorithm has enough USDT assigned.
        if current_funds_usdt - usdt < 0f64 {
            return Err(tradealgorithm::Error::AlgorithmError("Insufficient algorithm funds".into()));
        }

        // Check if account has enough funds.
        if !self.check_funds(api.clone(), "BUY", usdt_in_btc, usdt).await? {
            return Err(tradealgorithm::Error::AlgorithmError("Insufficient account funds".into()));
        }

        // Execute order.
        api.order(&mut params).await?;

        // Register order in database.
        let query = psql.lock().await
           .query("
                INSERT INTO history 
                    (algorithm_id, order_id, action, btc, usdt, btc_price)
                VALUES
                    ($1, $2, 'BUY', $3, $4, $5)
            ", &[&self.id, &order_id, super::sqlf64!(usdt_in_btc), super::sqlf64!(usdt * -1f64), super::sqlf64!(btc_price)]).await;

        match query {
            Ok(_) => Ok(()),
            Err(e) => {
                return Err(tradealgorithm::Error::DatabaseError(e.to_string()));
            }
        }
    }

    // Check if there are enough funds in the account to make the order.
    pub async fn check_funds(&self, api: Api, side: &str, quantity: f64, usdt_price: f64) -> Result<bool, tradealgorithm::Error> {
       let balance = api.account_balance().await?;

        match side {
            "BUY" => {
                if balance.0 < usdt_price {
                    return Ok(false);
                }
            },
            "SELL" => {
                if balance.1 < quantity {
                    return Ok(false);
                }
            },
            _ => { return Ok(false); }
        }

        Ok(true)
    }

    // Get chart of this algorithm.
    pub async fn get_chart(&self, psql: Psql) -> Result<std::vec::Vec<serde_json::Value>, tradealgorithm::Error> {
        
        // Create data object we will return.
        #[derive(Serialize)]
        struct Data {
            timestamp: String,
            total: String,
            usdt: String,
            btc: String,
        }
            let query = psql.lock().await
           .query("
                WITH btc_price_cte AS (
                    SELECT created_at, btc_price FROM history where algorithm_id = $1 ORDER BY created_at
                )
                SELECT
                 start_funds_usdt + COALESCE(h.total_usdt, 0) + COALESCE(h.total_btc * btc_price_cte.btc_price, 0) AS current_funds_total,
                 start_funds_usdt + COALESCE(h.total_usdt, 0) AS current_funds_usdt,
                COALESCE(h.total_btc, 0) AS current_funds_btc,
                h.created_at::TEXT AS ts
                FROM
                    algorithms
                LEFT JOIN
                    history_aggregate h ON h.algorithm_id = algorithms.id
                LEFT JOIN
                    btc_price_cte ON btc_price_cte.created_at = h.created_at
                WHERE
                    algorithms.id = $1
                GROUP BY
                    algorithm_id, start_funds_usdt, h.total_usdt, h.total_btc, btc_price_cte.btc_price, h.created_at
                ORDER BY h.created_at;
            ", &[&self.id]).await;

       match query {
            Ok(q) => {
                if q.len() == 0 {
                    return Err(tradealgorithm::Error::AlgorithmError("Algorithm not found".into()));
                }
        
                let mut data = std::vec::Vec::<serde_json::Value>::new();
              
                for row in q {
                    let current_funds_total = row.get::<&str, Decimal>("current_funds_total");
                    let current_funds_usdt = row.get::<&str, Decimal>("current_funds_usdt");
                    let current_funds_btc = row.get::<&str, Decimal>("current_funds_btc");
                    let timestamp = row.get::<&str, String>("ts");

                    data.push(serde_json::json!(Data {
                        timestamp: timestamp.to_string(),
                        total: format!("{:.5}", current_funds_total),
                        usdt: format!("{:.5}", current_funds_usdt),
                        btc: format!("{:.5}", current_funds_btc),
                    }));
                }

                return Ok(data);
            },
            Err(e) => {
                return Err(tradealgorithm::Error::DatabaseError(e.to_string()));
            }
       }
        
    }
    
    // Get history of this algorithm.
    pub async fn get_history(&self, psql: Psql, start_at_timestamp: Option<String>) -> Result<serde_json::value::Value, tradealgorithm::Error> {

        let start_at = match start_at_timestamp {
            Some(ts) => ts,
            None => "CURRENT_TIMESTAMP".into(),
        };
       
        let query = psql.lock().await
           .query("
                SELECT
                    order_id, action, btc, usdt, btc_price, created_at::TEXT
                FROM
                    history
                WHERE
                    algorithm_id = $1
                AND
                    created_at::TEXT < $2::TEXT
                AND
                    action IS NOT NULL
                ORDER BY
                    created_at DESC
                LIMIT 25
            ", &[&self.id, &start_at]).await;

        let rows = match query {
            Ok(q) => q,
            Err(e) => {
                return Err(tradealgorithm::Error::DatabaseError(e.to_string()));
            }
        };
        
        // Create data object we will return.
        #[derive(Serialize)]
        struct Data {
            order_id: String,
            action: String,
            btc: f64,
            usdt: f64,
            btc_price: f64,
            created_at: String,
        }

        let mut data = std::vec::Vec::<Data>::new();
        for row in rows {
            let d = Data {
                order_id: row.get("order_id"),
                action: row.get("action"),
                btc: sqldec!(row.get::<_, Decimal>("btc")),
                usdt: sqldec!(row.get::<_, Decimal>("usdt")),
                btc_price: sqldec!(row.get::<_, Decimal>("btc_price")),
                created_at: row.get("created_at"),
            };

            data.push(d);
        }

        Ok(serde_json::json!(data))
    }

    pub async fn insert_btc_price(api: Api, psql: Psql) -> Result<(), tradealgorithm::Error> {
        // Get BTC price.
        let btc_price = api.get_btc_price().await?;

        let sql = format!("
            DO
            $$
            declare f record;
            begin
                for f in SELECT DISTINCT id from algorithms
                    loop
                        insert into history(algorithm_id, btc, usdt, btc_price) values (f.id, 0.0, 0.00, {});
                    end loop;
            end
            $$;
        ", btc_price);

        let query = psql.lock().await
           .query(&*sql, &[]).await;

        match query {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(tradealgorithm::Error::DatabaseError(e.to_string()))
            }
        }
    }


    pub async fn reset(&self, psql: Psql, api: Api) -> Result<(), tradealgorithm::Error> {

        // Check if algorithm is still running.
        if self.active().await {
            return Err(tradealgorithm::Error::AlgorithmError("Algorithm is still running".into()));
        }

        // Get current total balance in USDT.
        let current_balance = self.get_current_balance(psql.clone(), api).await?;

        // Delete order history.
        let query = psql.lock().await
           .query("
                DELETE FROM history
                WHERE
                    algorithm_id = $1
            ", &[&self.id]).await;

        match query {
            Ok(_) => {
                // Reset start_funds_usdt to current_funds.
                let query = psql.lock().await
                   .query("
                        UPDATE
                            algorithms
                        SET
                            start_funds_usdt = $1
                        WHERE
                            id = $2
                    ", &[super::sqlf64!(current_balance), &self.id]).await;

                match query {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        return Err(tradealgorithm::Error::DatabaseError(e.to_string()));
                    }
                }
            },
            Err(e) => {
                return Err(tradealgorithm::Error::DatabaseError(e.to_string()));
            }
        }
    }
    
    pub fn get_code(&self) -> Result<String, tradealgorithm::Error> {
        let python_code = match std::fs::read_to_string(format!("trading_algos/{}.py", self.id)) {
            Ok(code) => code,
            Err(e) => {
                return Err(tradealgorithm::Error::AlgorithmError(format!("Couldn't read Python file {}", e)));
            }
        };

        Ok(python_code)
    }
}


// Error type for TradeAlgorithm.
#[derive(Debug)]
pub enum Error {
    APIError(String),
    AlgorithmError(String),
    ParseError(String),
    DatabaseError(String),
    PythonCodeError(String),
    StreamError(String),
}

impl std::error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::ParseError(e.to_string())
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::APIError(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::AlgorithmError(e.to_string())
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::ParseError(format!("line: {} {}", line!(), e.to_string()))
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(e: std::num::ParseFloatError) -> Self {
        Error::ParseError(format!("line: {} {}", line!(), e.to_string()))
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(e: tokio_postgres::Error) -> Self {
        Error::DatabaseError(e.to_string())
    }
}

impl From<api::Error> for Error {
    fn from(e: api::Error) -> Self {
        Error::APIError(e.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::APIError(error_msg) => write!(f, "\x1b[31m[Error] TradeAlgorithm - APIError: {}\x1b[0m", error_msg),
            Error::ParseError(error_msg) => write!(f, "\x1b[31m[Error] TradeAlgorithm - ParseError: {}\x1b[0m", error_msg),
            Error::DatabaseError(error_msg) => write!(f, "\x1b[31m[Error] TradeAlgorithm - DatabaseError: {}\x1b[0m", error_msg),
            Error::PythonCodeError(error_msg) => write!(f, "\x1b[31m[Error] TradeAlgorithm - PythonCodeError: {}\x1b[0m", error_msg),
            Error::AlgorithmError(error_msg) => write!(f, "\x1b[31m[Error] TradeAlgorithm - AlgorithmError: {}\x1b[0m", error_msg),
            Error::StreamError(error_msg) => write!(f, "\x1b[31m[Error] TradeAlgorithm - StreamError: {}\x1b[0m", error_msg),
        }
    }
}

