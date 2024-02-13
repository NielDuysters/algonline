use super::*;

type WsSender = Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>;

// Incoming requests require a action and session_token + API_KEY for authentication.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WebsocketRequest {
    pub action: String,
    pub session_token: String,
    pub api_key: String,
    pub params: Option<std::collections::HashMap<String, String>>,
}

pub async fn handle_websocket(ws_stream: TcpStream, psql: Psql) -> Result<(), websocket::Error> {

    // Upgrade to websocket stream.
    let ws_stream = tokio_tungstenite::accept_async(ws_stream).await
    .expect("Error during the websocket handshake occurred");

    // Make sender and receiver.
    let (ws_sender, mut ws_receiver) = ws_stream.split();
    let ws_sender = Arc::new(Mutex::new(ws_sender));

    while let Some(Ok(message)) = ws_receiver.next().await {
        match message {
            Message::Text(data) => {
                // Handle incoming message and execute asked function.
                let request : WebsocketRequest = serde_json::from_str(&*data)?;

                // Create API instance for this request.
                let mut api = create_api_instance();
                if let Some(params) = request.clone().params {
                    if let Some(session_token) = params.get("session_token") {
                        api.auth(session_token, psql.clone()).await?;
                    }
                }

                let api = Arc::new(api);


                if let Err(e) = process_request(request.clone(), ws_sender.clone(), psql.clone(), api.clone()).await {
                    eprintln!("{}", e);
                    break;
                }
            },
            Message::Close(_) => {
                // Close connection.
                if let Err(e) = ws_sender.lock().await.send(Message::Close(None)).await {
                    eprintln!("Error sending close message: {}", e);
                }
                return Ok(());
            }
            _ => {
                // Handle other message types
                println!("Received unknown message type");
            }
        }
    }

    Ok(())
}

pub async fn process_request(request: WebsocketRequest, ws_sender: WsSender, psql: Psql, api: Api) -> Result<(), websocket::Error> {
    fn validate_api_key(api_key: &str) -> bool {
        api_key == config::API_KEY.as_str()
    }

    if !validate_api_key(&*request.api_key) {
        return Err(websocket::Error::WebsocketError("Invalid API-key given.".into()));
    }
    
    match &*request.action {
        "btc-candlestick" => btc_candlestick(ws_sender, request.params, api).await,
        "algorithm-stats" => algorithm_stats(ws_sender, request.params, psql).await,
        _ => Err(websocket::Error::WebsocketError("Unknown action".into())),
    }

}

pub async fn btc_candlestick(ws_sender: WsSender, params: Option<std::collections::HashMap<String, String>>, api: Api) -> Result<(), websocket::Error> {

    // Get parameters for this function.
    let interval = match params {
        Some(p) => {
            if let Some(val) = p.get(&"interval".to_string()) {
                val.to_string()
            } else {
                return Err(websocket::Error::WebsocketError("Required parameter not given.".into()));
            }
        },
        None => {
            return Err(websocket::Error::WebsocketError("Required parameter not given.".into()));
        }
    };

    // Make a transmitter and receiver. The transmitter is passed to the Exchange API so the
    // receiver can send the incoming responsed back to the websocket cliet.
    let (tx, rx) = mpsc::channel::<tradealgorithm::CandleStick>(10);
    let tx = Arc::new(Mutex::new(tx));
    let rx = Arc::new(Mutex::new(rx));
   
    // Start websocket API of exchange and pass the transmitter as parameter
    // so the incoming data is send to the algorithm.
    tokio::spawn(async move {
        if let Err(e) = api.ws_kline(tx, interval).await {
            eprintln!("{}", e);
        }
    }); 
           
    // Let the receiver retrieve data from API and send it back to
    // our websocket client.
    tokio::spawn(async move {
        while let Some(n) = rx.lock().await.recv().await {
            let json = match serde_json::to_string(&n) {
                Ok(j) => j,
                Err(_) => {
                    continue;
                }
            };

            let mut sender = ws_sender.lock().await;
            if let Err(e) = sender.send(Message::Text(json)).await {
                eprintln!("{}", e);
                break;
            }
        }
    });

    Ok(())
}

