/// The backserv listen on unix socket, as
/// specified in the config file
/// 

//use tower::{BoxError, ServiceBuilder};
//use tower_http::trace::TraceLayer;
//use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// use uuid::Uuid;

/*
tracing_subscriber::registry()
.with(
    tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| "example_todos=debug,tower_http=debug".into()),
)
.with(tracing_subscriber::fmt::layer())
.init();
*/



use bytes::Bytes;
use hyper::Error;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};

//use http_body_util::{Full, BodyExt, StreamBody};

use hyper::body::Frame;
use hyper::server::conn::http1;
use hyper::service::{Service, service_fn};
use hyper::{body::Incoming as IncomingBody, Request, Response};
use tokio::net::{TcpListener,UnixListener};
use hyper_util::rt::TokioIo;

use std::future::Future;
use std::pin::Pin;

use crate::arbiter::{ArbiterHandler, ForHttpResponse};

pub async fn run_backserv(socketpath: &str, arbiter: &ArbiterHandler) {
    let path = std::path::Path::new(socketpath);

    if path.exists() {
        tokio::fs::remove_file(path).await.expect("Could not remove old socket!");
    }

    let listener = UnixListener::bind(path).unwrap();

    //let listener = TcpListener::bind(addr).await.unwrap();
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(stream);
        
        let svc = Svc::new(arbiter);
        tokio::task::spawn(async move {
            if let Err(err) = // http1::Builder::new()
            http1::Builder::new().serve_connection(
                io,
                svc,
            )
            .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}

#[derive(Clone)]
struct Svc<T> {
    arbiter: T
}

impl<T: Clone> Svc<T> {
    fn new(arbiter:&T) -> Svc<T> {
        Self { arbiter: arbiter.clone() }
    }
}

impl Service<Request<IncomingBody>> for Svc<ArbiterHandler> {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        let a = self.arbiter.clone();
        Box::pin(async move {
            let uri = req.uri().clone();
            //println!("req: {:?}",req);
            //let bod = req.collect().await.unwrap().to_bytes();
            //println!("received {:?}", bod);
            let frame_stream = req.into_body().map_frame(|frame| {
                let frame = if let Ok(data) = frame.into_data() {
                    data.iter()
                        .map(|byte| byte.to_ascii_uppercase())
                        .collect::<Bytes>()
                } else {
                    Bytes::new()
                };

                Frame::data(frame)
            }).collect();
            //let (parts, body) = req.into_parts();
            //let body = serde_json::from_slice(&body).unwrap();
            let bites =  frame_stream.await.unwrap().to_bytes();
            //println!("received {:?}", bites);
            let str = Vec::<u8>::from(bites.as_ref());
            let astr = match std::str::from_utf8(&str) {
                Ok(s) => s,
                Err(e) => {eprintln!("err{}",e); ""}
            };
            println!("thats string {} for {}", astr, uri.path());
            let parts: Vec<_> = uri.path().split("/").collect();
            println!("parts: {:?}",parts);
            let req_id = if parts.len() == 3 {
                parts[2].to_string()
            } else {
                String::from("123")
            };
            println!("matching req_id:: {}", &req_id);

            let resp = a.fulfill_request(&req_id, ForHttpResponse{
                code: 200,
                data: serde_json::Value::Bool(true)
            });

            match resp.await {
                Ok(exresp) => {
                    //serde_json::to_string(value)
                    let a = Response::builder().status(200).body(Full::new(Bytes::from("ok va bene\n"))).unwrap();
                    Ok(a)
                    //Ok(Response::builder().body(str).)
                    //let response = Response::new(str);
                    //let (mut parts, body) = response.into_parts();
                    //Ok(Response::from_parts(parts, body))
                }
                Err(e) => {
                    let b = Response::builder().status(500).body(Full::new(Bytes::from(""))).unwrap();
                    Ok(b)
                }
            }
            
        })
    }
}

// use https://docs.rs/axum/latest/axum/routing/struct.Router.html#method.nest_service
// and tower service:
// https://docs.rs/tower-service/0.3.2/tower_service/trait.Service.html

// Everything that can be handled by proxyto
// would be maybe-filter-in, executed, maybe-filter-out, maybe-timedout
// 
// match proxyto(arbiter).await {
//   Ok(Consumed) => return Consumed
//   Err(e) =>
// }
