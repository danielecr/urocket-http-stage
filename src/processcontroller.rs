/// Process Controller - Controls OS process spawned, and stops them if timeout expires
/// This just spawn process and after timeout send kill 9 (SIGKILL) and wait4 to get exit status
/// (Well, that kill 9 feature is just in my mind, not in the code. !!!TODO: Change This.)
/// There are three cases:
///  1. timeout
///  2. normal termination
///  3. abnormal termination (exit code != 0)
/// Timeout and normal termination are not handled: requestsvisor would handle timeout by itself
/// and "normal termination without a feedback on the socket" would be handled as a timeout.
/// In case of abnormal termination, the policy is defined by the `exitAutoFeedback`:
/// - exitAutoFeedback: true, on exit !=0 send a 500 message (internal service error)
/// - exitAutoFeedback: false, does nothing (wait the timeout)
/// 
/// Process stdout and stderr are logged on stdout with req_id info, i.e.:
/// 
/// [ts] [req_id] - [stdout from process]
/// 
/// also this options is specific for each path

use std::sync::Arc;
use tokio::sync::{Mutex as TMutex, mpsc, oneshot};
use std::collections::HashMap;
use crate::{arbiter::ArbiterHandler, procenv::ProcEnv, restmessage::RestMessage};
extern crate toktor;
use toktor::actor_handler;
use crate::toktor_send;

enum ProcMsg {
    AddProc {
        proce: ProcEnv,
        rest_message: RestMessage,
        uuid: String
    },
    CheckTimeout,
}

impl ProcMsg {
    fn newproc(proce: &ProcEnv, restmessage: RestMessage, uuid: &str) -> Self {
        ProcMsg::AddProc {
            proce: proce.clone(),
            rest_message: restmessage,
            uuid: uuid.to_string()
        }
    }
}

#[derive(Default)]
struct ProcessInfos {
    uuid: String
}

struct ProcessControllerActor {
    receiver: mpsc::Receiver<ProcMsg>,
    arbiter: ArbiterHandler,
    proc_infos: Arc<TMutex<HashMap<String,ProcessInfos>>>,
}

impl ProcessControllerActor {
    pub fn new(receiver: mpsc::Receiver<ProcMsg>, arbiter: &ArbiterHandler) -> Self {
        ProcessControllerActor {
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
            ProcMsg::AddProc { proce, rest_message, uuid } => {
                // TODO:
                // 1. match the rest_message with config
                // 2. create a process compatible
                // 3. store the process in a proclist for timeout
                // do some thing based on config
                tokio::spawn(async move {
                    use std::io::{BufRead, BufReader};
                    use std::process::{Command, Stdio};
                    use wait4::{ResUse, Wait4};
                    
                    let timeout = proce.timeout.unwrap_or(1000);
                    let cmd_and_args = proce.cmd_to_arr_replace("{{jsonpayload}}", rest_message.body());
                    //println!("COMMMA: {:?}",cmd_and_args);
                    let comma = format!("Cmd{}: {:?}",&uuid, cmd_and_args);
                    //let mut cmd_ex = tokio::process::Command::new(&cmd_and_args[0]);
                    let mut cmd_ex = std::process::Command::new(&cmd_and_args[0]);
                    cmd_ex.env("REQUEST_ID", uuid.clone());
                    for argx in cmd_and_args.iter().skip(1) {
                        cmd_ex.arg(argx);
                    }
                    for (k,v) in proce.get_env() {
                        cmd_ex.env(k,v);
                    }
                    cmd_ex.stderr(Stdio::piped());
                    cmd_ex.stdout(Stdio::piped());
                    let mut child = cmd_ex.spawn().unwrap();
                    
                    let id = child.id();
                    let _ = tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(timeout as u64)).await;
                        println!("Time is over for {}",id as i32);
                        unsafe { libc::kill(id as i32, libc::SIGTERM); }
                    }).await;
                    let child_stdout = child
                    .stdout
                    .take()
                    .expect("Internal error, could not take stdout");
                    let child_stderr = child
                    .stderr
                    .take()
                    .expect("Internal error, could not take stderr");
                    let stdout_lines = BufReader::new(child_stdout).lines();
                    let stderr_lines = BufReader::new(child_stderr).lines();
                    for line in stdout_lines {
                        let line = line.unwrap();
                        println!("stdOut Pid({id}): {}", line);
                    }
                    for line in stderr_lines {
                        let line = line.unwrap();
                        println!("stdErr Pid({id}): {}", line);
                    }
                    match child.wait4() {
                        Ok(ruse)=> {
                            println!("Resource USAGE Pid({id}) {comma}?? {:?}",ruse);
                        }
                        Err(e) => {
                            println!("Execution ERROR Pid({id}) {comma}{}", e);
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

actor_handler!({arbiter: &ArbiterHandler} => ProcessControllerActor, ProcessController, ProcMsg);

impl ProcessController {
    pub async fn run_back_process(&self, proce: &ProcEnv, req: RestMessage, uuid: &str) -> () {
        let msg = ProcMsg::newproc(proce, req, uuid);
        match toktor_send!(self,msg).await {
            _ => {}
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::toktor_new;
    use super::*;
    
    #[tokio::test]
    async fn run_process_controller() {
        let arbiter = toktor_new!(ArbiterHandler);
        let proco = toktor_new!(ProcessController, &arbiter);
        let j = serde_json::json!({"error": null, "data": [{"this":false,"that":true}]});
        let pl = serde_json::to_string(&j).unwrap();
        let req = RestMessage::new("POST", "/put/staff/in", &pl);
        //let proce = ProcEnv::new("", vec![], "echo {{jsonpayload}} $REQUEST_ID $SHELL", "");
        let mut proce = ProcEnv::new("", vec![], "sleep 1", "");
        proce.timeout = Some(100);
        proco.run_back_process(&proce, req, "123123123123").await;
        println!("now await ...");
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        println!("the time is over");
    }

    #[tokio::test]
    async fn process_print_env() {
        let arbiter = toktor_new!(ArbiterHandler);
        let proco = toktor_new!(ProcessController, &arbiter);
        let j = serde_json::json!({"error": null, "data": [{"this":false,"that":true}]});
        let pl = serde_json::to_string(&j).unwrap();
        let req = RestMessage::new("POST", "/put/staff/in", &pl);
        let proce = ProcEnv::new_v("", vec!["MYENV=provolone"], &vec!["/bin/sh", "-c", "echo {{jsonpayload}} $REQUEST_ID myenv:$MYENV"], "");
        proco.run_back_process(&proce, req, "IQARRAY").await;
        println!("now await ...");
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        println!("the time is over");
    }
}