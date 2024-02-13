use super::*;
use async_recursion::async_recursion;

// Struct Routes holds the following:
// - A method `goto` to find and execute a the fn pointer to a route for the matching Http-method
// and path.
// - Defined custom routes.
// - Predefined standard routes (e.g 404, 500)
pub struct Routes;
impl Routes {

    // Custom routes.
    //

    // Ping.
    pub async fn ping(&self, _req: http::Http, _psql: Psql, _api: Api) -> Result<http::HttpResponse, http::Error> {
        Ok(http::HttpResponse {
            status: 200,
            headers: vec![
                ("Content-Type".into(), "text/plain".into()),
            ],
            body: "Server is online.".into(),
        })
    }
    
    // Ping exchange.
    pub async fn ping_exchange(&self, _req: http::Http, _psql: Psql, api: Api) -> Result<http::HttpResponse, http::Error> {
        
        match api.ping().await {
            true => {
                Ok(http::HttpResponse {
                    status: 200,
                    headers: vec![
                        ("Content-Type".into(), "text/plain".into()),
                    ],
                    body: "Exchange is online.".into(),
                })
            },
            false => {
                Ok(http::HttpResponse {
                    status: 503,
                    headers: vec![
                        ("Content-Type".into(), "text/plain".into()),
                    ],
                    body: "Exchange is offline.".into(),
                })
            }
        }
    }

    // Start a algorithm.
    pub async fn start_algorithm(&self, req: http::Http, psql: Psql, api: Api) -> Result<http::HttpResponse, http::Error> {

        // Retrieve algorithm ID from URL-paremeter.
        let algo_id = match req.params.get("id").cloned() {
            Some(id) => id,
            None => {
                return Ok(Routes::not_found().await);
            }
        };

        // Check if algorithm belongs to user doing request.
        match req.headers.get("session_token") {
            Some(token) => {
                match http::validate_session_token(token, http::DBTable::Algorithm(&*algo_id), psql.clone()).await {
                    Ok(v) => {
                        if !v {
                            return Ok(Routes::unauthorized().await);
                        }
                    },
                    Err(_) => {
                        return Ok(Routes::internal_server_error().await);
                    }
                }
            },
            None => {
                return Ok(Routes::unauthorized().await);
            }
        }
       
        // Retrieve algorithm and get starttime of first run.
        let algorithm = TradeAlgorithm::get(algo_id.to_string(), psql.clone()).await.unwrap();
        let algorithm = Box::new(algorithm);
        let current_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
        let starttime = current_time - algorithm.prepend_data as u128;
       
        // Recursive function: If we receive an error in one of the threads we run
        // the function again. This way the websocket connection is remade.
        #[async_recursion]
        async fn start_algorithm_recursive(algorithm: Box<TradeAlgorithm>, starttime: u128, psql: Psql, api: Api) -> Result<(), String> {
            // Make a transmitter and receiver. The receiver is passed to the algorithm when started
            // so it can receive the data from the transmitter which we pass to the exchange API.
            let (tx, rx) = mpsc::channel::<CandleStick>(10);
            let tx = Arc::new(Mutex::new(tx));
            let rx = Arc::new(Mutex::new(rx));
           
            // Make websocket stream to API endpoint for making orders.
            let url = url::Url::parse(api.get_urls("ws_api_url").unwrap()).unwrap();
            let (ws_stream, _) = connect_async(url.clone()).await.expect("Failed to connect");
            let (send, _) = ws_stream.split();
            
            let send = Arc::new(Mutex::new(send));

            // Retrieve algorithm by id and start.
            let interval = algorithm.interval.to_string();
            let (process_handle, thread_a_handle, thread_b_handle) = match algorithm.clone().start(psql.clone(), rx, api.clone(), send, starttime).await {
                Ok((ph, th_a, th_b)) => (ph, th_a, th_b),
                Err(e) => {
                    return Err(format!("Error starting algortithm: {}", e));
                }
            };

            // Start websocket API of exchange and pass the transmitter as parameter
            // so the incoming data is send to the algorithm.
            let api_clone = api.clone();
            tokio::spawn(async move {
                if let Err(e) = api_clone.ws_kline(tx, interval).await {
                    eprintln!("{}", e);
                }
            }); 


            // Add process handle to process-handle list so we can stop algorithm later.
            let mut shared_process_handles = tradealgorithm::PROCESS_HANDLES.lock().await;
            shared_process_handles.insert(algorithm.id.to_string(), process_handle);

            tokio::spawn(async move {
                if let Err(_) = tokio::try_join!(thread_a_handle, thread_b_handle) {
                    if let Err(e) = algorithm.stop().await {
                        match e  {
                            tradealgorithm::Error::AlgorithmError(_) => {
                                eprintln!("\x1b[31m[Error] {} - Exiting.\x1b[0m", algorithm.id);
                                panic!();
                            },
                            _ => (),
                        }
                    }
                    
                    eprintln!("\x1b[31m[Error] {} - A thread panicked. Attempting to restart websocket...\x1b[0m]", algorithm.id);
                    start_algorithm_recursive(algorithm, starttime, psql.clone(), api.clone()).await.unwrap();
                    tokio::time::sleep(tokio::time::Duration::from_millis(10000)).await;
                }
            });

            Ok(())
        }

        tokio::spawn(async move {
            start_algorithm_recursive(algorithm, starttime, psql, api).await.expect("Error starting algorithm.");
        });
        
        // Forge and return response.
        let response = http::HttpResponse {
            status: 200,
            headers: vec![
                ("Content-Type".into(), "text/plain".into()),
            ],
            body: "Algorithm started".into(),
        };

        Ok(response)
    }
    
