use bytes::Bytes;
use http_body_util::Full;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use tokio::net::UnixListener;
use hyper_util::rt::TokioIo;

use std::convert::Infallible;


async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

#[tokio::main]
async fn main() {
    let socketpath = "/tmp/notexistent.sock";
    let path = std::path::Path::new(socketpath);

    if path.exists() {
        tokio::fs::remove_file(path).await.expect("Could not remove old socket!");
    }

    let listener = UnixListener::bind(path).unwrap();

    //let listener = TcpListener::bind(addr).await.unwrap();
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(stream);
        
        tokio::task::spawn(async move {
            if let Err(err) = // http1::Builder::new()
            http1::Builder::new().serve_connection(
                io,
                service_fn(hello),
            )
            .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}