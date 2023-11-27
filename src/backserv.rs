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
use tokio::task::JoinError;

use std::future::Future;
use std::pin::Pin;

use crate::arbiter::{ArbiterHandler, ForHttpResponse};
use crate::requestsvisor::RequestsVisorHandler;
use crate::restmessage::{self, RestMessage};

pub async fn run_backserv(socketpath: &str, arbiter: &RequestsVisorHandler) {
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
    vh: T
}

impl<T: Clone> Svc<T> {
    fn new(vh:&T) -> Svc<T> {
        Self { vh: vh.clone() }
    }
}

fn uri_extract_req_id(uri: &hyper::Uri) -> String {
    // uri.path() is "/uri/{req_id}" -> ["","uri","{req_id}"]
    let rid = uri.path().split("/").nth(2);
    
    if let Some(reqid) = rid {
        reqid.to_string()
    } else {
        String::from("")
    }
}

async fn getpayload(req: Request<IncomingBody>) -> Result<serde_json::Value,serde_json::Error> {
    let frame_stream = req.into_body().map_frame(|frame| {
        let frame = if let Ok(data) = frame.into_data() {
            data.iter()
                .map(|byte| byte.to_be())
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
    let _astr = match std::str::from_utf8(&str) {
        Ok(s) => s,
        Err(e) => {eprintln!("err{}",e); ""}
    };
    println!("received payload from back: {}",_astr);
    serde_json::from_slice(&str)
}

impl Service<Request<IncomingBody>> for Svc<RequestsVisorHandler> {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        let vh = self.vh.clone();
        Box::pin(async move {
            let uri: hyper::Uri = req.uri().clone();
            let req_id = uri_extract_req_id(&uri);
            let message = match getpayload(req).await {
                Ok(r) => {
                    let payload = r;
                    ForHttpResponse { code: 200, data: payload }
                },
                Err(e) => {
                    eprintln!("error parsing backserv {}", e);
                    let payload = serde_json::Value::Bool(false);
                    ForHttpResponse { code: 500, data: payload }
                }
            };
            let resp = vh.push_fulfill(&req_id, message);
            //let bod = req.collect().await.unwrap().to_bytes();
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
