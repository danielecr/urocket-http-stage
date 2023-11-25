use std::sync::Arc;
use tokio::sync::{Mutex as TMutex, mpsc, oneshot};
use std::collections::HashMap;
use crate::{arbiter::ArbiterHandler, serviceconf::ServiceConf, restmessage::RestMessage};
extern crate toktor;
use toktor::actor_handler;
use crate::toktor_send;

enum ProcMsg {
    AddProc {
        rest_message: RestMessage,
        uuid: String
    },
    CheckTimeout,
}

#[derive(Default)]
struct ProcessInfos {
    uuid: String
}

struct ProcessController {
    receiver: mpsc::Receiver<ProcMsg>,
    arbiter: ArbiterHandler,
    proc_infos: Arc<TMutex<HashMap<String,ProcessInfos>>>,
}

impl ProcessController {
    pub fn new(receiver: mpsc::Receiver<ProcMsg>, arbiter: &ArbiterHandler, config: &ServiceConf) -> Self {
        ProcessController {
                receiver,
                arbiter: arbiter.clone(),
                proc_infos: Arc::new(TMutex::new(HashMap::new()))
        }
    }

    pub async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg);
        }
    }

    fn handle_message(&mut self, msg: ProcMsg) {
        match msg {
            ProcMsg::AddProc { rest_message, uuid } => {
                // TODO:
                // 1. match the rest_message with config
                // 2. create a process compatible
                // 3. store the process in a proclist for timeout
                // do some thing based on config
                tokio::spawn(async {
                    //let str = "ciao".to_string();
                    //let v8 = Vec::<u8>::from(str);
                    let a = tokio::process::Command::new("echo")
                    .arg("test.php")
                    .arg(" world")
                    .output();
                    let oo = a.await;
                    match oo {
                        Ok(xxx)=> {
                            println!("Execution STATUS ?? {:?}",xxx.status.success());
                            let oo = std::str::from_utf8(&xxx.stdout).unwrap();
                            println!("Execution STDOUT ?? {}",oo);
                            println!("Execution STDERR ?? {:#?}",xxx.stderr);
                        }
                        Err(e) => {
                            println!("Execution ERROR {}", e);
                        }
                    };
                });
            }
            ProcMsg::CheckTimeout => {
                // maybe has process info, so it kill the process after timeout
            }
        }
    }
}

actor_handler!({arbiter: &ArbiterHandler, config: &ServiceConf} => ProcessController, ProcessControllerHandler, ProcMsg);

