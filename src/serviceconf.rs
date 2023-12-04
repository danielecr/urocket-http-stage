use std::collections::HashMap;

/// ServiceConf - reppresent the file servicedef.yaml as the configuration of the service
/// it includes all service configuration: register-notiservice.notitypes is a map
/// between notification type name and the notification type definition

// for uri scheme Other(T) as `outtake` validation of "usocket://mypath/bla"
// see: https://docs.rs/http/latest/src/http/uri/scheme.rs.html#21
// use http::uri::Uri;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize,Debug)]
pub struct ProcEnv {
    pub wd: String,
    pub env: Vec<String>,
    pub cmd: String,
    pub encoding: String,
    pub channel: String
}

#[derive(Serialize,Deserialize,Debug,Default)]
pub struct VerbAction {
    #[serde(default)]
    pub validatein: bool,
    #[serde(default)]
    pub inject: Option<ProcEnv>,
    pub outtake: String
}

#[derive(Serialize,Deserialize,Debug)]
pub struct PathVerb {
    get: VerbAction,
    post: Option<VerbAction>,
//    #[serde(rename="post")]
//    Post{ validate_in: bool,
//        inject: ProcEnv },
//    Delete{ validate_in: bool,
//        inject: ProcEnv },
}

#[derive(Serialize,Deserialize,Debug)]
pub struct PathVerbT {
    get: VerbAction
}

#[derive(Deserialize,Debug)]
pub struct ServiceConf {
    pub servicename: String,
    pub socketpath: String,
    pub port: String,
    //pub paths: HashMap<String, serde_json::Value>
    pub paths: HashMap<String, PathVerb>
}

async fn read_conf_file(conf_file: &str) -> String {
    // TODO: handle error!!
    let mut f = match File::open(conf_file).await {
        Ok(f) => f,
        Err(e) => {
            panic!("conf file error {:?} while trying to open file :\"{}\"\n\tMissing -c [filename] ??", e, conf_file);
        }
    };
    
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
                println!("{:?}",&s);
                s
            },
            Err(e) => {
                panic!("\nPANIC Error reading configuration \n\nfile:{} > {e}\n", configfilename);
            }
        }
    }
    pub fn get_socket(&self) -> &str {
        &self.socketpath
    }
}