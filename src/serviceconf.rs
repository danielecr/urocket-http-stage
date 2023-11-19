/** ServiceConf - reppresent the file servicedef.yaml as the configuration of the service
 * it includes all service configuration: register-notiservice.notitypes is a map
 * between notification type name and the notification type definition
 */
use std::collections::HashMap;

use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};
use serde::{Serialize, Deserialize};


#[derive(Deserialize,Debug)]
pub struct ServiceConf {
    pub servicename: String,
    pub socketpath: String,
    pub port: String,
}

async fn read_conf_file(conf_file: &str) -> String {
    // TODO: handle error!!
    let mut f = File::open(conf_file).await.unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).await.unwrap();
    match String::from_utf8(buffer) {
        Ok(s) => s,
        Err(e) => e.to_string()
    }
}

impl ServiceConf {
    pub async fn parse_service_def(configfilename: &str) -> ServiceConf {
        let content = read_conf_file(configfilename).await;
        println!("READ\n{}",content);

        match serde_yaml::from_str::<ServiceConf>(&content) {
            Ok(s) => {
                s
            },
            Err(e) => {
                panic!("\nPANIC Error reading configuration \n\nfile:{} > {e}\n", configfilename);
            }
        }
    }
}