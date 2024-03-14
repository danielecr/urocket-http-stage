use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot::{Receiver, Sender, self}};

use std::sync::Arc;
use tokio::sync::Mutex as TMutex;

use tracing::{warn, info};

extern crate toktor;
use toktor::actor_handler;
use crate::{toktor_send, serviceconf::ServiceConf, processcontroller::ProcessController};

use crate::restmessage::RestMessage;


#[derive(Default,Serialize,Deserialize,Debug,Clone,PartialEq)]
pub struct ForHttpResponse {
    pub code: u32,
    pub data: serde_json::Value,
}

pub enum FrontResponse {
    BackMsg(ForHttpResponse),
    InternalError,
}

struct Subscriber {
    request_id: String,
    timeout: u64,
    respond_to: oneshot::Sender<FrontResponse>
}

enum ReqVisorMsg {
    RegisterPending {
        //req: Request<IncomingBody>,
        req: RestMessage,
        respond_to: Sender<(Receiver<FrontResponse>,String)>
    },
    FulfillPending {
        req_id: String,
        response: ForHttpResponse,
        respond_to: Sender<bool>
        // the response is true if req_id match some unfulfilled message
        // it is false elsewise
    }
}

struct RequestsVisorActor {
    receiver: mpsc::Receiver<ReqVisorMsg>,
    subscriptions: Arc<TMutex<HashMap<String,Subscriber>>>,
    pctl: ProcessController,
    config: ServiceConf
}

impl RequestsVisorActor {
    pub fn new(receiver: mpsc::Receiver<ReqVisorMsg>, pctl: &ProcessController, conf: &ServiceConf) -> Self {
        //println!("REQUEST ACTOR:: {:?}",conf);
        RequestsVisorActor {
            receiver,
            subscriptions: Arc::new(TMutex::new(HashMap::new())),
            pctl: pctl.clone(),
            config: conf.clone()
        }
    }

    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg);
        }
    }

    fn handle_message(&mut self, msg: ReqVisorMsg) {
        match msg {
            ReqVisorMsg::RegisterPending { req, respond_to } => {
                let subscriptions = self.subscriptions.clone();
                let config = self.config.clone();
                let pctl = self.pctl.clone();
                tokio::spawn(async move {
                    match config.match_request(&req) {
                        Some(va) => {
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            let uuid: String = uuid::Uuid::new_v4().to_string();
                            let msg_sub = Subscriber {
                                request_id: uuid.clone(),
                                timeout: 40000,
                                respond_to: tx
                            };
                            {
                                let mut subscrs = subscriptions.lock().await;
                                (*subscrs).insert(uuid.clone(), msg_sub);
                                drop(subscrs);
                            }
                            info!("associated action def {:?}", va.inject);
                            if let Some(proce) = va.inject {
                                pctl.run_back_process(&proce, req, &uuid).await;
                            } else {
                                warn!("not found");
                            }
                            let _ = respond_to.send((rx,uuid));
                        },
                        None => {
                            warn!("No executor associated");
                            // so return something like code 500 to the caller.
                            let (tx2, rx ) = oneshot::channel();
                            let _ = tx2.send(FrontResponse::InternalError);
                            let _ = respond_to.send((rx,String::from("")));
                        }
                    };

                    // here spawn the process that will eventually fulfill the request!
                    // the process is spawned based on RestMessage and configuration
                    // based on configuration, the events of: exit bad, timeout, etc.
                    // Could cause the exceptional handling of the req_id
                    // So the method is called as
                    // ProcessController::run_back_process(RestMessage, uuid);
                });
            }
            ReqVisorMsg::FulfillPending { req_id, response, respond_to } => {
                let subscriptions = self.subscriptions.clone();
                tokio::spawn(async move {
                    let mut subscrs = subscriptions.lock().await;
                    if let Some(m) = subscrs.remove(&req_id) {
                        let Subscriber { request_id: _ , timeout: _, respond_to: tx } = m;
                        let _ = tx.send(FrontResponse::BackMsg(response));
                        let _ = respond_to.send(true);
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

actor_handler!({pctl: &ProcessController, conf: &ServiceConf} => RequestsVisorActor, RequestsVisor, ReqVisorMsg);


impl RequestsVisor {
    pub fn wait_for(&self, req: RestMessage) -> Receiver<(Receiver<FrontResponse>,String)> {
        //let arbiter = self.arbiter.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg = ReqVisorMsg::RegisterPending {
            req,
            respond_to: tx,
        };
        let s = self.clone();
        tokio::spawn(async move {
            match toktor_send!(s, msg).await {
                _ => println!()
            };
        });
        return rx;
    }

    pub fn push_fulfill(&self, req_id: &str, response: ForHttpResponse)-> tokio::sync::oneshot::Receiver<bool> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg = ReqVisorMsg::FulfillPending {
            req_id: req_id.to_string(),
            response,
            respond_to: tx
        };
        let s = self.clone();
        tokio::spawn(async move {
            match toktor_send!(s, msg).await {
                _ => println!()
            };
        });
        rx
    }
}


#[cfg(test)]
mod tests {
    use crate::toktor_new;
    use super::*;

    #[tokio::test]
    async fn visor_run() {
        let conf = ServiceConf::default();
        let pctl = toktor_new!(ProcessController);
        let visor = toktor_new!(RequestsVisor, &pctl, &conf);
        let req = RestMessage::new("get", "/myurl", "");
        let rx = visor.wait_for(req);
        let (x, uuid) =  rx.await.unwrap();
        let response = ForHttpResponse { code: 1, data: serde_json::Value::String(String::from("helpme please")) };
        let rx = visor.push_fulfill(&uuid, response);
        match rx.await {
            Ok(d) => {
                if d {
                    println!("VISOR: Message matched {}",d);
                } else {
                    println!("VISOR: Message not matched {}",d);
                }
            },
            Err(e) => {
                println!("VISOR error {}",e);
            }
        }
        match x.await {
            Ok(message_back) => {
                match message_back {
                    FrontResponse::BackMsg(mb) => {
                        println!("VISOR caller finally got the message to send back: {:?}", mb);
                    },
                    FrontResponse::InternalError => {
                        eprintln!("Internal error");
                        //assert_eq!(false, true);
                    }
                }
            },
            Err(e) => {
                println!("VISOR caller received the channel error: {:?}",e);
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
    }
}
