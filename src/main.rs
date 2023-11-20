
use urocket_http_stage::cmdlineparser::parse;
//use urocket_http_stage::serviceconf::ServiceConf;

use urocket_http_stage::toktor_new;

use urocket_http_stage::arbiter::*;

use urocket_http_stage::frontserv::run_front;

#[tokio::main]
async fn main() -> Result<(),()> {
    let mut config = parse();
    config.parse_configfile().await;

    let arbiter = toktor_new!(ArbiterHandler);
    run_front(arbiter).await;
    println!("Hello, world!");
    Ok(())
}
