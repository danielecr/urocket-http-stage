
use tokio::io::{self, AsyncReadExt};

use urocket_http_stage::cmdlineparser::parse;
//use urocket_http_stage::serviceconf::ServiceConf;

use urocket_http_stage::{toktor_send, toktor_new};

use urocket_http_stage::arbiter::*;

#[tokio::main]
async fn main() -> Result<(),()> {
    let mut config = parse();
    config.parse_configfile().await;

    let arbiter = toktor_new!(ArbiterHandler);
    let (tx, rx) = tokio::sync::oneshot::channel();
    let msg_sub = ProxyMsg::AddSubscriber {
        request_id: String::from("123"),
        timeout: 40000,
        respond_to: tx
    };

    match toktor_send!(arbiter, msg_sub).await {
        _ => println!("anyway")
    };
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let rpay = ForHttpResponse::default();
    let (tx2, rx2) = tokio::sync::oneshot::channel();
    let msg_ff = ProxyMsg::FulfillRequest {
        request_id: String::from("123"),
        response_payload: rpay,
        respond_to: tx2
    };

    match toktor_send!(arbiter, msg_ff).await {
        _ => println!("sent the ff message")
    };
    // should arrive rx: delivering payload rpay
    match rx.await {
        Ok(m) => {
            println!("payload to give back: {:?}",m.clone());
        },
        Err(e) => panic!("er {:?}",e)
    };
    // then it should arrive rx2: the payload is accepted/rejected
    match rx2.await {
        Ok(r) => println!("it does succeed? {}",r),
        Err(e) => panic!("it does not succeeded {}",e)
    }
    println!("Hello, world!");
    Ok(())
}
