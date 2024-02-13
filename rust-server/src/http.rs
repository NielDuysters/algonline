use super::*;


// Process the incoming stream.
pub async fn process(mut stream: &mut TcpStream, psql: Psql) -> Result<(), http::Error> {
   
    // Function to write the response to the stream.
    async fn return_response(response: &mut HttpResponse, stream: &mut TcpStream) -> Result<(), http::Error> {
        // Make CORS rules.
        let cors = Cors::rules(vec![
            ("Access-Control-Allow-Origin".into(), "*".into()),
            ("Access-Control-Allow-Headers".into(), "*".into()),
            ("Access-Control-Allow-Methods".into(), "GET, POST, PUT, DELETE".into()),
        ]);
    
        // Apply CORS to response.
        cors.apply(response);
        
        stream.write_all(response.to_string().as_bytes()).await?;
        
        // Consume stream.
        let mut buffer = [0; 4096];
        stream.read(&mut buffer).await?;
        
        Ok(())
    }

    // Buffer holding the stream in bytes.
    let mut buffer = [0; 4096];
    if let Err(e) = stream.peek(&mut buffer).await {
        let mut response = Routes::internal_server_error().await;
        return_response(&mut response, &mut stream).await?;
        return Err(http::Error::StreamError(format!("{}", e)));
    }
   
    // Create a string from the contents of the buffer.
    let stream_string = String::from_utf8_lossy(&buffer[..]);

    // Turn the raw stream as string into a workable Http-object.
    let mut http = Http::from_str(&stream_string);

    // Check API_KEY
    if http.method != "OPTIONS" {
        match http.headers.get("API_KEY") {
            Some(api_key) => {
                if api_key != config::API_KEY.as_str() {
                    return_response(&mut Routes::unauthorized().await, &mut stream).await?;
                    return Ok(());
                }
            },
            None => {
                return_response(&mut Routes::unauthorized().await, &mut stream).await?;
                return Ok(());
            }
        }
    }

    // Create API instance for this request.
    let mut api = create_api_instance();
    if let Some(session_token) = http.headers.get("session_token") {
        api.auth(session_token, psql.clone()).await.unwrap_or_default();
    }
    let api = Arc::new(api);

    // Check if there is a route matching the HTTP-method and path.
    let mut response = match RouteHandler::goto(&mut http, psql, api).await {
        Ok(r) => r,
        Err(_) => {
            Routes::internal_server_error().await
        }
    };
    
    return_response(&mut response, &mut stream).await?;
    Ok(())
}

// Struct for CORS.
pub struct Cors {
    headers: std::vec::Vec<(String, String)>,
}

impl Cors {
    pub fn rules(headers: std::vec::Vec<(String, String)>) -> Self {
        Cors {
            headers: headers,
        }
    }

    pub fn apply(self, response: &mut HttpResponse) {
        response.headers.extend(self.headers);
    }
}

// Struct for incoming HTTP requests.
#[derive(Clone, Debug)]
pub struct Http {
    pub method: String,
    pub path: String,
    pub params: std::collections::HashMap<String, String>,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
}

impl Http {
    // Generate Http-object from raw stream string.
    // Note: This function assumes that all incoming requests are well formed in the expected
    // format.
    fn from_str(str: &str) -> Self {
        let lines : std::vec::Vec<&str> = str.lines().collect();
        let request_line : std::vec::Vec<&str> = lines[0].split_whitespace().collect();
        
        let method = request_line[0];
        let path = request_line[1];
     
        let mut headers : std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for line in &lines[1..] {
            if line.is_empty() {
                break;
            }

            let parts : std::vec::Vec<&str> = line.splitn(2, ":").collect();
            if parts.len() == 2 {
                headers.insert(
                    parts[0].trim().into(),
                    parts[1].trim().into()
                );
            }
        }

        let mut body = "".to_string();
        let mut sep_found = false;
        for line in lines {
            if line == "" {
                sep_found = true;
                continue;
            }

            if sep_found {
                body = format!("{}\n{}", body, line);
            }
        }

        // Remove trailing zero-bytes.
        body = body.chars().filter(|&c| c != '\0').collect();

        Http {
            method: method.into(),
            path: path.into(),
            headers: headers,
            // Params will be set in Route::goto where we find the matching route.
            params: std::collections::HashMap::new(),
            body: body,
        }
    }
}

// Struct for a HTTP Response.
// This way we can easily define a response and use the string formatter to write
// the response to the stream as string.
pub struct HttpResponse {
    pub status: u16,
    pub headers: std::vec::Vec<(String, String)>,
    pub body: String,
}

