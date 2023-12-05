use bytes::Bytes;
use hyper::{body::Frame, Method};
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{Request, body::Incoming as IncomingBody};

fn uri_extract_req_id(uri: hyper::Uri) -> String {
    // uri.path() is "/uri/{req_id}" -> ["","uri","{req_id}"]
    let rid = uri.path().split("/").nth(2);
    
    if let Some(reqid) = rid {
        reqid.to_string()
    } else {
        String::from("")
    }
}

/// Structure to keep the incoming request from frontserv
#[derive(Default,Debug)]
pub struct RestMessage {
    method: Method,
    uri: String,
    data: String,
}
impl RestMessage {
    pub fn new(m:&str, u:&str, d:&str) ->Self {
        let m = match Method::from_bytes(m.to_uppercase().as_bytes()) {
            Ok(m) => m,
            Err(_) => Method::GET
        };
        Self {method: Method::GET, uri: u.to_string(), data: d.to_string()}
    }
    /// Create a new RestMessage from the Request payload
    pub async fn parse_incoming(req: hyper::Request<IncomingBody>) -> Self {
        let method = req.method().clone();
        let uri = req.uri().path().to_string();
        let bites: Bytes = req.collect().await.unwrap().to_bytes();
        let str = Vec::<u8>::from(bites.as_ref());
        let body = match std::str::from_utf8(&str) {
            Ok(s) => s,
            Err(e) => {eprintln!("err{}",e); ""}
        };

        let data = String::from(body);
        Self{ method,uri , data}
    }
    pub fn method(&self) -> &Method {
        &self.method
    }
    pub fn uri(&self) -> &str {
        &self.uri
    }
    pub fn body(&self) -> &str {
        &self.data
    }
}