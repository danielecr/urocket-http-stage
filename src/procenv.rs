/// ProcEnv process execution environment definition is used to store
/// the environment and the command to execute the
/// callback process that fulfill the frondend request
use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize,Debug,Clone)]
pub enum CmdDefinition {
    ToSplit(String),
    Splitted(Vec<String>)
}

impl Default for CmdDefinition {
    fn default() -> Self {
        CmdDefinition::ToSplit(String::from(""))
    }
}

impl From<&str> for CmdDefinition 
{
    fn from(value: &str) -> Self {
        Self::ToSplit(value.to_string())
    }
}

impl From<Vec<&str>> for CmdDefinition
{
    fn from(v: Vec<&str>) -> Self {
        Self::Splitted(v.iter().map(|x|{x.to_string()}).collect())
    }
}

impl<'a> CmdDefinition {
    pub fn cmd_to_arr_replace(&'a self, placeholder: &'a str, value: &'a str) -> Vec<String> {
        match &self {
            CmdDefinition::Splitted(x) => {
                x.iter().map(|x|{
                    let a = x.replace(placeholder, value).to_string();
                    String::from(a)
                }).collect()
            }
            CmdDefinition::ToSplit(c) => {
                c.split(&[' ','\t'][..])
                .map(|x|{
                    let a = x.replace(placeholder, value).to_string();
                    String::from(a)
                })
                .collect()
            }
        }
    }
}


#[derive(Serialize,Deserialize,Debug,Clone,Default)]
pub struct ProcEnv {
    pub wd: String,
    pub env: Vec<String>,
    pub cmd: CmdDefinition,
    pub encoding: String,
    pub channel: String
}

impl ProcEnv {
    pub fn new(wd: &str, env: Vec<String>, cmd: &str, encoding: &str) -> Self {
        ProcEnv {
            wd: wd.to_string(),
            env,
            //cmd: CmdDefinition::ToSplit(cmd.to_string()),
            cmd: CmdDefinition::from(cmd),
            encoding: encoding.to_string(),
            channel: "cmdline".to_string()
        }
    }
    pub fn new_v(wd: &str, env: Vec<String>, cmd: &[&str], encoding: &str) -> Self 
    {
        ProcEnv {
            wd: wd.to_string(),
            env,
            //cmd: CmdDefinition::Splitted(cmd.iter().map(|x|{x.to_string()}).collect()),
            cmd: CmdDefinition::from(cmd.to_vec()),
            encoding: encoding.to_string(),
            channel: "cmdline".to_string()
        }
    }
    
    pub fn cmd_to_arr_replace<'a>(&'a self, placeholder: &'a str, value: &'a str) -> Vec<String> {
        self.cmd.cmd_to_arr_replace(placeholder, value)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_cmd() {
        let cmd = CmdDefinition::from(vec!["bin/sh","echo hello world"]);
        let v = cmd.cmd_to_arr_replace("{{string}}", "hello");
        let mut c = std::process::Command::new(&v[0]);
        c.arg(&v[1]);
        // drop everything, it is enough for test
    }
}