    // Stop a algorithm.
    pub async fn stop_algorithm(&self, req: http::Http, psql: Psql, _api: Api) -> Result<http::HttpResponse, http::Error> {
        // Retrieve algorithm ID from URL-paremeter.
        let algo_id = match req.params.get("id").cloned() {
            Some(id) => id,
            None => {
                return Ok(Routes::not_found().await);
            }
        };
        
        // Check if algorithm belongs to user doing request.
        match req.headers.get("session_token") {
            Some(token) => {
                match http::validate_session_token(token, http::DBTable::Algorithm(&*algo_id), psql.clone()).await {
                    Ok(v) => {
                        if !v {
                            return Ok(Routes::unauthorized().await);
                        }
                    },
                    Err(_) => {
                        return Ok(Routes::internal_server_error().await);
                    }
                }
            },
            None => {
                return Ok(Routes::unauthorized().await);
            }
        }
        
        // Retrieve algorithm.
        let algorithm = TradeAlgorithm::get(algo_id.to_string(), psql.clone()).await.unwrap();
        
        let response = match algorithm.stop().await {
            Ok(_) => {
                http::HttpResponse {
                    status: 200,
                    headers: vec![
                        ("Content-Type".into(), "text/plain".into()),
                    ],
                    body: "Algorithm stopped.".into(),
                }
            },
            Err(_) => {
                http::HttpResponse {
                    status: 200,
                    headers: vec![
                        ("Content-Type".into(), "text/plain".into()),
                    ],
                    body: "Algorithm could not be stopped.".into(),
                }
            }
        };
 
        Ok(response)
    }

