/// arbiter between frontserv and backserv
/// 
/// There could be an arbiter in the middle:
///  - the arbiter provide a channel to frontserv
///  - the arbiter store the request_id associated with the channel (is it possible to store a rx in a hashmap? Maybe no, but it is possible to store rx in array?)
///  - the arbiter: 1. provide feedback to backserv, 2. send back response to frontserv, 3. dealloc/close the channel for synchronization
///  - the arbiter manage a timeout on the request, and return a standard reply

extern crate toktor;
use std::collections::HashMap;

use serde::Serialize;
use toktor::actor_handler;
use tokio::sync::{mpsc, oneshot};

#[derive(Default)]
pub struct ForHttpResponse {
    pub code: u32,
    pub data: serde_json::Value,
}

//#[derive(Serialize)]
pub enum ProxyMsg {
    AddSubscriber {
        request_id: String,
        timeout: u64,
        respond_to: oneshot::Sender<ForHttpResponse>
    }
}

pub struct Arbiter {
    receiver: mpsc::Receiver<ProxyMsg>,
    subscriptions: HashMap<String,ProxyMsg>,
}

impl Arbiter {
    pub fn new(receiver: mpsc::Receiver<ProxyMsg>) -> Self {
        Arbiter {
            receiver,
            subscriptions: HashMap::new()
        }
    }
    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg);
        }
    }
    fn handle_message(&mut self, msg: ProxyMsg) {

    }
}

actor_handler!({} => Arbiter, ArbiterHandler, ProxyMsg);