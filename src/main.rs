use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use round_robin::RoundRobin;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::fs;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper::body::Bytes;
use hyper::service::{self, service_fn};
use hyper::{Request, Response, StatusCode, Uri, };
use tokio::net::TcpListener;
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let contents = match fs::read_to_string("config.toml").await {
        Ok(contents) => contents,
        Err(error) => panic!("Error reading file: {:?}", error),
    };

    let cargo_toml: Config = toml::from_str(&contents).expect("Failed to deserialize Cargo.toml");
    let server_urls: Vec<String> = cargo_toml.servers.into_iter().map(|s| s.url).collect();
    let lb = Arc::new(RoundRobin::new(server_urls));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

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
        // let service = service_fn(move |req: Request<hyper::body::Incoming>| {
        //     let lb = lb.clone();
        //     async move {
        //         match lb.get_next_server().await {
        //             Some(backend_url) => {
        //                 println!("Routing request to {}", backend_url);
        //                 Response::builder()
        //                     .status(StatusCode::OK)
        //                     .body(Full::new(Bytes::from(format!("Proxied to {}", backend_url))))
        //                     .map_err(|e| e.to_string())
        //             }
        //             None => {
        //                 Response::builder()
        //                     .status(StatusCode::SERVICE_UNAVAILABLE)
        //                     .body(Full::new(Bytes::from("No healthy backends available")))
        //                     .map_err(|e| e.to_string())
        //             }
        //         }
        //     }

        // });
        // if let Err(err) = hyper::server::conn::http1::Builder::new()
        //     .serve_connection(io, service)
        //     .await
        // {
        //     println!("Error serving connection: {:?}", err);
        // }
    }
}

async fn forward_request(
    req: Request<hyper::body::Incoming>,
    backend_url: &str,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>>  {
    let client = Client::builder(TokioExecutor::new()).build(HttpConnector::new());
    
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let target_uri: Uri = format!("{}{}", backend_url, path).parse()?;

    let (parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes();
    let forwarded_req = Request::from_parts(parts, Full::new(body_bytes));
    
    let proxy_req = Request::builder()
        .method(forwarded_req.method())
        .uri(target_uri)
        .body(forwarded_req.into_body())?;

    let response = client.request(proxy_req).await?;
    let (parts, body) = response.into_parts();
    let body_bytes = body.collect().await?.to_bytes();
    
    Ok(Response::from_parts(parts, Full::new(body_bytes)))
    // let client = hyper::Client::builder()
    // .executor(TokioExecutor::new())
    // .build_http();

    // let path = req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
    // let target_uri = format!("{}{}", backend_url, path).parse()?;

    // let (mut parts, body) = req.into_parts();
    // remove_hop_by_hop_headers(&mut parts.headers);

    // let body_bytes = body.collect().await?.to_bytes();
    // let forwarded_req = Request::from_parts(parts, Full::new(body_bytes));

    // let proxy_req = Request::builder()
    // .method(forwarded_req.method())
    // .uri(target_uri)
    // .headers(forwarded_req.headers().clone())
    // .body(forwarded_req.into_body())?;

    // let response = client.request(proxy_req).await?;
    // Ok(response.map(|b| b.boxed()))
}
