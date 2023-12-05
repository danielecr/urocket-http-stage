
use urocket_http_stage::cmdlineparser::parse;
//use urocket_http_stage::serviceconf::ServiceConf;

use urocket_http_stage::serviceconf::ServiceConf;
use urocket_http_stage::toktor_new;

use urocket_http_stage::arbiter::ArbiterHandler;

use urocket_http_stage::frontserv::run_front;
use urocket_http_stage::backserv::run_backserv;
use urocket_http_stage::requestsvisor::RequestsVisor;

#[tokio::main]
async fn main() -> Result<(),()> {
    let mut config = parse();
    //config.set_config(ServiceConf::parse_service_def(&config.configfile).await);
    config.parse_configfile().await;
    let socketpath = if let Some(x) = &config.get_socket() {
        x
    } else {
        //panic!("socket path should be setted");
        "/tmp/urocketsocket.sock"
    };

    let conf = config.clone_paths();

    let arbiter = toktor_new!(ArbiterHandler);
    let requests_visor = toktor_new!(RequestsVisor, &arbiter, &conf);
    let rv = requests_visor.clone();
    tokio::spawn(async move {
        run_front(&rv).await;
    });
    run_backserv(socketpath, &requests_visor).await;
    println!("Hello, world!");
    Ok(())
}
