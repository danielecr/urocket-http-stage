
use urocket_http_stage::cmdlineparser::parse;
//use urocket_http_stage::serviceconf::ServiceConf;

use urocket_http_stage::toktor_new;

use urocket_http_stage::arbiter::*;

use urocket_http_stage::frontserv::run_front;
use urocket_http_stage::backserv::run_backserv;
use urocket_http_stage::requestsvisor::RequestsVisorHandler;

#[tokio::main]
async fn main() -> Result<(),()> {
    let mut config = parse();
    config.parse_configfile().await;

    let arbiter = toktor_new!(ArbiterHandler);
    let visor_handler = toktor_new!(RequestsVisorHandler, &arbiter);
    let vh = visor_handler.clone();
    tokio::spawn(async move {
        run_front(&vh).await;
    });
    run_backserv("/tmp/listenur.sock", &visor_handler).await;
    println!("Hello, world!");
    Ok(())
}
