use regex::Regex;
use std::{collections::HashMap, sync::OnceLock};

#[derive(PartialEq, Eq, Debug)]
pub enum Literal {
    Int(u64),
    String(String),
    Void,
}

impl ToString for Literal {
    fn to_string(&self) -> String {
        match self {
            Literal::Int(i) => i.to_string(),
            Literal::String(s) => format!("\"{}\"", s),
            Literal::Void => "<Void>".to_string(),
        }
    }
}

pub struct Block {
    pub proc_name: String,
    pub args: Vec<Box<Block>>,
}

pub type Behavior =
    fn(exec_env: &mut ExecuteEnv, args: &Vec<Box<Block>>) -> Result<Literal, String>;

pub struct ExecuteEnv {
    procs: HashMap<String, Behavior>,
}

fn to_int(str: &String) -> Option<u64> {
    static REGEX: OnceLock<regex::Regex> = OnceLock::<Regex>::new();
    let regex = REGEX.get_or_init(|| Regex::new(r"^[0-9]+$").unwrap());
    if regex.is_match(str) {
        u64::from_str_radix(str, 10).ok()
    } else {
        None
    }
}

impl ExecuteEnv {
    pub fn new(procs: HashMap<String, Behavior>) -> ExecuteEnv {
        ExecuteEnv { procs }
    }

    pub fn execute(&mut self, name: &String, args: &Vec<Box<Block>>) -> Result<Literal, String> {
        if let Some(proc) = self.procs.get(name) {
            proc(self, args)
        } else if name.starts_with("\"") && name.ends_with("\"") {
            Ok(Literal::String(name[1..(name.len() - 1)].to_string()))
        } else if let Some(int) = to_int(name) {
            Ok(Literal::Int(int))
        } else if name == "" {
            Ok(Literal::Void)
        } else {
            panic!("Undefind Proc Name {}", name);
        }
    }
}

impl Block {
    pub fn execute(&self, exec_env: &mut ExecuteEnv) -> Result<Literal, String> {
        exec_env.execute(&self.proc_name, &self.args)
    }
}
