/// The backserv listen on unix socket, as
/// specified in the config file
/// 

//use tower::{BoxError, ServiceBuilder};
//use tower_http::trace::TraceLayer;

use tracing::{span, warn, info, Level};

use bytes::Bytes;
//use hyper::Error;
//use http_body_util::{combinators::BoxBody, Empty};
use http_body_util::{BodyExt, Full};

//use http_body_util::{Full, BodyExt, StreamBody};

use hyper::body::Frame;
use hyper::server::conn::http1;
use hyper::service::Service;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use tokio::net::unix::UCred;
use tokio::net::{UnixListener, UnixStream};
use hyper_util::rt::TokioIo;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::requestsvisor::ForHttpResponse;
use crate::requestsvisor::RequestsVisor;

// code from axum example
#[derive(Clone,Debug)]
struct UdsConnectInfo {
    peer_addr: Arc<tokio::net::unix::SocketAddr>,
    peer_cred: UCred,
}

impl UdsConnectInfo {
    fn connect_info(target: &UnixStream) -> Self {
        let peer_addr = target.peer_addr().unwrap();
        let peer_cred = target.peer_cred().unwrap();
        
        Self {
            peer_addr: Arc::new(peer_addr),
            peer_cred,
        }
    }
}

pub async fn run_backserv(socketpath: &str, rv: &RequestsVisor) {
    let path = std::path::Path::new(socketpath);
    
    if path.exists() {
        tokio::fs::remove_file(path).await.expect("Could not remove old socket!");
    }
    span!(Level::WARN, "backserv");
    span!(Level::INFO, "backserv");
    let listener = UnixListener::bind(path).unwrap();
    info!("Backservice listening on unix:///{}", socketpath);
    //let listener = TcpListener::bind(addr).await.unwrap();
    loop {
        let (stream, socket) = listener.accept().await.unwrap();
        let ci = UdsConnectInfo::connect_info(&stream);
        let io = TokioIo::new(stream);

        
        let svc = Svc::new(ci, rv);
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
    ci: UdsConnectInfo,
    rv: T
}

impl<T: Clone> Svc<T> {
    fn new(ci: UdsConnectInfo, rv:&T) -> Svc<T> {
        Self { ci, rv: rv.clone() }
    }
}

fn uri_extract_req_id(uri: &hyper::Uri) -> Option<String> {
    if uri.path().starts_with("/urhttp/") {
        Some(uri.path().replace("/urhttp/", ""))
    } else {
        println!("bad news: {} ", uri.path());
        None
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
    info!("received payload from back: {}",_astr);
    serde_json::from_slice(&str)
}

impl Service<Request<IncomingBody>> for Svc<RequestsVisor> {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    
    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        info!("received connection from {:?}",self.ci);
        let vh = self.rv.clone();
        Box::pin(async move {
            let uri: hyper::Uri = req.uri().clone();
            match uri_extract_req_id(&uri) {
                Some(req_id) => {
                    let message = match getpayload(req).await {
                        Ok(r) => {
                            let payload = r;
                            ForHttpResponse { code: 200, data: payload }
                        },
                        Err(e) => {
                            warn!("error parsing backserv {}", e);
                            let payload = serde_json::Value::Bool(false);
                            ForHttpResponse { code: 500, data: payload }
                        }
                    };
                    let resp = vh.push_fulfill(&req_id, message);
                    //let bod = req.collect().await.unwrap().to_bytes();
                    match resp.await {
                        Ok(exresp) => {
                            //serde_json::to_string(value)
                            let message = if exresp {
                                Bytes::from("ok\n")
                            } else {
                                info!("sending to back 'no-matching'");
                                Bytes::from("Does not match any response\n")
                            };
                            let a = Response::builder().status(200).body(Full::new(message)).unwrap();
                            Ok(a)
                            //Ok(Response::builder().body(str).)
                            //let response = Response::new(str);
                            //let (mut parts, body) = response.into_parts();
                            //Ok(Response::from_parts(parts, body))
                        }
                        Err(_e) => {
                            let b = Response::builder().status(500).body(Full::new(Bytes::from(""))).unwrap();
                            Ok(b)
                        }
                    }
                }
                None => {
                    let a = Response::builder().status(200).body(Full::new(Bytes::from("not handled\n"))).unwrap();
                    Ok(a)
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
    