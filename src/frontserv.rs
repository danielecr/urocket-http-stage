/// The front service:
/// Accepts request from tcp port:
/// 1. assign a unique request id
/// 2. accordingly to conf file:
///   * rely request to backend (executor backend)
/// 
/// Accept command from other "actors"
/// (the only actor is the executor backserv):
/// 1. match the unique request id
/// 2. send back the payload received as a response to request_id
/// 
/// Problems:
/// - the frontservice callback synchronize with backserv: it waits until the corresponding response is ready.
/// - the backserv synchronize with the frontserv: a message sent to backend is matched with a waiting frontserv's message.
/// 
/// There could be an arbiter in the middle:
///  - the arbiter provide a channel to frontserv
///  - the arbiter store the request_id associated with the channel (is it possible to store a rx in a hashmap? Maybe no, but it is possible to store rx in array?)
///  - the arbiter: 1. provide feedback to backserv, 2. send back response to frontserv, 3. dealloc/close the channel for synchronization
///  - the arbiter manage a timeout on the request, and return a standard reply
/// 

use bytes::Bytes;
//use axum::body::Bytes;
//use axum::body::Full;
use hyper::Error;
//use http_body_util::{Full, BodyExt};
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};

//use http_body_util::{BodyExt, StreamBody};

use hyper::body::Frame;
use hyper::server::conn::http1;
use hyper::service::Service;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use tokio::net::TcpListener;
use hyper_util::rt::TokioIo;

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    //time::Duration, simd::SimdConstPtr,
    //net::SocketAddr
};
//use tower::{BoxError, ServiceBuilder};
//use tower_http::trace::TraceLayer;
//use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// use uuid::Uuid;

use crate::arbiter::{ArbiterHandler, self};

/*
tracing_subscriber::registry()
.with(
    tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| "example_todos=debug,tower_http=debug".into()),
)
.with(tracing_subscriber::fmt::layer())
.init();
*/
pub async fn run_front(arbiter: &ArbiterHandler) {
    // let db = Db::default();
    let addr: SocketAddr = ([0, 0, 0, 0], 8080).into();

    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on http://{}", addr);
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(stream);

        let svc = Svc::new(arbiter);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
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

            let (rx,req_id) = a.add_request().await;
            println!("I stored the reqid :: {}",&req_id);
            //let exresp  = rx.await;
            match rx.await {
                Ok(exresp) => {
                    //serde_json::to_string(value)
                    let response = serde_json::to_string(&exresp).unwrap();
                    let a = Response::builder().status(200).body(Full::new(Bytes::from(response))).unwrap();
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
