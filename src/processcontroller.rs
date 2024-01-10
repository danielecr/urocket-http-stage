/// Process Controller - Controls OS process spawned, and stops them if timeout expires
/// This just spawn process and after timeout send kill 9 (SIGKILL) and wait4 to get exit status
/// There are three cases:
///  1. timeout
///  2. normal termination
///  3. abnormal termination (exit code != 0)
/// Timeout is controlled by `timeout` in ProcEnv, after timeout ms the process receive
/// kill() with SIGKILL (9).
/// Normal termination.
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
use tokio::sync::{Mutex as TMutex, mpsc};
use tokio::sync::mpsc::Sender;
use std::collections::HashMap;
use crate::{procenv::ProcEnv, restmessage::RestMessage};
extern crate toktor;
use toktor::actor_handler;
use crate::toktor_send;

enum ProcMsg {
    AddProc {
        proce: ProcEnv,
        rest_message: RestMessage,
        uuid: String
    },
    GetInfos {
        uuid: String,
        tx: tokio::sync::mpsc::Sender<Option<ProcessInfos>>
    },
}

impl ProcMsg {
    fn new_proc(proce: &ProcEnv, restmessage: RestMessage, uuid: &str) -> Self {
        ProcMsg::AddProc {
            proce: proce.clone(),
            rest_message: restmessage,
            uuid: uuid.to_string()
        }
    }
    fn new_infos(uuid: &str, tx: Sender<Option<ProcessInfos>>) -> Self {
        ProcMsg::GetInfos { uuid: uuid.to_string(), tx }
    }
}

use wait4::{ResUse, Wait4};

#[derive(Default, Debug)]
pub struct ProcessInfos {
    uuid: String,
    resources: Option<ResUse>,
    stdout: String,
    stderr: String,
}

type AtomicHash = Arc<TMutex<HashMap<String, ProcessInfos>>>;

struct ProcessControllerActor {
    receiver: mpsc::Receiver<ProcMsg>,
    proc_infos: AtomicHash,
}

impl ProcessControllerActor {
    pub fn new(receiver: mpsc::Receiver<ProcMsg>) -> Self {
        ProcessControllerActor {
                receiver,
                proc_infos: Arc::new(TMutex::new(HashMap::new()))
        }
    }

    pub async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg);
        }
    }

    async fn add_proc_infos(b: AtomicHash, uuid: &str, ruse: ResUse, sout: String, serr: String) {
        //let b: Arc<TMutex<HashMap<String, ProcessInfos>>> = self.proc_infos.clone();
        let mut infos = b.lock().await;
        let pi = ProcessInfos {
            uuid: uuid.to_string(),
            resources: Some(ruse),
            stdout: sout,
            stderr: serr,
        };
        (*infos).insert(uuid.to_string(), pi);
        drop(infos);
    }

    fn handle_message(&mut self, msg: ProcMsg) {
        match msg {
            ProcMsg::AddProc { proce, rest_message, uuid } => {
                // TODO:
                // 1. match the rest_message with config
                // 2. create a process compatible
                // 3. store the process in a proclist for timeout
                // do some thing based on config
                let proc_infos = self.proc_infos.clone();
                tokio::spawn(async move {
                    use std::io::{BufRead, BufReader};
                    use std::process::{Command, Stdio};
                    
                    let timeout = proce.timeout.unwrap_or(1000);
                    let cmd_and_args = proce.cmd_to_arr_replace("{{jsonpayload}}", rest_message.body());
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
                    let _eutanasia = tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(timeout as u64)).await;
                        let res = unsafe { libc::kill(id as i32, libc::SIGTERM) };
                        println!("Killing result {res} {}",id as i32);
                    });
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
                    let stdout_buf = stdout_lines.map(|x|{
                        match x {
                            Ok(s) => format!("Pid({id}) OUT: {s}"),
                            Err(e) => format!("error: {:?}",e)
                        }
                    }).collect::<Vec<String>>().join("\n");
                    let stderr_buf = stderr_lines.map(|x|{
                        match x {
                            Ok(s) => format!("Pid({id}) ERR: {s}"),
                            Err(e) => format!("error: {:?}",e)
                        }
                    }).collect::<Vec<String>>().join("\n");
                    match child.wait4() {
                        Ok(ruse)=> {
                            //println!("Resource USAGE Pid({id}) {comma}?? {:?}",ruse);
                            Self::add_proc_infos(proc_infos.clone(), &uuid, ruse, stdout_buf, stderr_buf).await;
                        }
                        Err(e) => {
                            println!("Execution ERROR Pid({id}) {comma}{}", e);
                        }
                    };
                    let _selfclean = tokio::spawn(async move {
                        let pis = proc_infos.clone();
                        tokio::time::sleep(tokio::time::Duration::from_millis((timeout as u64)+ 4000)).await;
                        let mut pis = pis.lock().await;
                        let pi = (*pis).remove(&uuid);
                        if let Some(pi) = pi {
                            println!("after long run, removing staff {:?}", pi);
                        }
                    }).await;
                    // this is gone: out of scope, does not matter
                    //let _ = eutanasia.await;
                });
            }
            ProcMsg::GetInfos { uuid, tx } => {
                // return process infos, and resource usage
                let proc_infos = self.proc_infos.clone();
                tokio::spawn(async move {
                    let mut b = proc_infos.lock().await;
                    let pi = (*b).remove(&uuid);
                    //let pi = (*b).get(&uuid);
                    if let Some(pi) = pi {
                        let _ = tx.send(Some(pi)).await;
                    } else {
                        let _ = tx.send(None).await;
                    }
                });
            }
        }
    }
}

