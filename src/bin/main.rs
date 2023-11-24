
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
    let a2 = arbiter.clone();
    tokio::spawn(async move {
        run_front(&a2).await;
    });
    run_backserv("/tmp/listenur.sock", &arbiter).await;
    println!("Hello, world!");
    Ok(())
}
