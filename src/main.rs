
use urocket_http_stage::cmdlineparser::parse;
//use urocket_http_stage::serviceconf::ServiceConf;

use urocket_http_stage::toktor_new;

use urocket_http_stage::arbiter::*;

#[tokio::main]
async fn main() -> Result<(),()> {
    let mut config = parse();
    config.parse_configfile().await;

    let _arbiter = toktor_new!(ArbiterHandler);
    println!("Hello, world!");
    Ok(())
}