actor_handler!({} => ProcessControllerActor, ProcessController, ProcMsg);

impl ProcessController {
    pub async fn run_back_process(&self, proce: &ProcEnv, req: RestMessage, uuid: &str) -> () {
        let msg = ProcMsg::new_proc(proce, req, uuid);
        match toktor_send!(self,msg).await {
            _ => {}
        };
    }

    pub async fn get_infos(&self, uuid: &str, tx: Sender<Option<ProcessInfos>>) -> () {
        let msg = ProcMsg::new_infos(uuid, tx);
        match toktor_send!(self, msg).await {
            _ => {}
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::toktor_new;
    use super::*;
    
    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    async fn run_process_controller() {
        let proco = toktor_new!(ProcessController);
        let j = serde_json::json!({"error": null, "data": [{"this":false,"that":true}]});
        let pl = serde_json::to_string(&j).unwrap();
        let req = RestMessage::new("POST", "/put/staff/in", &pl);
        //let proce = ProcEnv::new("", vec![], "echo {{jsonpayload}} $REQUEST_ID $SHELL", "");
        let mut proce = ProcEnv::new("", vec![], "sleep 2", "");
        proce.timeout = Some(4000);
        proco.run_back_process(&proce, req, "123123123123").await;
        println!("now await ...");
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(7000)).await;
        println!("the time is over");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    async fn process_print_env() {
        let proco = toktor_new!(ProcessController);
        let j = serde_json::json!({"error": null, "data": [{"this":false,"that":true}]});
        let pl = serde_json::to_string(&j).unwrap();
        let req = RestMessage::new("POST", "/put/staff/in", &pl);
        let proce = ProcEnv::new_v("", vec!["MYENV=provolone"], &vec!["/bin/sh", "-c", "echo {{jsonpayload}} $REQUEST_ID myenv:$MYENV"], "");
        proco.run_back_process(&proce, req, "IQARRAY").await;
        println!("now await ...");
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
        println!("the time is over");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    async fn process_get_infos() {
        let proco = toktor_new!(ProcessController);
        let j = serde_json::json!({"error": null, "data": [{"this":false,"that":true}]});
        let pl = serde_json::to_string(&j).unwrap();
        let req = RestMessage::new("POST", "/put/staff/in", &pl);
        let proce = ProcEnv::new_v("", vec!["MYENV=provolone"], &vec!["/bin/sh", "-c", "echo {{jsonpayload}} $REQUEST_ID myenv:$MYENV"], "");
        let uuid = String::from("REQUEST-ID1");
        proco.run_back_process(&proce, req, &uuid).await;
        println!("now await ...");
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        proco.get_infos(&uuid, tx).await;
        match rx.recv().await {
            Some( r) => {
                println!("Received {:?}", r);
            },
            _ => {
                //println!("receive error {:?}", _);
            }
        };
        tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
        println!("the time is over");
    }
}