use super::*;
use sha2::Sha256;
use hmac::{Hmac, Mac};

use serde_json::json;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct Binance {
    rest_api_url: String,
    ws_api_url: String,
    ws_stream_url: String,
    api_key: String,
    api_secret: String,
}

#[async_trait]
impl ExchangeAPI for Binance {
    
    // Construct Binance object.
    // :param rest_api_url: &str representing the base URL of the REST API.
    // :param ws_api_url: &str representing the base URL of the WebSocket API (Action stream).
    // :param ws_stream_url: &str representing the base URL of the WebSocket Stream (Info stream).
    // :param api_key: &str of the API KEY you can find in your account.
    // :param api_secret: &str of the API SECRET matching the API key.
    fn new(rest_api_url: &str, ws_api_url: &str, ws_stream_url: &str, api_key: &str, api_secret: &str) -> Self {
        Binance {
            rest_api_url: rest_api_url.into(),
            ws_api_url: ws_api_url.into(),
            ws_stream_url: ws_stream_url.into(),
            api_key: api_key.into(),
            api_secret: api_secret.into(),
        }
    }

    // Set API Keys to this instance of the API by retrieving user from database by session token.
    async fn auth(&mut self, session_token: &str, psql: Psql) -> Result<(), api::Error> {
        
        // Retrieve keys with session token.
        let query = psql.lock().await
            .query("
                SELECT
                    api_key, api_secret
                FROM
                    users
                WHERE
                    session_token = $1
             ", &[&session_token]).await;

        let (api_key, api_secret) : (Option<String>, Option<String>) = match query {
            Ok(q) => {
                if q.len() == 0 {
                    return Err(api::Error::AuthenticationError("No user found with session token.".into()));
                }

                (q[0].get("api_key"), q[0].get("api_secret"))
            },
            Err(e) => {
                return Err(api::Error::DatabaseError(format!("{}", e)));
            }
        };

        match (api_key, api_secret) {
            (Some(key), Some(secret)) => {
                self.api_key = key;
                self.api_secret = secret;
            },
            _ => {
                return Err(api::Error::DatabaseError("API Keys are NULL.".into()));
            }
        }
        
        Ok(())
    }
    
    // Return API and Stream URLs from this API.
    fn get_urls(&self, url_type: &str) -> Option<&str> {
        match url_type {
            "rest_api_url" => Some(&*self.rest_api_url),
            "ws_api_url" => Some(&*self.ws_api_url),
            "ws_stream_url" => Some(&*self.ws_stream_url),
            _ => None,
        }
    }

    // Ping the exchange API-server to check connectivity.
    async fn ping(&self) -> bool {
        // Initiate client to make request.
        let http_client = reqwest::Client::new();

        // Send a simple GET-request.
        let request = 
            http_client.get(format!("{url}/ping", url = self.rest_api_url)).send().await;

        // Return true if the request was succesful and with HTTP status 200.
        // Return false in all other cases.
        match request {
            Err(_) => false,
            Ok(req) => {
                match req.status() {
                    reqwest::StatusCode::OK => { return true; },
                    _ => { return false; },
                }
            }
        }
    }

    // Make a order (buy or sell).
    // :param params:   A hashmap holding the key-values necessary for the order.
    //                  Read: https://binance-docs.github.io/apidocs/futures/en/#new-order-trade
    async fn order(&self, params: &mut std::collections::HashMap<String, String>) -> Result<String, api::Error> {

        // Initiate client to make request.
        let http_client = reqwest::Client::new();

        // Generate timestamp as string required to make the order-request.
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string(); 
        params.insert("timestamp".to_string(), timestamp);

        // Convert params to string-payload for the request.
        // {"key": "value"} -> key=value
        let mut payload = params
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<std::vec::Vec<String>>()
            .join("&");
        
        // Signature using key-hashed message authentication code based on SHA256.
        let mut hmac = HmacSha256::new_from_slice(self.api_secret.as_bytes())?;
        hmac.update(payload.as_bytes());
        let signature = hmac.finalize().into_bytes();
        
        // Append the signature to the payload.
        payload = format!("{}&signature={}", payload, hex::encode(signature));

        // Send request.
        let request = 
            http_client.post(format!("{url}/order", url = self.rest_api_url))
            .header("X-MBX-APIKEY", self.api_key.to_string())
            .body(payload.to_string());

            let request = request.send()
            .await;

        let request = match request {
            Ok(req) => req,
            Err(e) => {
                return Err(api::Error::ExchangeAPIError(format!("{}", e)));
            }       
        };

        match request.status().is_success() {
            true => {
                Ok(request.text().await?)
            },
            false => {
                Err(api::Error::ExchangeAPIError(format!("{}", request.text().await?)))
            }
        }
    }


    // Get account balance. Returns a tuple -> (USDT, BTC).
    async fn account_balance(&self) -> Result<(f64, f64), api::Error> {

        // Initiate client to make request.
        let http_client = reqwest::Client::new();

        let mut params = std::collections::HashMap::<String, String>::new();

        // Generate timestamp as string required to make the order-request.
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string(); 
        params.insert("timestamp".to_string(), timestamp);

        // Convert params to string-payload for the request.
        // {"key": "value"} -> key=value
        let mut payload = params
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<std::vec::Vec<String>>()
            .join("&");
        
        // Signature using key-hashed message authentication code based on SHA256.
        let mut hmac = HmacSha256::new_from_slice(self.api_secret.as_bytes())?;
        hmac.update(payload.as_bytes());
        let signature = hmac.finalize().into_bytes();
        
        // Append the signature to the payload.
        payload = format!("{}&signature={}", payload, hex::encode(signature));

        // Send request.
        let request = 
            http_client.get(format!("{url}/account?{payload}", url = self.rest_api_url, payload = payload))
            .header("X-MBX-APIKEY", self.api_key.to_string())
            .send()
            .await;


        let request = match request {
            Ok(req) => req,
            Err(e) => {
                return Err(api::Error::ExchangeAPIError(format!("Request could not be made: {}", e)));
            },
        };

        let response = request.text().await?;
        let json = serde_json::from_str::<serde_json::Value>(&*response)?;

        let mut balances = (None, None);
        if let Some(arr) = json["balances"].as_array() {
            for a in arr {
                if a["asset"].as_str().unwrap() == "USDT" {
                    balances.0 = Some(json_str_to_f64!(a["free"]));
                }
                if a["asset"].as_str().unwrap() == "BTC" {
                    balances.1 = Some(json_str_to_f64!(a["free"]));
                }

                if balances.0.is_some() && balances.1.is_some() {
                    break;
                }
            }
        } 

        match (balances.0, balances.1) {
            (Some(usdt), Some(btc)) => Ok((usdt, btc)),
            _ => {
                return Err(api::Error::ExchangeAPIError("Balances were not retrieved.".to_string()));
            }
        }
    }
    
    
    // Get trade history.
    async fn trade_history(&self) -> Result<(), api::Error> {
        // Initiate client to make request.
        let http_client = reqwest::Client::new();

        let mut params = std::collections::HashMap::<String, String>::new();

        // Generate timestamp as string required to make the order-request.
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string(); 
        params.insert("symbol".to_string(), "BTCUSDT".into());
        params.insert("limit".to_string(), "1".into());
        params.insert("timestamp".to_string(), timestamp);

        // Convert params to string-payload for the request.
        // {"key": "value"} -> key=value
        let mut payload = params
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<std::vec::Vec<String>>()
            .join("&");
        
        // Signature using key-hashed message authentication code based on SHA256.
        let mut hmac = HmacSha256::new_from_slice(self.api_secret.as_bytes())?;
        hmac.update(payload.as_bytes());
        let signature = hmac.finalize().into_bytes();
        
        // Append the signature to the payload.
        payload = format!("{}&signature={}", payload, hex::encode(signature));

        // Send request.
        let request = 
            http_client.get(format!("{url}/myTrades?{payload}", url = self.rest_api_url, payload = payload))
            .header("X-MBX-APIKEY", self.api_key.to_string())
            .send()
            .await;


        match request {
            Ok(_) => Ok(()),
            Err(e) => {
                return Err(api::Error::ExchangeAPIError(format!("Error making request {}", e)));
            },
        }
    }

    async fn ws_order(&self, params: &mut std::collections::HashMap<String, String>, ws_send: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>) -> Result<(), api::Error> {
       
        // Get timestamp.
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string(); 

        // Set parameters payload for order request.
        params.insert("apiKey".into(), self.api_key.to_string());
        params.insert("timestamp".into(), timestamp.to_string());

        // Generate signature.
        let signature = self.generate_signature(params)?;

        // JSON string order-message.
        let json = 
        format!("
        {{\n
          \"id\": \"{}\",\n
          \"method\": \"order.place\",\n
          \"params\": {}\n
          }}
        ",  timestamp, serde_json::to_string(&params)?); 
    

        // JSON string to JSON object.
        let mut json = serde_json::from_str::<serde_json::Value>(&*json)?;
        json["params"] = json!(params);
        json["params"]["signature"] = json!(signature);

        // Send to websocket.
        match ws_send.lock().await.send(Message::Text(json.to_string())).await {
            Ok(_) => Ok(()),
            Err(e) => {
                return Err(api::Error::ExchangeAPIError(format!("Error in websocket {}", e)));
            }
        }
    }
    
    // Get klines using Rest-API.
    // :param params:   A hashmap holding the key-values necessary for requesting the klines.
    //                  Read: https://binance-docs.github.io/apidocs/spot/en/#kline-candlestick-data
    async fn klines(&self, params: &mut std::collections::HashMap<String, String>) -> Result<std::vec::Vec<serde_json::Value>, api::Error> {

        // Initiate client to make request.
        let http_client = reqwest::Client::new();

        // Convert params to string-payload for the request.
        // {"key": "value"} -> key=value
        let payload = params
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<std::vec::Vec<String>>()
            .join("&");

        // Send request.
        let request = 
            http_client.get(format!("{url}/klines?{payload}", url = self.rest_api_url, payload = payload))
            .header("X-MBX-APIKEY", self.api_key.to_string());

        let request = request.send()
            .await;

        let request = match request {
            Ok(req) => req,
            Err(e) =>  {
                return Err(api::Error::ExchangeAPIError(format!("{}", e)));
            }
        };

        let text = match request.text().await {
            Ok(t) => t,
            Err(e) => {
                return Err(api::Error::ExchangeAPIError(format!("{}", e)));
            }
        };


        let data: std::vec::Vec<serde_json::Value> = match serde_json::from_str(&text) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("lol {}", e);
                return Err(api::Error::ExchangeAPIError(format!("{}", e)));
            }
        };

        Ok(data)
    }

