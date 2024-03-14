
use tracing::info;
use urocket_http_stage::cmdlineparser::parse;

use urocket_http_stage::processcontroller::ProcessController;
use urocket_http_stage::toktor_new;

use urocket_http_stage::frontserv::run_front;
use urocket_http_stage::backserv::run_backserv;
use urocket_http_stage::requestsvisor::RequestsVisor;

use tracing_subscriber;

//use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
/*
tracing_subscriber::registry()
.with(
    tracing_subscriber::EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| "example_todos=debug,tower_http=debug".into()),
)
.with(tracing_subscriber::fmt::layer())
.init();
*/


#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<(),()> {
    tracing_subscriber::fmt::init();
    info!("version: 0.1.0-something");
    
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

    let pctl = toktor_new!(ProcessController);
    let requests_visor = toktor_new!(RequestsVisor, &pctl, &conf);
    let rv = requests_visor.clone();
    tokio::spawn(async move {
        run_front(&rv).await;
    });
    run_backserv(socketpath, &requests_visor).await;
    println!("Hello, world!");
    Ok(())
}