    // Get properties of an algorithm like id, description, start_funds,...
    pub async fn get_algorithm(&self, req: http::Http, psql: Psql, api: Api) -> Result<http::HttpResponse, http::Error> {


        // Retrieve algorithm ID from URL-paremeter.
        let algo_id = match req.params.get("id").cloned() {
            Some(id) => id,
            None => {
                return Ok(Routes::not_found().await);
            }
        };
        
        // Check if algorithm belongs to user doing request.
        match req.headers.get("session_token") {
            Some(token) => {
                match http::validate_session_token(token, http::DBTable::Algorithm(&*algo_id), psql.clone()).await {
                    Ok(v) => {
                        if !v {
                            return Ok(Routes::unauthorized().await);
                        }
                    },
                    Err(_) => {
                        return Ok(Routes::internal_server_error().await);
                    }
                }
            },
            None => {
                return Ok(Routes::unauthorized().await);
            }
        }
       
        // Retrieve algorithm
        let algorithm = match TradeAlgorithm::get(algo_id.clone(), psql.clone()).await {
            Ok(algo) => algo,
            Err(_) => {
                return Ok(Routes::not_found().await);
            }
        };
        
        // Create data object we will return.
        #[derive(Serialize)]
        struct Data<'a> {
            id: &'a str,
            description: &'a str,
            start_funds: f64,
            current_funds: f64,
            is_running: bool,
        }

        let data = Data {
            id: &algorithm.id,
            description: &algorithm.description,
            start_funds: algorithm.start_funds,
            current_funds: algorithm.get_current_balance(psql, api).await?,
            is_running: algorithm.active().await,
        };
        
