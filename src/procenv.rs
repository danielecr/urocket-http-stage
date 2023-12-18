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


#[derive(Serialize,Deserialize,Debug,Clone,Default)]
pub struct ProcEnv {
    pub wd: String,
    pub env: Vec<String>,
    pub cmd: CmdDefinition,
    pub encoding: String,
    pub channel: String
}

impl ProcEnv {
    pub fn new<T: std::string::ToString>(wd: &str, env: Vec<String>, cmd: T, encoding: &str) -> Self {
        ProcEnv {
            wd: wd.to_string(),
            env,
            cmd: CmdDefinition::ToSplit(cmd.to_string()),
            encoding: encoding.to_string(),
            channel: "cmdline".to_string()
        }
    }
    pub fn new_v(wd: &str, env: Vec<String>, cmd: &[&str], encoding: &str) -> Self 
    {
        ProcEnv {
            wd: wd.to_string(),
            env,
            cmd: CmdDefinition::Splitted(cmd.iter().map(|x|{x.to_string()}).collect()),
            encoding: encoding.to_string(),
            channel: "cmdline".to_string()
        }
    }
    pub fn cmd_to_arr<'a> (&'a self) -> Vec<&'a str> {
        match &self.cmd {
            CmdDefinition::Splitted(x) => {
                x.iter().map(|x|{x.as_str()}).collect()
            }
            CmdDefinition::ToSplit(c) => {
                c.split(&[' ','\t'][..]).collect()

            }
        }
    }
    
    pub fn cmd_to_arr_replace<'a>(&'a self, placeholder: &'a str, value: &'a str) -> Vec<&'a str> {
        match &self.cmd {
            CmdDefinition::Splitted(x) => {
                x.iter().map(|x|{
                    if x == placeholder {
                        value
                    } else {
                        x.as_str()
                    }
                }).collect()
            }
            CmdDefinition::ToSplit(c) => {
                c.split(&[' ','\t'][..])
                .map(|x|{
                    if x == placeholder {
                        value
                    } else {
                        x
                    }
                })
                .collect()
            }
        }
    }
}