    // Use Binance websocket API to retrieve a stream of klines of BTCUSDT with a specified
    // interval.
    // The klines are send to Sender 's'.
    async fn ws_kline(&self, s: Arc<Mutex<mpsc::Sender<CandleStick>>>, interval: String) -> Result<(), api::Error> {
        // Automatically try to reconnect after 5 seconds if connection drops.
        loop {
            let url = url::Url::parse(&*format!("{url}/btcusdt@kline_{interval}", url = &*self.ws_stream_url, interval = interval))?;
            
            let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
            let (_, mut read) = ws_stream.split();
            
            while let Some(message) = read.next().await {
                let data = message.unwrap().into_data();
                let data = &*String::from_utf8(data).unwrap();
                let json = serde_json::from_str::<serde_json::Value>(data).unwrap();

                let candlestick = || -> Result<CandleStick, &str> {
                    Ok(CandleStick {
                        timestamp: json["k"]["T"].as_number().ok_or("Error")?.as_u64().ok_or("Error")?,
                        open: json_str_to_f64!(json["k"]["o"]),
                        close: json_str_to_f64!(json["k"]["c"]),
                        low: json_str_to_f64!(json["k"]["l"]),
                        high: json_str_to_f64!(json["k"]["h"]),
                        volume: json_str_to_f64!(json["k"]["v"]),
                    })
                };

                match candlestick() {
                    Ok(c) => {
                        if let Err(e) = s.lock().await.send(c).await {
                            return Err(api::Error::ExchangeAPIError(format!("{}", e)));
                        }
                    },
                    Err(_) => continue,
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
        }
    }

    async fn get_btc_price(&self) -> Result<f64, api::Error> {
        // Initiate client to make request.
        let http_client = reqwest::Client::new();
        
        // Send request.
        let request = http_client
            .get(format!("{url}/klines?symbol=BTCUSDT&interval=1s&limit=1", url = self.rest_api_url))
            .send()
            .await;


        let request = match request {
            Ok(req) => req,
            Err(e) => return Err(api::Error::ExchangeAPIError(format!("{}", e))),
        };
        
        // Get response and make JSON.
        let response = request.text().await?;
        let json : std::vec::Vec<serde_json::Value> = match serde_json::from_str(&*response) {
            Ok(js) => js,
            Err(e) => {
                return Err(api::Error::ExchangeAPIError(format!("{}", e)));
            }
        };

        if json.is_empty() {
            return Err(api::Error::ExchangeAPIError("No klines retrieved to get btc price".into()));
        }

        Ok(json_str_to_f64!(json[0][4]))
    }
    fn keys(&self) -> String {
        self.api_key.to_string()   
    }
}

impl Binance {
    // Generate HMAC-SHA256 signature of paramerers to validate API-call.
    fn generate_signature(&self, params: &mut std::collections::HashMap<String, String>) -> Result<String, api::Error> {
        // Sort the parameters alphabetically by key
        let mut sorted_params: Vec<(&String, &String)> = params.iter().collect();
        sorted_params.sort_by(|a, b| a.0.cmp(b.0));

        // Generate query string to create signature.
        let query_string = sorted_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
            .join("&");

        // Generate signature.
        let mut hmac = HmacSha256::new_from_slice(self.api_secret.as_bytes())?;
        hmac.update(query_string.as_bytes());
        
        // Return signature.
        Ok(hex::encode(hmac.finalize().into_bytes()))
    }
}
