use std::collections::HashMap;

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
    pub fn cmd_to_arr_replacements(&'a self, placeholders: HashMap<&'a str,&'a str> ) -> Vec<String> {
        match &self {
            CmdDefinition::Splitted(x) => {
                x.iter().map(|x|{
                    text_placeholder::Template::new(x).fill_with_hashmap(&placeholders)
                }).collect()
            }
            CmdDefinition::ToSplit(c) => {
                c.split(&[' ','\t'][..])
                .map(|x|{
                    text_placeholder::Template::new(x).fill_with_hashmap(&placeholders)
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
    pub timeout: Option<u32>,
    pub encoding: String,
    pub channel: String
}

impl ProcEnv {
    pub fn new(wd: &str, env: Vec<String>, cmd: &str, encoding: &str) -> Self {
        ProcEnv {
            wd: wd.to_string(),
            env,
            cmd: CmdDefinition::from(cmd),
            encoding: encoding.to_string(),
            timeout: Some(1000),
            channel: "cmdline".to_string()
        }
    }
    pub fn new_v(wd: &str, env: Vec<&str>, cmd: &[&str], encoding: &str) -> Self 
    {
        ProcEnv {
            wd: wd.to_string(),
            env: env.iter().map(|x|{x.to_string()}).collect(),
            //cmd: CmdDefinition::Splitted(cmd.iter().map(|x|{x.to_string()}).collect()),
            cmd: CmdDefinition::from(cmd.to_vec()),
            timeout: Some(1000),
            encoding: encoding.to_string(),
            channel: "cmdline".to_string()
        }
    }
    
    pub fn cmd_to_arr_replacements<'a>(&'a self, placeholders: HashMap<&'a str,&'a str>) -> Vec<String> {
        self.cmd.cmd_to_arr_replacements(placeholders)
    }
    
    pub fn get_env(&self) -> Vec<(&str,&str)> {
        self.env.iter().map(|x|{
            let p = x.find("=").unwrap_or(0);
            let (a,_) = x.split_at(p);
            let (_,c) = x.split_at(p+1);
            (a,c)
        }).collect()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_cmd() {
        let cmd = CmdDefinition::from(vec!["bin/sh","echo hello world {{string}}"]);
        let mut placeholders = HashMap::new();
        placeholders.insert("string","hello");
        let v = cmd.cmd_to_arr_replacements(placeholders);
        assert_eq!("echo hello world hello", &v[1]);
        let mut c = std::process::Command::new(&v[0]);
        c.arg(&v[1]);
    }

    #[test]
    fn proc_env() {
        let penv = ProcEnv::new("",vec![],"cmd {{jsonpayload}}","");
        let mut placeholders = HashMap::new();
        placeholders.insert("jsonpayload","123");
        let v = penv.cmd_to_arr_replacements(placeholders);
        let str = v.join(" ");
        assert_eq!("cmd 123",&str);
    }
}