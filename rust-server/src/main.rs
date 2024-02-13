use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};
use tokio_postgres::{NoTls, types::ToSql, Client};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use serde::{Deserialize, Serialize};
use futures::future::BoxFuture;
use futures::stream::SplitSink;
use futures::{stream};
use futures::TryStreamExt;
use once_cell::sync::Lazy as LazyOnceCell;
use lazy_static::lazy_static;
use async_trait::async_trait;
use rust_decimal::Decimal;
use tokio::io::Interest;
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use futures::FutureExt;
use std::sync::Arc;
use memmap2::MmapOptions;
use std::io::Write;
use std::io::Read;
use sha256;
use rand::Rng;
use regex::Regex;
use serde_json;
use reqwest;
use url;

mod api;
mod http;
mod config;
mod routes;
mod websocket;
mod routehandler;
mod tradealgorithm;

use routehandler::RouteHandler;
use routes::Routes;
use api::ExchangeAPI;
use tradealgorithm::{TradeAlgorithm, CandleStick};

// Defined types
//
// Type for PostgreSQL-wrapper.
type Psql = Arc<Mutex<Client>>;

// Type for API-wrapper.
type Api = Arc<dyn ExchangeAPI>;

// Type for tuple containing routes. (HTTP-method, path, fn pointer to route function).
type Route = (
    &'static str,
    &'static str,
     fn(&'static Routes, http::Http, Psql, Api)
        -> BoxFuture<'static, Result<http::HttpResponse, http::Error>>
);

// Macros
//
// This macro improves the readability of the code. Instead of
// ("GET", "/", |r, h| Routes::index(r, h).boxed()) the programmer can just
// type route!("GET", "/", Routes::index).
macro_rules! route {
    ($m:expr, $p:expr, $fn:path) => {
        ($m, $p, |r, h, c, a| $fn(r, h, c, a).boxed())
    }
}

// Macro to improve the readability to convert a f64 Rust type to
// the NUMERIC type from PostgreSQL.
// Instead of Decimal::from_f64_retain(500.00).unwrap() as &(dyn ToSql + Sync)
// the programmer can type sqlf64!(500.00).
#[macro_export]
macro_rules! sqlf64 {
    ($n: expr) => {
        &Decimal::from_f64_retain($n).unwrap() as &(dyn ToSql + Sync)
    }
}
#[macro_export]
macro_rules! sqldec {
    ($n: expr) => {
        $n.to_string().parse::<f64>().unwrap()
    }
}

#[macro_export]
macro_rules! json_str_to_f64 {
    ($s: expr) => {
        $s.as_str().unwrap().parse::<f64>().unwrap()
    }
}


lazy_static! {
    pub static ref ROUTES : std::vec::Vec<Route> = vec![
        route!("GET", "/ping", Routes::ping),
        route!("GET", "/ping-exchange", Routes::ping_exchange),
        route!("POST", "/algorithms/{id}/start", Routes::start_algorithm),
        route!("POST", "/algorithms/{id}/stop", Routes::stop_algorithm),
        route!("GET", "/algorithms/{id}", Routes::get_algorithm),
        route!("GET", "/algorithms/{id}/history/{start_at}", Routes::get_algorithm_history),
        route!("GET", "/algorithms/{id}/code", Routes::get_algorithm_code),
        route!("PUT", "/algorithms/{id}/reset", Routes::reset_algorithm),
        route!("GET", "/algorithms/{id}/chart/{interval}", Routes::get_algorithm_chart),
        route!("POST", "/algorithms/add", Routes::add_algorithm),
        route!("DELETE", "/algorithms/{id}", Routes::delete_algorithm),
        route!("GET", "/balance", Routes::balance),
        route!("PUT", "/users/init", Routes::init_user),
        route!("GET", "/trade_history", Routes::trade_history),
        route!("GET", "/btc_price", Routes::get_btc_price),
        route!("GET", "/klines/{interval}/{amount}", Routes::get_klines),
        route!("GET", "/btc_price", Routes::get_btc_price),
        route!("POST", "/order", Routes::order),
    ]; 
}

fn create_api_instance() -> api::binance::Binance {
    let api = ExchangeAPI::new(
        config::REST_API_URL.as_str(),
        config::WEBSOCKET_API_URL.as_str(),
        config::WEBSOCKET_STREAM_URL.as_str(),
        "",
        "",
    );

    api
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // PostgreSQL connection.
    let (client, mut connection) = tokio_postgres::connect(&*format!("host={} user={} password={} dbname={}",
            config::DB_HOST.as_str(),
            config::DB_USER.as_str(), 
            config::DB_PASS.as_str(), 
            config::DB_NAME.as_str()
            ), NoTls).await.unwrap();
    let client = Arc::new(Mutex::new(client)); 

    // Make transmitter and receiver.
    let (tx, _) = futures_channel::mpsc::unbounded();
    let stream = stream::poll_fn(move |cx| connection.poll_message(cx)).map_err(|e| panic!("{}", e));
    let listener_connection = stream.forward(tx).map(|r| r.unwrap());
    tokio::spawn(listener_connection);
        
    // Connect with API.
    let api = Arc::new(create_api_instance());

    // Start server.
    let server = TcpListener::bind("127.0.0.1:8080").await.expect("Failed to bind to 127.0.0.1:8080");
    println!("HTTP server on 127.0.0.1:8080...");
    
    let ws_server = TcpListener::bind("127.0.0.1:8081").await.expect("Failed to bind to 127.0.0.1:8080");
    println!("Websocket server on 127.0.0.1:8081...");

    // Insert BTC price into all algorithm history for accurate charts.
    let api_clone = api.clone();
    let client_clone = client.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(60000)).await;
            tradealgorithm::TradeAlgorithm::insert_btc_price(api_clone.clone(), client_clone.clone()).await.unwrap_or_default();
        }
    });

    let client_clone2 = client.clone();
    // Handle websocket.
    tokio::spawn(async move {
       while let Ok((ws_stream, _)) = ws_server.accept().await {
            tokio::spawn(websocket::handle_websocket(ws_stream, client_clone2.clone()));
        }
    });


    // Spawn process to handle incoming streams.
    loop {
    let client_clone3 = client.clone();
        // Stream for HTTP.
        let (mut stream, _) = server.accept().await.unwrap();

        tokio::spawn(async move {
            if let Err(e) = http::process(&mut stream, client_clone3.clone()).await {
                eprintln!("Error: {}", e);
            }
        });
    }

}

