use colored::Colorize;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::service::service_fn;
use hyper::{header, Request, Response, StatusCode, Uri};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use round_robin::RoundRobin;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::{fs, time};
mod round_robin;

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
    #[allow(dead_code)]
    health_check_path: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let contents = match fs::read_to_string("config.toml").await {
        Ok(contents) => contents,
        Err(error) => panic!("Error reading file: {:?}", error),
    };

    let cargo_toml: Config = toml::from_str(&contents).expect("Failed to deserialize Cargo.toml");
    let servers: Vec<Servers> = cargo_toml.servers.into_iter().collect();
    let lb = Arc::new(RoundRobin::new(servers));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    let lb_clone = lb.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(10));


        loop {
            interval.tick().await;

            let servers = match lb_clone.get_all_servers() {
                Some(servers) => servers,
                None => continue,
            };
            for  (i, ser) in servers.iter().enumerate() {
             let is_healthy=   health_check(&ser).await;
                if !is_healthy {
                    println!("Server {} is not healthy", ser.url.red());
                    lb_clone.update_healthy(i,false);
                    // ser.healthy = false;
                }else{
                    println!("Server {} is healthy", ser.url.green().bold());
                    lb_clone.update_healthy(i,true);
                }
            }
         
        }
    });
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let lb = lb.clone();

        tokio::task::spawn(async move {
            let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                let lb = lb.clone();
                async move {
                    let backend_url = match lb.get_next_server().await {
                        Some(url) => url,
                        None => {
                            return Ok::<_, hyper::Error>(
                                Response::builder()
                                    .status(StatusCode::SERVICE_UNAVAILABLE)
                                    .body(Full::new(Bytes::from("No healthy backends available")))
                                    .unwrap(),
                            );
                        }
                    };
                    println!("Routing {} {} to {}", req.method(), req.uri(), backend_url);

                    match forward_request(req, &backend_url).await {
                        Ok(response) => Ok(response),
                        Err(e) => {
                            eprintln!("Proxy error: {}", e);
                            Ok(Response::builder()
                                .status(StatusCode::BAD_GATEWAY)
                                .body(Full::new(Bytes::from(format!("Proxy error: {}", e))))
                                .unwrap())
                        }
                    }
                }
            });
            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                eprintln!("Connection error: {:?}", err);
            }
        });
    }
}

async fn forward_request(
    req: Request<hyper::body::Incoming>,
    backend_url: &str,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    let client: Client<HttpConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build(HttpConnector::new());

    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let target_uri: Uri = format!("{}{}", backend_url, path).parse()?;

    let (mut parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes();
    remove_hop_by_hop_headers(&mut parts.headers);
    let forwarded_req = Request::from_parts(parts, Full::new(body_bytes));
    let proxy_req = Request::builder()
        .method(forwarded_req.method())
        .uri(target_uri)
        .body(forwarded_req.into_body())?;
    let response = client.request(proxy_req).await?;
    let (parts, body) = response.into_parts();
    let body_bytes = body.collect().await?.to_bytes();

    Ok(Response::from_parts(parts, Full::new(body_bytes)))
}

fn remove_hop_by_hop_headers(headers: &mut header::HeaderMap) {
    const HOP_HEADERS: [header::HeaderName; 7] = [
        header::CONNECTION,
        header::PROXY_AUTHENTICATE,
        header::PROXY_AUTHORIZATION,
        header::TE,
        header::TRAILER,
        header::TRANSFER_ENCODING,
        header::UPGRADE,
    ];

    for header in HOP_HEADERS.iter() {
        headers.remove(header);
    }
}

async fn health_check(server: &round_robin::RoundRobinServers) -> bool {
    let client: Client<HttpConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build(HttpConnector::new());
    let url: Uri = format!("{}{}", server.url, server.health_check_path)
        .parse()
        .unwrap();
    match client.get(url).await {
        Ok(res) => res.status() == StatusCode::OK,
        Err(_) => false,
    }
}
