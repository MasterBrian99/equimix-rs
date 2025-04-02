use http_body_util::Full;
use hyper_util::rt::TokioIo;
use serde::Deserialize;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::fs;

use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use tokio::net::TcpListener;
mod round_robin;
use hyper::server::conn::http1::Builder;

#[derive(Debug, Deserialize)]
struct Config {
    #[allow(dead_code)]
    config: ConfigDetails,
    #[allow(dead_code)]
    servers: Vec<Servers>,
}

#[derive(Deserialize, Debug)]
struct ConfigDetails {
    #[allow(dead_code)]
    algo: String,
}

#[derive(Deserialize, Debug)]
struct Servers {
    #[allow(dead_code)]
    url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let contents = match fs::read_to_string("config.toml").await {
        Ok(contents) => contents,
        Err(error) => panic!("Error reading file: {:?}", error),
    };

    let cargo_toml: Config = toml::from_str(&contents).expect("Failed to deserialize Cargo.toml");
    let server_urls: Vec<String> = cargo_toml.servers.into_iter().map(|s| s.url).collect();
    let  lb = Arc::new(round_robin::RoundRobin::new(server_urls));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    // Start the load balancer server
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let  lb = lb.clone();

        tokio::task::spawn(async move {
            let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                let   lb = lb.clone();
                async move {
                    match lb.get_next_server().await {
                        Some(backend_url) => {
                            println!("Routing request to {}", backend_url);
                            Response::builder()
                                .status(StatusCode::OK)
                                .body(Full::new(Bytes::from(format!("Proxied to {}", backend_url))))
                                .map_err(|e| e.to_string())
                        },
                        None => {
                            Response::builder()
                                .status(StatusCode::SERVICE_UNAVAILABLE)
                                .body(Full::new(Bytes::from("No healthy backends available")))
                                .map_err(|e| e.to_string())
                        }
                    }
                }
            });

        });
    }
}

