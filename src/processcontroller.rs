/// Process Controller - Controls OS process spawned, and stops them if timeout expires
/// This just spawn process and after timeout send kill 9 (SIGKILL) and wait4 to get exit status
/// There are three cases:
///  1. timeout
///  2. normal termination
///  3. abnormal termination (exit code != 0)
/// Timeout is controlled by `timeout` in ProcEnv, after timeout ms the process receive
/// kill() with SIGKILL (9).
/// Note: timeout's kill is called in a std::thread::spawn, not tokio async rt, this
/// is more reliable.
/// ProcessInfos containing the execution infos with details (including stderr and stdout),
/// can be requested by:
/// 
///   let (tx, mut rx) = tokio::sync::mpsc::channel(1);
///   pc.get_infos(&uuid, tx).await;
///   match rx.recv().await {
///     Some(r: Option<ProcessInfos>) => {
///         println!("Received {:?}", r);
///     },
///     _ => {
///         //println!("receive error {:?}", _);
///     }
///   };
/// 
/// it might returns None if uuid is wrong or the request is older than 4 seconds.
/// After 4 seconds the ProcessInfos is thrown away to free memory,
/// the frontserv must be quick enough to collect or forget stats about process


use std::sync::Arc;
use tokio::sync::{Mutex as TMutex, mpsc};
use tokio::sync::mpsc::Sender;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use wait4::{ResUse, Wait4};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{procenv::ProcEnv, restmessage::RestMessage};
extern crate toktor;
use toktor::actor_handler;
use crate::toktor_send;

fn get_now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH)
    .unwrap().as_millis()
}

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

/// ProcessInfos contain the execution infos
/// uuid is the request id, pid is the pid, and so on
#[derive(Default, Debug)]
pub struct ProcessInfos {
    uuid: String,
    pid: u32,
    start_ms: u128,
    stop_ms: u128,
    resources: Option<ResUse>,
    was_killed: bool,
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

    async fn add_proc_infos(b: AtomicHash, pid: u32, start_ms: u128, stop_ms: u128, was_killed: bool, uuid: &str, ruse: ResUse, sout: String, serr: String) {
        let mut infos = b.lock().await;
        let pi = ProcessInfos {
            uuid: uuid.to_string(),
            pid,
            start_ms,
            stop_ms,
            resources: Some(ruse),
            was_killed,
            stdout: sout,
            stderr: serr,
        };
        (*infos).insert(uuid.to_string(), pi);
        drop(infos);
    }

    fn handle_message(&mut self, msg: ProcMsg) {
        match msg {
            ProcMsg::AddProc { proce, rest_message, uuid } => {
                let proc_infos = self.proc_infos.clone();
                tokio::spawn(async move {
                    let start_ms = get_now_ms();
                    let timeout = proce.timeout.unwrap_or(1000);
                    let cmd_and_args = proce.cmd_to_arr_replace("{{jsonpayload}}", rest_message.body());
                    let comma = format!("Cmd{}: {:?}",&uuid, cmd_and_args);
                    let mut cmd_ex = Command::new(&cmd_and_args[0]);
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
                    
                    let pid = child.id();
                    let in_millis = std::time::Duration::from_millis(timeout as u64);
                    let eutanasia = std::thread::spawn(move || {
                        // sleep for at least the specified amount of time
                        std::thread::sleep(in_millis);
                        unsafe { libc::kill(pid as i32, libc::SIGTERM) }
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
                            Ok(s) => s,
                            Err(e) => format!("EE: {:?}",e)
                        }
                    }).collect::<Vec<String>>().join("\n");
                    let stderr_buf = stderr_lines.map(|x|{
                        match x {
                            Ok(s) => s,
                            Err(e) => format!("EE: {:?}",e)
                        }
                    }).collect::<Vec<String>>().join("\n");
                    match child.wait4() {
                        Ok(ruse)=> {
                            let stop_ms = get_now_ms();
                            let was_killed = if eutanasia.is_finished() {
                                eutanasia.join().unwrap() != -1
                            } else {
                                false
                            };
                            Self::add_proc_infos(proc_infos.clone(), pid, start_ms, stop_ms, was_killed, &uuid, ruse, stdout_buf, stderr_buf).await;
                        }
                        Err(e) => {
                            println!("Execution ERROR Pid({pid}) {comma}{}", e);
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
    
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn run_process_controller() {
        let proco = toktor_new!(ProcessController);
        let j = serde_json::json!({"error": null, "data": [{"this":false,"that":true}]});
        let pl = serde_json::to_string(&j).unwrap();
        let req = RestMessage::new("POST", "/put/staff/in", &pl);
        //let proce = ProcEnv::new("", vec![], "echo {{jsonpayload}} $REQUEST_ID $SHELL", "");
        let mut proce = ProcEnv::new("", vec![], "sleep 2", "");
        proce.timeout = Some(300);
        proco.run_back_process(&proce, req, "123123123123").await;
        println!("now await ...");
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(8000)).await;
        println!("the time is over");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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