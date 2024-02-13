use super::*;

pub mod binance;

// Thanks to this trait we can easily adapt between different implementations for different
// exchanges.
#[async_trait]
pub trait ExchangeAPI : Send + Sync {
    fn new(rest_api_url: &str, ws_api_url: &str, ws_stream_url: &str, api_key: &str, api_secret: &str) -> Self where Self: Sized;
    async fn auth(&mut self, session_token: &str, psql: Psql) -> Result<(), Error>;
    fn get_urls(&self, url_type: &str) -> Option<&str>;
    async fn ping(&self) -> bool;
    async fn order(&self, params: &mut std::collections::HashMap<String, String>) -> Result<String, Error>;
    async fn trade_history(&self) -> Result<(), Error>;
    async fn ws_order(&self, params: &mut std::collections::HashMap<String, String>, ws_send: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>) -> Result<(), Error>;
    async fn account_balance(&self) -> Result<(f64, f64), Error>;
    async fn klines(&self, params: &mut std::collections::HashMap<String, String>) -> Result<std::vec::Vec<serde_json::Value>, Error>;
    async fn ws_kline(&self, s :Arc<Mutex<mpsc::Sender<CandleStick>>>, interval: String) -> Result<(), Error>;
    async fn get_btc_price(&self) -> Result<f64, Error>;
    fn keys(&self) -> String;
}

// Error type for ExchangeAPI.
#[derive(Debug)]
pub enum Error {
    ExchangeAPIError(String),
    RequestError(String),
    ParseError(String),
    DatabaseError(String),
    AuthenticationError(String),
}

impl std::error::Error for Error {}
impl From<sha1::digest::InvalidLength> for Error {
    fn from(e: sha1::digest::InvalidLength) -> Self {
        Error::ParseError(e.to_string())
    }
}

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

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::ParseError(e.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::ExchangeAPIError(error_msg) => write!(f, "\x1b[31m[Error] ExchangeAPI - ExchangeAPIError: {}\x1b[0m", error_msg),
            Error::ParseError(error_msg) => write!(f, "\x1b[31m[Error] ExchangeAPI - ParseError: {}\x1b[0m", error_msg),
            Error::DatabaseError(error_msg) => write!(f, "\x1b[31m[Error] ExchangeAPI - DatabaseError: {}\x1b[0m", error_msg),
            Error::RequestError(error_msg) => write!(f, "\x1b[31m[Error] ExchangeAPI - RequestError: {}\x1b[0m", error_msg),
            Error::AuthenticationError(error_msg) => write!(f, "\x1b[31m[Error] ExchangeAPI - AuthenticationError: {}\x1b[0m", error_msg),
        }
    }
}

