/// arbiter between frontserv and backserv
/// 
/// There could be an arbiter in the middle:
///  - the arbiter provide a channel to frontserv
///  - the arbiter store the request_id associated with the channel (is it possible to store a rx in a hashmap? Maybe no, but it is possible to store rx in array?)
///  - the arbiter: 1. provide feedback to backserv, 2. send back response to frontserv, 3. dealloc/close the channel for synchronization
///  - the arbiter manage a timeout on the request, and return a standard reply

extern crate toktor;
use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use toktor::actor_handler;
use tokio::sync::{mpsc, oneshot};

use std::sync::Arc;
use tokio::sync::Mutex as TMutex;


#[derive(Default,Serialize,Deserialize,Debug,Clone,PartialEq)]
pub struct ForHttpResponse {
    pub code: u32,
    pub data: serde_json::Value,
}

pub enum ProxyMsg {
    AddSubscriber {
        request_id: String,
        timeout: u64,
        respond_to: oneshot::Sender<ForHttpResponse>
    },
    FulfillRequest {
        request_id: String,
        response_payload: ForHttpResponse,
        respond_to: oneshot::Sender<bool>
    }
}

pub struct Arbiter {
    receiver: mpsc::Receiver<ProxyMsg>,
    subscriptions: Arc<TMutex<HashMap<String,ProxyMsg>>>,
}

impl Arbiter {
    pub fn new(receiver: mpsc::Receiver<ProxyMsg>) -> Self {
        Arbiter {
            receiver,
            subscriptions: Arc::new(TMutex::new(HashMap::new()))
        }
    }
    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg);
        }
    }
    fn handle_message(&mut self, msg: ProxyMsg) {
        match msg {
            ProxyMsg::AddSubscriber { request_id, timeout, respond_to } => {
                {
                    let tx = respond_to;
                    let subscriptions = self.subscriptions.clone();
                    tokio::spawn(async move {
                        let mut subscrs = subscriptions.lock().await;
                        (*subscrs).insert(request_id.clone(), ProxyMsg::AddSubscriber { request_id: request_id, timeout, respond_to: tx });
                        drop(subscrs);
                    });
                }
            },
            ProxyMsg::FulfillRequest { request_id, response_payload, respond_to } => {
                let subscriptions = self.subscriptions.clone();
                //let response_payload = response_payload.clone();
                tokio::spawn(async move {
                    let mut subscrs = subscriptions.lock().await;
                    
                    if let Some(m) = subscrs.remove(&request_id) {
                        match m {
                            ProxyMsg::AddSubscriber { request_id: _, timeout: _, respond_to: tx } => {
                                let _ = tx.send(response_payload);
                                let _ = respond_to.send(true);
                            },
                            _ => {}
                        };
                    };
                });
            }
        };
    }
}

actor_handler!({} => Arbiter, ArbiterHandler, ProxyMsg);

#[cfg(test)]
mod tests {
    use crate::{toktor_send, toktor_new};
    use super::*;

    #[tokio::test]
    async fn arbiter_run() {
        let arbiter = toktor_new!(ArbiterHandler);
        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg_sub = ProxyMsg::AddSubscriber {
            request_id: String::from("123"),
            timeout: 40000,
            respond_to: tx
        };
        
        match toktor_send!(arbiter, msg_sub).await {
            _ => println!("anyway")
        };
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let rpay = ForHttpResponse::default();
        let (tx2, rx2) = tokio::sync::oneshot::channel();
        let msg_ff = ProxyMsg::FulfillRequest {
            request_id: String::from("123"),
            response_payload: rpay.clone(),
            respond_to: tx2
        };
        
        match toktor_send!(arbiter, msg_ff).await {
            _ => println!("sent the ff message")
        };
        // should arrive rx: delivering payload rpay
        match rx.await {
            Ok(m) => {
                println!("payload to give back: {:?}",m.clone());
                assert_eq!(m.clone(),rpay);
            },
            Err(e) => panic!("er {:?}",e)
        };
        // then it should arrive rx2: the payload is accepted/rejected
        match rx2.await {
            Ok(r) => {
                assert!(r, "it is true");
                println!("it does succeed? {}",r);
            },
            Err(e) => panic!("it does not succeeded {}",e)
        }
    }
}