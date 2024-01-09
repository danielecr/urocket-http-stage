use tokio::sync::{mpsc, oneshot::{Receiver, Sender, self}};

extern crate toktor;
use toktor::actor_handler;
use crate::{toktor_send, arbiter::ForHttpResponse, arbiter::FrontResponse, serviceconf::ServiceConf, processcontroller::ProcessController};

use crate::arbiter::ArbiterHandler;

use crate::restmessage::RestMessage;

pub enum ReqVisorMsg {
    RegisterPending {
        //req: Request<IncomingBody>,
        req: RestMessage,
        respond_to: Sender<(Receiver<FrontResponse>,String)>
    },
    FulfillPending {
        req_id: String,
        response: ForHttpResponse,
        respond_to: Sender<Result<bool,()>>
    }
}

struct RequestsVisorActor {
    receiver: mpsc::Receiver<ReqVisorMsg>,
    arbiter: ArbiterHandler,
    pctl: ProcessController,
    config: ServiceConf
}

impl RequestsVisorActor {
    pub fn new(receiver: mpsc::Receiver<ReqVisorMsg>, arbiter: &ArbiterHandler, pctl: &ProcessController, conf: &ServiceConf) -> Self {
        //println!("REQUEST ACTOR:: {:?}",conf);
        RequestsVisorActor {
            receiver,
            arbiter: arbiter.clone(),
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
            ReqVisorMsg::RegisterPending { req, respond_to: tx } => {
                let arb = self.arbiter.clone();
                let config = self.config.clone();
                let pctl = self.pctl.clone();
                tokio::spawn(async move {
                    match config.match_request(&req) {
                        Some(va) => {
                            let (rx, uuid) = arb.add_request();
                            println!("Do something {:?}", va.inject);
                            if let Some(proce) = va.inject {
                                pctl.run_back_process(&proce, req, &uuid).await;
                            }
                            let _ = tx.send((rx,uuid));
                            // processcontroller.delegate(va.inject, uuid, req);
                        },
                        None => {
                            println!("RequestVisor: leider it does not match any executor");
                            // so return something like code 500 to the caller.
                            let (tx2, rx ) = oneshot::channel();
                            let _ = tx2.send(FrontResponse::InternalError);
                            let _ = tx.send((rx,String::from("")));
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
                let arb = self.arbiter.clone();
                tokio::spawn(async move {
                    let mypush = arb.fulfill_request(&req_id, response).await;
                    match mypush.await {
                        Ok(matched) => respond_to.send(Ok(matched)),
                        Err(e) => {
                            eprintln!("channel error: {:?}",e);
                            respond_to.send(Err(()))
                        }
                    }
                    //respond_to.send(Ok(true));
                });
            }
        };
    }
}

actor_handler!({arbiter: &ArbiterHandler, pctl: &ProcessController, conf: &ServiceConf} => RequestsVisorActor, RequestsVisor, ReqVisorMsg);


pub struct ErrorBack {

}
impl std::fmt::Display for ErrorBack {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "some kind of message")
    }
}

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
    pub async fn push_fulfill(&self, req_id: &str, response: ForHttpResponse)-> Result<bool,ErrorBack> {
        let (tx2, rx2) = tokio::sync::oneshot::channel();
        let msg = ReqVisorMsg::FulfillPending {
            req_id: req_id.to_string(),
            response,
            respond_to: tx2
        };
        match toktor_send!(self, msg).await {
            _ => println!()
        };
        match rx2.await {
            Ok(result) => {
                result.map_err(|_|{ErrorBack{}})
            }
            Err(_e) => {
                Err(ErrorBack {  })
            }
        }
        //Err(ErrorBack {  })
    }
}


#[cfg(test)]
mod tests {
    use crate::toktor_new;
    use super::*;

    #[tokio::test]
    async fn visor_run() {
        let arbiter = toktor_new!(ArbiterHandler);
        let conf = ServiceConf::default();
        let pctl = toktor_new!(ProcessController, &arbiter);
        let visor = toktor_new!(RequestsVisor, &arbiter, &pctl, &conf);
        let req = RestMessage::new("get", "/myurl", "");
        let rx = visor.wait_for(req);
        let (x, uuid) =  rx.await.unwrap();
        let response = ForHttpResponse { code: 1, data: serde_json::Value::String(String::from("helpme please")) };
        let a = visor.push_fulfill(&uuid, response).await;
        match a {
            Ok(d) => {
                println!("VISOR all fine good {}",d);
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