// Convert HttpResponse to string.
impl std::fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut headers : String = "".into();
        for (key, value) in self.headers.iter() {
            headers = format!("{}{}: {}\n", headers, key, value);
        }

        write!(
            f, 
            "HTTP/1.1 {status}\n{headers}\n\n{body}",
            status = self.status,
            headers = headers,
            body = self.body
        )
    }
}

// Function to check if requested resource belongs to authenticated user.
#[derive(PartialEq)]
pub enum DBTable<'a> {
    Algorithm(&'a str),
}

pub async fn validate_session_token(session_token: &str, table: DBTable<'_>, psql: Psql) -> Result<bool, http::Error> {
    // Retrieve user with session token.
    let query = psql.lock().await
       .query("
            SELECT
                id
            FROM 
                users
            WHERE
                session_token = $1
        ", &[&session_token]).await;

    let user_id : i32 = match query {
        Ok(q) => {
            if q.len() == 0 {
                return Err(http::Error::DatabaseError("No user found for session token".into()));
            }

            q[0].get("id")
        },
        Err(e) => {
           return Err(http::Error::DatabaseError(format!("{}", e)));
        }
    };

    // Check if user_id is foreign key of DBTable.
    let (table_name, id) = match table {
        DBTable::Algorithm(id) => ("algorithms", id),
    };
    
    let sql = format!("SELECT * FROM {table} WHERE id = $1 AND user_id = $2", table = table_name);
    let query = psql.lock().await.query(&*sql, &[&id, &user_id]).await;

    match query {
        Ok(q) => {
            return Ok(q.len() > 0);
        },
        Err(e) => {
           return Err(http::Error::DatabaseError(format!("Error: {}", e)));
        }
    }
}


// Error type for the HTTPServer.
#[derive(Debug)]
pub enum Error {
    HttpServerError(String),
    ParseError(String),
    DatabaseError(String),
    RequestError(String),
    WebsocketError(String),
    StreamError(String),
    TradeAlgorithmError(String),
}

impl std::error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::ParseError(e.to_string())
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::RequestError(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::StreamError(e.to_string())
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Self {
        Error::ParseError(e.to_string())
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::ParseError(e.to_string())
    }
}

impl From<chrono::ParseError> for Error {
    fn from(e: chrono::ParseError) -> Self {
        Error::ParseError(e.to_string())
    }
}

impl From<regex::Error> for Error {
    fn from(e: regex::Error) -> Self {
        Error::ParseError(e.to_string())
    }
}

impl From<base64::DecodeError> for Error {
    fn from(e: base64::DecodeError) -> Self {
        Error::ParseError(e.to_string())
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Error::ParseError(e.to_string())
    }
}

impl From<tradealgorithm::Error> for Error {
    fn from(e: tradealgorithm::Error) -> Self {
        Error::TradeAlgorithmError(e.to_string())
    }
}

impl From<api::Error> for Error {
    fn from(e: api::Error) -> Self {
        Error::RequestError(e.to_string())
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(e: tokio_postgres::Error) -> Self {
        Error::DatabaseError(e.to_string())
    }
}

impl From<deadpool::managed::PoolError<tokio_postgres::Error>> for Error {
    fn from(e: deadpool::managed::PoolError<tokio_postgres::Error>) -> Self {
        Error::DatabaseError(e.to_string())
    }
}


impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::HttpServerError(error_msg) => write!(f, "\x1b[31m[Error] HttpServer - HttpServerError: {}\x1b[0m", error_msg),
            Error::ParseError(error_msg) => write!(f, "\x1b[31m[Error] HttpServer - ParseError: {}\x1b[0m", error_msg),
            Error::DatabaseError(error_msg) => write!(f, "\x1b[31m[Error] HttpServer - DatabaseError: {}\x1b[0m", error_msg),
            Error::RequestError(error_msg) => write!(f, "\x1b[31m[Error] HttpServer - RequestError: {}\x1b[0m", error_msg),
            Error::WebsocketError(error_msg) => write!(f, "\x1b[31m[Error] HttpServer - WebsocketError: {}\x1b[0m", error_msg),
            Error::StreamError(error_msg) => write!(f, "\x1b[31m[Error] HttpServer - StreamError: {}\x1b[0m", error_msg),
            Error::TradeAlgorithmError(error_msg) => write!(f, "\x1b[31m[Error] HttpServer - TradeAlgorithmError: {}\x1b[0m", error_msg),
        }
    }
}
