use tokio::sync::{mpsc, oneshot::{self, Receiver, Sender}};

extern crate toktor;
use toktor::actor_handler;
use crate::{toktor_send, arbiter::ForHttpResponse};

use crate::arbiter::ArbiterHandler;

use crate::restmessage::RestMessage;

pub enum ReqVisorMsg {
    RegisterPending {
        //req: Request<IncomingBody>,
        req: RestMessage,
        respond_to: Sender<(Receiver<ForHttpResponse>,String)>
    },
    FulfillPending {
        req_id: String,
        response: ForHttpResponse,
        respond_to: Sender<Result<bool,()>>
    }
}

pub struct RequestsVisor {
    receiver: mpsc::Receiver<ReqVisorMsg>,
    arbiter: ArbiterHandler
}

impl RequestsVisor {
    pub fn new(receiver: mpsc::Receiver<ReqVisorMsg>, arbiter: &ArbiterHandler) -> Self {
        RequestsVisor {
            receiver,
            arbiter: arbiter.clone()
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
                tokio::spawn(async move {
                    let (rx, uuid) = arb.add_request();
                    let _ = tx.send((rx,uuid));
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
                    match mypush {
                        Ok(rec) => {
                            match rec.await {
                                Ok(x) => respond_to.send(Ok(x)),
                                Err(_) => respond_to.send(Ok(false))
                            }
                        },
                        Err(_x) => {
                            respond_to.send(Ok(false))
                        }
                    }
                    //respond_to.send(Ok(true));
                });
            }
        };
    }
}

actor_handler!({arbiter: &ArbiterHandler} => RequestsVisor, RequestsVisorHandler, ReqVisorMsg);


pub struct ErrorBack {

}
impl std::fmt::Display for ErrorBack {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "some kind of message")
    }
}

impl RequestsVisorHandler {
    pub fn wait_for(&self, req: RestMessage) -> Receiver<(Receiver<ForHttpResponse>,String)> {
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
            Ok(data) => {
                Ok(true)
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
        let visor = toktor_new!(RequestsVisorHandler, &arbiter);
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
                println!("error {}",e);
            }
        }
        match x.await {
            Ok(reason) => {
                println!("VISOR there are no reasons: {:?}", reason);
            },
            Err(e) => {
                println!("there are no error: {:?}",e);
            }
        }
}
}
