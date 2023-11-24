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
    }
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
                // do some thing
            }
        }
    }
}

actor_handler!({arbiter: &ArbiterHandler, config: &ServiceConf} => ProcessController, ProcessControllerHandler, ProcMsg);

