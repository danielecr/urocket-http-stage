/// arbiter between frontserv and backserv
/// 
/// There could be an arbiter in the middle:
///  - the arbiter provide a channel to frontserv
///  - the arbiter store the request_id associated with the channel (is it possible to store a rx in a hashmap? Maybe no, but it is possible to store rx in array?)
///  - the arbiter: 1. provide feedback to backserv, 2. send back response to frontserv, 3. dealloc/close the channel for synchronization
///  - the arbiter manage a timeout on the request, and return a standard reply

use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use tokio::sync::{mpsc, oneshot::{self, Receiver}};

use std::sync::Arc;
use tokio::sync::Mutex as TMutex;
extern crate toktor;
use toktor::actor_handler;


#[derive(Default,Serialize,Deserialize,Debug,Clone,PartialEq)]
pub struct ForHttpResponse {
    pub code: u32,
    pub data: serde_json::Value,
}

pub enum FrontResponse {
    BackMsg(ForHttpResponse),
    InternalError,
}

/// ProxyMsg
/// !!!TODO separate Subscriber from Fulfiller, also timeout is handled somewhere else
pub enum ProxyMsg {
    AddSubscriber {
        request_id: String,
        timeout: u64,
        respond_to: oneshot::Sender<FrontResponse>
    },
    FulfillRequest {
        request_id: String,
        response_payload: ForHttpResponse,
        respond_to: oneshot::Sender<bool>
    }
}

/// Arbiter
/// Does not it sound weird that it is storing the ProxyMsg::FulfillRequest's as well?
/// !!!TODO: review this
struct Arbiter {
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
                let tx = respond_to;
                let subscriptions = self.subscriptions.clone();
                tokio::spawn(async move {
                    let mut subscrs = subscriptions.lock().await;
                    (*subscrs).insert(request_id.clone(), ProxyMsg::AddSubscriber { request_id: request_id, timeout, respond_to: tx });
                    drop(subscrs);
                });
            },
            ProxyMsg::FulfillRequest { request_id, response_payload, respond_to } => {
                let subscriptions = self.subscriptions.clone();
                //let response_payload = response_payload.clone();
                tokio::spawn(async move {
                    let mut subscrs = subscriptions.lock().await;
                    
                    if let Some(m) = subscrs.remove(&request_id) {
                        match m {
                            ProxyMsg::AddSubscriber { request_id: _, timeout: _, respond_to: tx } => {
                                let _ = tx.send(FrontResponse::BackMsg(response_payload));
                                let _ = respond_to.send(true);
                            },
                            _ => {
                                // Literally impossible: ProxyMsg::FulfillRequest is never inserted
                            }
                        };
                    } else {
                        // !!!TODO:
                        // 1. something else replied 
                        // 2. receiving a message not belonging to arbiter
                        // in any case log the message
                        let _ = respond_to.send(false);
                    }
                });
            }
        };
    }
}

actor_handler!({} => Arbiter, ArbiterHandler, ProxyMsg);

use crate::toktor_send;

impl ArbiterHandler {
    pub fn add_request(&self) -> (Receiver<FrontResponse>, String) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let unique: String = uuid::Uuid::new_v4().to_string();
        let msg_sub = ProxyMsg::AddSubscriber {
            request_id: unique.clone(),
            timeout: 40000,
            respond_to: tx
        };
        let s = self.clone();
        tokio::spawn(async move{
            match toktor_send!(s, msg_sub).await {
                _ => {}//println!("anyway")
            };
        });
        (rx,unique)
    }
    
    pub async fn fulfill_request(&self, request_id: &str, payload: ForHttpResponse) -> Receiver<bool> {
        
        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg_ff = ProxyMsg::FulfillRequest {
            request_id: request_id.to_string(),
            response_payload: payload,
            respond_to: tx
        };
        
        match toktor_send!(self, msg_ff).await {
            _ => println!("sent the ff message")
        };
        rx
    }
}

#[cfg(test)]
mod tests {
    use crate::toktor_new;
    use super::*;
    
    #[tokio::test]
    async fn arbiter_run() {
        let arbiter = toktor_new!(ArbiterHandler);
        let (rx, req_id) = arbiter.add_request();
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let rpay = ForHttpResponse::default();
        let rx2  = arbiter.fulfill_request(&req_id.clone(), rpay.clone()).await;
        // should arrive rx: delivering payload rpay
        match rx.await {
            Ok(m) => {
                match m {
                    FrontResponse::BackMsg(m)=> {
                        println!("payload to give back: {:?}",m.clone());
                        assert_eq!(m.clone(),rpay);
                    },
                    FrontResponse::InternalError => {
                        eprintln!("Internal error (unspecified)");
                        assert_eq!(false, true);
                    }
                }
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