pub async fn algorithm_stats(ws_sender: WsSender, params: Option<std::collections::HashMap<String, String>>, psql: Psql) -> Result<(), websocket::Error> {

    // Get parameters for this function.
    let algorithm_id = match params {
        Some(p) => {
            if let Some(val) = p.get(&"id".to_string()) {
                val.to_string()
            } else {
                return Err(websocket::Error::WebsocketError("Required parameter not given.".into()));
            }
        },
        None => {
            return Err(websocket::Error::WebsocketError("Required parameter not given.".into()));
        }
    };
       

    // PostgreSQL connection.
    let (client, mut connection) = tokio_postgres::connect(&*format!("host={} user={} password={} dbname={}",
        config::DB_HOST.as_str(),
        config::DB_USER.as_str(), 
        config::DB_PASS.as_str(),
        config::DB_NAME.as_str()
    ), NoTls).await.unwrap();

    // Make transmitter and receiver.
    let (tx, rx) = futures_channel::mpsc::unbounded();
    let stream = stream::poll_fn(move |cx| connection.poll_message(cx)).map_err(|e| panic!("{}", e));
    let listener_connection = stream.forward(tx).map(|r| r.unwrap());
    tokio::spawn(listener_connection);
  
    // Execute listen.
    match client
        .query(
            "LISTEN history_record_inserted", &[]
        )
        .await
        {
            Ok(_) => (),
            Err(e) => {
                eprintln!("{}", e);
            }
        }

    // Data for response. We return or a history_row or a new datapoint for the chart.
    #[derive(Serialize, Deserialize)]
    enum ResponseType {
        HistoryRow,
        ChartDataPoint,
    }

    #[derive(Serialize)]
    struct Data {
        response_type: ResponseType,
        json: serde_json::Value,
    }

    // Wait for notifications and send back to websocket client.
    rx.filter_map(|m| {
        futures_util::future::ready(
            if let tokio_postgres::AsyncMessage::Notification(n) = m {
                Some(n)
            } else {
                None
            }
        )
    })
    .for_each(|n| {
        let ws_sender_clone = ws_sender.clone();
        let psql_clone = psql.clone();
        let algorithm_id = algorithm_id.to_string();
        async move {
            let payload = &n.payload();
            let parsed_json: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(payload);

            match parsed_json {
                Ok(json) => {
                    
                    if json["algorithm_id"].as_str().unwrap() != algorithm_id.to_string() {
                        return;
                    }

                    // Retrieve algorithm.
                    let algorithm = TradeAlgorithm::get(algorithm_id.to_string(), psql_clone.clone()).await.unwrap();

                    let (current_funds_usdt, current_funds_btc) = match algorithm.get_current_funds(psql_clone.clone()).await {
                        Ok((usdt, btc)) => (usdt, btc),
                        Err(_) => {
                            return;
                        }
                    };
                    
                    let btc_price: f64 = if let Some(p) = json["btc_price"].as_f64() { p } else { return; };

                    let btc_in_usdt = current_funds_btc * btc_price;
                    let total_funds = current_funds_usdt + btc_in_usdt;
                    
                    let chart_datapoint = serde_json::json!(Data {
                        response_type: ResponseType::ChartDataPoint,
                        json: serde_json::json!({
                            "timestamp": json["created_at"].as_str(),
                            "total": format!("{:.5}", total_funds),
                            "usdt": format!("{:.5}", current_funds_usdt),
                            "btc": format!("{:.5}", current_funds_btc),
                        }),
                    });
                    
                    let mut sender = ws_sender_clone.lock().await;
                    if let Err(e) = sender.send(Message::Text(chart_datapoint.to_string())).await {
                        eprintln!("\x1b[31m[Error] Websocket error: {}\x1b[0m", e);
                        if let Err(e) = sender.close().await {
                            match e {
                                tokio_tungstenite::tungstenite::error::Error::Io(_) => {
                                    eprintln!("\x1b[31m[Error] Exiting websocket: {}\x1b[0m", e);
                                    panic!();
                                },
                                _ => (),
                           }
                        }

                        return;
                    }
        
                    if json["action"].is_null() {
                        return;
                    }

                    let history_row = serde_json::json!(Data {
                        response_type: ResponseType::HistoryRow,
                        json: json,
                    });

                    if let Err(e) = sender.send(Message::Text(history_row.to_string())).await {
                        eprintln!("\x1b[31m[Error] Websocket error: {}\x1b[0m", e);
                        if let Err(e) = sender.close().await {
                            match e {
                                tokio_tungstenite::tungstenite::error::Error::Io(_) => {
                                    eprintln!("\x1b[31m[Error] Exiting websocket: {}\x1b[0m", e);
                                    panic!();
                                },
                                _ => (),
                           }
                        }

                        return;
                    }
                },
                Err(e) => {
                    eprintln!("{}", e);
                }
            }

        }
    })
    .await;

    Ok(())
}

// Error type for the Websocket.
#[derive(Debug)]
pub enum Error {
    ParseError(String),
    WebsocketError(String),
    StreamError(String),
    ApiError(String),
}

impl std::error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::ParseError(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::StreamError(e.to_string())
    }
}

impl From<api::Error> for Error {
    fn from(e: api::Error) -> Self {
        Error::ApiError(e.to_string())
    }
}


impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::WebsocketError(error_msg) => write!(f, "\x1b[31m[Error] Websocket - WebsocketError: {}\x1b[0m", error_msg),
            Error::ParseError(error_msg) => write!(f, "\x1b[31m[Error] Websocket - ParseError: {}\x1b[0m", error_msg),
            Error::StreamError(error_msg) => write!(f, "\x1b[31m[Error] Websocket - StreamError: {}\x1b[0m", error_msg),          
            Error::ApiError(error_msg) => write!(f, "\x1b[31m[Error] Websocket - ApiError: {}\x1b[0m", error_msg),          
        }
    }
}
