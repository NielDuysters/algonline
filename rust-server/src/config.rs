/*
 * Edit this file to configure the application.
 */

use super::*;

// Key of this application's API which is required by third parties.
pub static API_KEY: LazyOnceCell<String> = LazyOnceCell::new(|| "abc123".to_string());

// Database credentials.
pub static DB_HOST: LazyOnceCell<String> = LazyOnceCell::new(|| "localhost".to_string());
pub static DB_USER: LazyOnceCell<String> = LazyOnceCell::new(|| "postgres".to_string());
pub static DB_PASS: LazyOnceCell<String> = LazyOnceCell::new(|| "password".to_string());
pub static DB_NAME: LazyOnceCell<String> = LazyOnceCell::new(|| "algonline_db".to_string());

// Exchange API config
pub static REST_API_URL: LazyOnceCell<String> = LazyOnceCell::new(|| "https://testnet.binance.vision/api/v3".to_string());
pub static WEBSOCKET_API_URL: LazyOnceCell<String> = LazyOnceCell::new(|| "wss://testnet.binance.vision/ws-api/v3".to_string());
pub static WEBSOCKET_STREAM_URL: LazyOnceCell<String> = LazyOnceCell::new(|| "wss://stream.binance.com:443/ws".to_string());

// Hash of PyExecutor binary.
pub static PY_EXECUTOR_HASH: LazyOnceCell<String> = LazyOnceCell::new(|| "a5f60233e08d7669f8f1765142ca21d8dfa7ac30a94dc5046ecb7ea0a271bc52".to_string());
