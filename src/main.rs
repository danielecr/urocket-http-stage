use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};

use urocket_http_stage::cmdlineparser::parse;
use urocket_http_stage::serviceconf::ServiceConf;

#[tokio::main]
async fn main() -> Result<(),()> {
    let mut config = parse();
    config.parse_configfile().await;

    println!("Hello, world!");
    Ok(())
}