        Ok(http::HttpResponse{
            status: 200,
            headers: vec![
                ("Content-Type".into(), "application/json".into()),
            ],
            body: serde_json::to_string(&data).unwrap(),
        })
    }
    
    // Add algorithm to database.
    pub async fn add_algorithm(&self, req: http::Http, psql: Psql, api: Api) -> Result<http::HttpResponse, http::Error> {
      
        // Retrieve session token if set.
        let session_token = match req.headers.get("session_token") {
            Some(token) => token,
            None => {
                return Ok(Routes::unauthorized().await);
            }
        };

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

       let user_id = match query {
            Ok(q) => {
                if q.len() == 0 {
                    return Err(http::Error::DatabaseError("No user found with session token.".into()));
                }

                q[0].get("id")
            },
            Err(e) => {
                return Err(http::Error::DatabaseError(format!("{}", e)));
            }
       };

        // Define the expected data from the POST-request/
        #[derive(Serialize, Deserialize)]
        struct Data<'a> {
            id: &'a str,
            description: &'a str,
            start_funds: f64,
            first_btc_order: f64,
            interval: &'a str,
            run_every_sec: i32,
            prepend_data: &'a str,
            code: &'a str,
        }

        // Make Data object from POST request body. Return error 400
        // if sent data is malformed.
        let data : Data = match serde_json::from_str(&*req.body) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{}", e);

                return Ok(http::HttpResponse {
                    status: 400,
                    headers: vec![
                        ("Content-Type".into(), "text/plain".into()),
                    ],
                    body: e.to_string(),
                });
            }
        };

        // Convert prepend_data to ms.
        let prepend_data = match data.prepend_data {
            "30m" => 60000 * 30,
            "1h" => 60000 * 60,
            "1d" => 60000 * 60 * 24,
            "1w" => 60000 * 60 * 24 * 7,
            "30d" => 60000 * 60 * 24 * 30,
            _ => 0,
        };

        // Create algorithm.
        let id = data.id.to_string().replace(" ", "_").replace("-", "_").to_lowercase();
        let algorithm = 
            match TradeAlgorithm::new(
                id.to_string(), 
                data.description.into(), 
                data.start_funds, 
                data.interval.into(), 
                data.run_every_sec,
                prepend_data,
                user_id,
                psql.clone()
            ).await {
            Ok(algo) => algo,
            Err(e) => {
                eprintln!("{}", e);
                return Ok(Routes::internal_server_error().await);
            }
        };

        // Save code.
        let file_path = format!("trading_algos/{}.py", id);
        let mut file = match std::fs::File::create(file_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{}", e);
                algorithm.delete(psql).await?;
                return Err(http::Error::TradeAlgorithmError(format!("Error creating file: {}", e)));
            }
        };

        let mut code = std::vec::Vec::new();
        let b64_engine = base64::engine::GeneralPurpose::new(&base64::alphabet::STANDARD, base64::engine::general_purpose::PAD);
        let mut b64_decoder = base64::read::DecoderReader::new(data.code.as_bytes(), &b64_engine);

        if let Err(e) = b64_decoder.read_to_end(&mut code) {
            eprintln!("{}", e);
            algorithm.delete(psql).await?;
            return Err(http::Error::ParseError(format!("Error parsing base64 {}", e)));
        }

        if let Err(e) = file.write_all(code.as_slice()) {
            eprintln!("{}", e);
            algorithm.delete(psql).await?;
            return Err(http::Error::TradeAlgorithmError(format!("Error writing to file: {}", e)));
        }

        // Execute first BTC order.
        if data.first_btc_order > 0f64 {
            if let Err(e) = algorithm.first_btc_order(data.first_btc_order, psql.clone(), api).await {
                eprintln!("{}", e);
                algorithm.delete(psql).await?;
                return Err(http::Error::TradeAlgorithmError(format!("Error making first BTC order: {}", e)));
            }
        }

        // Return response.
        Ok(http::HttpResponse{
            status: 201,
            headers: vec![
                ("Content-Type".into(), "text/plain".into()),
            ],
            body: "".into(),
        })
    }
    
    // Delete algorithm from database.
    pub async fn delete_algorithm(&self, req: http::Http, psql: Psql, _api: Api) -> Result<http::HttpResponse, http::Error> {
        
        // Retrieve algorithm ID from URL-paremeter.
        let algo_id = match req.params.get("id").cloned() {
            Some(id) => id,
            None => {
                return Ok(Routes::not_found().await);
            }
        };
        
        // Check if algorithm belongs to user doing request.
        match req.headers.get("session_token") {
            Some(token) => {
                match http::validate_session_token(token, http::DBTable::Algorithm(&*algo_id), psql.clone()).await {
                    Ok(v) => {
                        if !v {
                            return Ok(Routes::unauthorized().await);
                        }
                    },
                    Err(_) => {
                        return Ok(Routes::internal_server_error().await);
                    }
                }
            },
            None => {
                return Ok(Routes::unauthorized().await);
            }
        }
        
        // Retrieve algorithm.
        let algorithm = TradeAlgorithm::get(algo_id, psql.clone()).await?;

        // Check if algorithm is running. 
        if algorithm.active().await {
            return Ok(http::HttpResponse{
                status: 409,
                headers: vec![
                    ("Content-Type".into(), "text/plain".into()),
                ],
                body: "Algorithm is currently running. Stop it before deleting".into(),
            });
        }

        // Delete algorithm.
        algorithm.delete(psql).await?;

        // Return response.
        Ok(http::HttpResponse{
            status: 204,
            headers: vec![],
            body: "".into(),
        })
    }
    
    // Reset algorithm. Set current total balance as new start_funds_usdt and delete all order
    // history.
    pub async fn reset_algorithm(&self, req: http::Http, psql: Psql, api: Api) -> Result<http::HttpResponse, http::Error> {
        
        // Retrieve algorithm ID from URL-paremeter.
        let algo_id = match req.params.get("id").cloned() {
            Some(id) => id,
            None => {
                return Ok(Routes::not_found().await);
            }
        };
        
        // Check if algorithm belongs to user doing request.
        match req.headers.get("session_token") {
            Some(token) => {
                match http::validate_session_token(token, http::DBTable::Algorithm(&*algo_id), psql.clone()).await {
                    Ok(v) => {
                        if !v {
                            return Ok(Routes::unauthorized().await);
                        }
                    },
                    Err(_) => {
                        return Ok(Routes::internal_server_error().await);
                    }
                }
            },
            None => {
                return Ok(Routes::unauthorized().await);
            }
        }
        
        // Retrieve algorithm.
        let algorithm = TradeAlgorithm::get(algo_id, psql.clone()).await?;

        // Check if algorithm is running. 
        if algorithm.active().await {
            return Ok(http::HttpResponse{
                status: 409,
                headers: vec![
                    ("Content-Type".into(), "text/plain".into()),
                ],
                body: "Algorithm is currently running. Stop it before deleting".into(),
            });
        }

        // Reset algorithm and return response.
        match algorithm.reset(psql, api).await {
            Ok(_) => {
                Ok(http::HttpResponse{
                    status: 204,
                    headers: vec![],
                    body: "".into(),
                })
            },
            Err(e) => {
                eprintln!("Reset error {}", e);
                Ok(Routes::internal_server_error().await)
            }
        }
    }
    
    // Get USDT, BTC and total account balance.
    pub async fn balance(&self, _req: http::Http, _psql: Psql, api: Api) -> Result<http::HttpResponse, http::Error> {
        
        // Get current USDT and BTC balance.
        let (usdt, btc) = match api.account_balance().await {
            Ok((a, b)) => (a, b),
            Err(e) => {
                return Err(http::Error::RequestError(format!("Couldn't retrieve balances {}", e)));
            }
        };

        // Calculate total balance.
        let btc_price = api.get_btc_price().await?;
        let btc_in_usdt = btc * btc_price;
        let total = usdt + btc_in_usdt;

        Ok(http::HttpResponse{
            status: 200,
            headers: vec![
                ("Content-Type".into(), "application/json".into()),
            ],
            body: serde_json::to_string(&(usdt, btc, total))?,
        })
    }
    
    // Initialize a user:
    // When a new user registers or an existing user resets his account we sell all the available
    // BTC so the user has a start balance in USDT. We compare the performance of the algorithms
    // over time by comparing the start balance with the current balance.
    // This function also removes all algorithms of this user.
    pub async fn init_user(&self, req: http::Http, psql: Psql, api: Api) -> Result<http::HttpResponse, http::Error> {

        // Retrieve session token if set.
        let session_token = match req.headers.get("session_token") {
            Some(token) => token,
            None => {
                return Ok(Routes::unauthorized().await);
            }
        };

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
                    return Err(http::Error::DatabaseError("No user found with session token.".into()));
                }

                q[0].get("id")
            },
            Err(e) => {
                return Err(http::Error::DatabaseError(format!("{}", e)));
            }
        };
        
        // Remove all algorithms from this user.
        let query = psql.lock().await
            .query("
                SELECT
                    id
                FROM
                    algorithms
                WHERE
                    user_id = $1
             ", &[&user_id]).await;
        match query {
            Ok(q) => {
                for row in q {
                    // Retrieve algorithm.
                    let algorithm = TradeAlgorithm::get(row.get("id"), psql.clone()).await?;
                   
                    // Stop algorithm if it's still running.
                    if algorithm.active().await {
                        algorithm.clone().stop().await?;
                    }

                    // Delete algorithm.
                    algorithm.delete(psql.clone()).await?;
                }
            },
            Err(e) => {
                return Err(http::Error::DatabaseError(format!("{}", e)));
            }
        }
       
        // Get current USDT and BTC balance.
        let (usdt, btc) = api.account_balance().await?;

        // Calculate total start funds.
        let btc_price = api.get_btc_price().await?;
        let btc_in_usdt = btc * btc_price;
        let total = usdt + btc_in_usdt;

        // Set start_funds of user.
        let query = psql.lock().await
            .query("
                UPDATE
                    users
                SET
                    start_funds_usdt = $1,
                    start_funds_btc = $2,
                    start_funds_total = $3
                WHERE
                    id = $4
             ", &[sqlf64!(usdt), sqlf64!(btc), sqlf64!(total), &user_id]).await;

        match query {
            Ok(_) => (),
            Err(e) => {
                return Err(http::Error::DatabaseError(format!("{}", e)));
            }
        }


        Ok(http::HttpResponse{
            status: 200,
            headers: vec![
                ("Content-Type".into(), "text/plain".into()),
            ],
            body: "User initialized.".into(),
        })
    }


    pub async fn trade_history(&self, _req: http::Http, _psql: Psql, api: Api) -> Result<http::HttpResponse, http::Error> {
    
        api.trade_history().await?;
        Ok(http::HttpResponse{
            status: 200,
            headers: vec![
                ("Content-Type".into(), "text/plain".into()),
            ],
            body: "User initialized.".into(),
        })
    }
    
    pub async fn get_btc_price(&self, _req: http::Http, _psql: Psql, api: Api) -> Result<http::HttpResponse, http::Error> {
        api.get_btc_price().await?;
        Ok(http::HttpResponse{
            status: 200,
            headers: vec![
                ("Content-Type".into(), "text/plain".into()),
            ],
            body: "User initialized.".into(),
        })
    }

    pub async fn get_algorithm_chart(&self, req: http::Http, psql: Psql, _api: Api) -> Result<http::HttpResponse, http::Error> {

        fn filter_data_by_duration(data: std::vec::Vec<serde_json::Value>, duration: chrono::Duration) -> Result<Vec<serde_json::Value>, http::Error> {

            let mut filtered_data = Vec::new();

            if let Some(first_obj) = data.first() {
                if let Some(start_time_str) = first_obj["timestamp"].as_str() {
                    let mut start_time = chrono::NaiveDateTime::parse_from_str(start_time_str, "%Y-%m-%d %H:%M:%S%.f")?;
                    filtered_data.push(first_obj.clone());

                    for json_object in &data[1..] {
                        if let Some(timestamp_str) = json_object["timestamp"].as_str() {
                            let timestamp = chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S%.f")?;

                            if timestamp >= start_time + duration {
                                start_time = timestamp;
                                filtered_data.push(json_object.clone());
                            }
                        }
                    }
                }
            }

            Ok(filtered_data)
        }

        // Retrieve algorithm ID from URL-paremeter.
        let algo_id = match req.params.get("id").cloned() {
            Some(id) => id,
            None => {
                return Ok(Routes::not_found().await);
            }
        };

        // Retrieve chart interval.
        let chart_interval : tradealgorithm::ChartInterval = match req.params.get("interval").cloned() {
            Some(ci) => std::str::FromStr::from_str(&*ci).unwrap(),
            None => {
                return Ok(Routes::not_found().await);
            }
        };

        // Check if algorithm belongs to user doing request.
        match req.headers.get("session_token") {
            Some(token) => {
                match http::validate_session_token(token, http::DBTable::Algorithm(&*algo_id), psql.clone()).await {
                    Ok(v) => {
                        if !v {
                            return Ok(Routes::unauthorized().await);
                        }
                    },
                    Err(e) => {
                        eprintln!("err {}", e);
                        return Ok(Routes::internal_server_error().await);
                    }
                }
            },
            None => {
                return Ok(Routes::unauthorized().await);
            }
        }
                    
        // Retrieve algorithm.
        let algorithm = TradeAlgorithm::get(algo_id, psql.clone()).await?;

        // Retrieve algorithm chart.
        let data = algorithm.get_chart(psql).await?;
        let data = match chart_interval {
            tradealgorithm::ChartInterval::Hourly => filter_data_by_duration(data, chrono::Duration::hours(1))?,
            tradealgorithm::ChartInterval::Daily => filter_data_by_duration(data, chrono::Duration::days(1))?,
            _ => data,
        };

        Ok(http::HttpResponse{
            status: 200,
            headers: vec![
                ("Content-Type".into(), "application/json".into()),
            ],
            body: serde_json::json!(data).to_string(),
        })

    }
    
    pub async fn get_algorithm_history(&self, req: http::Http, psql: Psql, _api: Api) -> Result<http::HttpResponse, http::Error> {

        // Retrieve all parameters from url.
        let algo_id = match req.params.get("id").cloned() {
            Some(id) => id,
            None => {
                return Ok(Routes::not_found().await);
            }
        };
       
       
        let start_at_timestamp = match req.params.get("start_at").cloned() {
            Some(ts) => {
                match urlencoding::decode(&ts) {
                    Ok(s) => {
                        if s.to_string() == "undefined".to_string() {
                            None
                        } else {
                            Some(s.to_string())
                        }
                    },
                    Err(_) => None,
                }
            },
            None => None,
        };

        // Check if algorithm belongs to user doing request.
        match req.headers.get("session_token") {
            Some(token) => {
                match http::validate_session_token(token, http::DBTable::Algorithm(&*algo_id), psql.clone()).await {
                    Ok(v) => {
                        if !v {
                            return Ok(Routes::unauthorized().await);
                        }
                    },
                    Err(_) => {
                        return Ok(Routes::internal_server_error().await);
                    }
                }
            },
            None => {
                return Ok(Routes::unauthorized().await);
            }
        }
                    
        // Retrieve algorithm.
        let algorithm = TradeAlgorithm::get(algo_id, psql.clone()).await?;

        // Retrieve algorithm history.
        let data = algorithm.get_history(psql, start_at_timestamp).await?;

        Ok(http::HttpResponse{
            status: 200,
            headers: vec![
                ("Content-Type".into(), "application/json".into()),
            ],
            body: data.to_string(),
        })

    }
    
    // Get code of algorithm.
    pub async fn get_algorithm_code(&self, req: http::Http, psql: Psql, _api: Api) -> Result<http::HttpResponse, http::Error> {
        
        // Retrieve algorithm ID from URL-paremeter.
        let algo_id = match req.params.get("id").cloned() {
            Some(id) => id,
            None => {
                return Ok(Routes::not_found().await);
            }
        };

        // Check if algorithm belongs to user doing request.
        match req.headers.get("session_token") {
            Some(token) => {
                match http::validate_session_token(token, http::DBTable::Algorithm(&*algo_id), psql.clone()).await {
                    Ok(v) => {
                        if !v {
                            return Ok(Routes::unauthorized().await);
                        }
                    },
                    Err(_) => {
                        return Ok(Routes::internal_server_error().await);
                    }
                }
            },
            None => {
                return Ok(Routes::unauthorized().await);
            }
        }
                    
        // Retrieve algorithm.
        let algorithm = TradeAlgorithm::get(algo_id, psql.clone()).await?;

        // Retrieve algorithm code and encode in Base64.
        let algorithm_code = algorithm.get_code()?;
        let mut code = std::vec::Vec::new();
        let b64_engine = base64::engine::GeneralPurpose::new(&base64::alphabet::STANDARD, base64::engine::general_purpose::PAD);
        let mut b64_encoder = base64::write::EncoderWriter::new(&mut code, &b64_engine);


        if let Err(e) = b64_encoder.write_all(algorithm_code.as_bytes()) {
            eprintln!("{}", e);
            algorithm.delete(psql).await?;
            return Err(http::Error::ParseError(format!("Error parsing base64 {}", e)));
        }

        drop(b64_encoder);

        Ok(http::HttpResponse{
            status: 200,
            headers: vec![
                ("Content-Type".into(), "text/plain".into()),
            ],
            body: String::from_utf8(code)?,
        })
    }
    
    pub async fn get_klines(&self, req: http::Http, _psql: Psql, api: Api) -> Result<http::HttpResponse, http::Error> {

        // Retrieve all required parameters.
        let interval = match req.params.get("interval").cloned() {
            Some(int) => int,
            None => "1s".into(),
        };
        
        let amount = match req.params.get("amount").cloned() {
            Some(amt) => amt.parse::<u32>()?,
            None => 15,
        };

        // Convert selected interval to matching amount in ms.
        let ms = match &*interval {
            "1s" => 0,
            "1m" => 60000,
            "5m" => 60000 * 5,
            "1h" => 60000 * 60,
            "1d" => 60000 * 60 * 24,
            _ => {
                return Ok(Routes::internal_server_error().await);
            }
        };

        // Set current time as endTime so we get all the data until now.
        let endtime = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();

        // Set startTime to the current time minus the amount of ms of the interval * the amount
        // of requested datapoints.
        let starttime = endtime - (amount as u128 * ms);
        
        // Create request parameters.
        let mut params = std::collections::HashMap::<String, String>::new();
        params.insert("symbol".into(), "BTCUSDT".into());
        params.insert("interval".into(), interval.into());
        params.insert("limit".into(), amount.to_string());

        if ms > 0 {
            params.insert("startTime".into(), starttime.to_string());
            params.insert("endTime".into(), endtime.to_string());
        }

        // Execute request.
        let data = match api.klines(&mut params).await {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error {}", e);
                return Ok(Routes::internal_server_error().await);
            }
        };

        Ok(http::HttpResponse{
            status: 200,
            headers: vec![
                ("Content-Type".into(), "application/json".into()),
            ],
            body: serde_json::json!(data).to_string(),
        })

    }
    
    // Make order
    pub async fn order(&self, req: http::Http, _psql: Psql, api: Api) -> Result<http::HttpResponse, http::Error> {
        
        #[derive(Serialize, Deserialize)]
        struct Data<'a> {
            action: &'a str,
            amount: f64,
        }
        
        // Make Data object from POST request body. Return error 400
        // if sent data is malformed.
        let data : Data = match serde_json::from_str(&*req.body) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error 4 {}", e);

                return Ok(http::HttpResponse {
                    status: 400,
                    headers: vec![
                        ("Content-Type".into(), "text/plain".into()),
                    ],
                    body: "".to_string(),
                });
            }
        };

        // Generate OrderID.
        let order_id : String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();
        
        // Convert amount in USDT to BTC.
        let btc_price = api.get_btc_price().await?;
        let usdt_in_btc = data.amount / btc_price;

        // Create order parameters.
        let mut params = std::collections::HashMap::<String, String>::new();
        params.insert("symbol".into(), "BTCUSDT".into());
        params.insert("side".into(), data.action.to_string().to_uppercase());
        params.insert("type".into(), "MARKET".into());
        params.insert("quantity".into(), format!("{:.5}", usdt_in_btc));
        params.insert("newClientOrderId".into(), order_id.to_string());
        
        // Execute order.
        match api.order(&mut params).await {
            Ok(_) => {
                Ok(http::HttpResponse {
                    status: 200,
                    headers: vec![
                        ("Content-Type".into(), "text/plain".into()),
                    ],
                    body: "Order made.".into(),
                })
            },
            Err(e) => {
                eprintln!("error {}", e);
                
                Ok(http::HttpResponse {
                    status: 400,
                    headers: vec![
                        ("Content-Type".into(), "text/plain".into()),
                    ],
                    body: e.to_string(),
                })
            }
        }
    }

    // Standard routes.
    //
    pub async fn not_found() -> http::HttpResponse {
        http::HttpResponse {
            status: 404,
            headers: vec![
                ("Content-Type".into(), "text/plain".into()),
            ],
            body: "Not found.".into(),
        }
    }

    pub async fn internal_server_error() -> http::HttpResponse {
        http::HttpResponse {
            status: 500,
            headers: vec![
                ("Content-Type".into(), "text/plain".into()),
            ],
            body: "Internal server error.".into(),
        }
    }
    
    pub async fn forbidden() -> http::HttpResponse {
        http::HttpResponse {
            status: 403,
            headers: vec![
                ("Content-Type".into(), "text/plain".into()),
            ],
            body: "Forbidden.".into(),
        }
    }
    
    pub async fn unauthorized() -> http::HttpResponse {
        http::HttpResponse {
            status: 401,
            headers: vec![
                ("Content-Type".into(), "text/plain".into()),
            ],
            body: "Unauthorized.".into(),
        }
    }

    pub async fn handle_preflight() -> http::HttpResponse {
        
        // Just return empty response and let Cors handle headers.
        http::HttpResponse {
            status: 204,
            headers: vec![],
            body: "".into(),
        }
    }

}
