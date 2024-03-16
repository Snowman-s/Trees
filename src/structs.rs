use regex::Regex;
use std::{collections::HashMap, sync::OnceLock};

#[derive(PartialEq, Eq, Debug, Clone)]
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

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Block {
  pub proc_name: String,
  pub args: Vec<Box<Block>>,
}

pub type Behavior = fn(exec_env: &mut ExecuteEnv, args: &Vec<Box<Block>>) -> Result<Literal, String>;

pub enum BehaviorOrVar {
  Behavior(Behavior),
  Var(Literal),
}

pub struct ExecuteEnv {
  namespace: HashMap<String, BehaviorOrVar>,
  out_stream: Box<dyn FnMut(String)>,
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
  pub fn new(namespace: HashMap<String, BehaviorOrVar>, out_stream: Box<dyn FnMut(String)>) -> ExecuteEnv {
    ExecuteEnv { namespace, out_stream }
  }

  pub fn execute(&mut self, name: &String, args: &Vec<Box<Block>>) -> Result<Literal, String> {
    if let Some(behavior_or_var) = self.namespace.get(name) {
      match behavior_or_var {
        BehaviorOrVar::Behavior(be) => be(self, args),
        BehaviorOrVar::Var(var) => Ok(var.clone()),
      }
    } else if name.starts_with("\"") && name.ends_with("\"") {
      Ok(Literal::String(name[1..(name.len() - 1)].to_string()))
    } else if let Some(int) = to_int(name) {
      Ok(Literal::Int(int))
    } else if name == "" {
      Ok(Literal::Void)
    } else {
      panic!("Undefined Proc Name {}", name);
    }
  }

  pub fn get_var(&mut self, name: &String) -> Option<Literal> {
    if let Some(BehaviorOrVar::Var(value)) = self.namespace.get(name) {
      Some(value.clone())
    } else {
      None
    }
  }

  pub fn set_var(&mut self, name: &String, value: &Literal) {
    self.namespace.insert(name.to_string(), BehaviorOrVar::Var(value.clone()));
  }

  pub fn print(&mut self, msg: String) {
    (self.out_stream)(msg);
  }
}

impl Block {
  pub fn execute(&self, exec_env: &mut ExecuteEnv) -> Result<Literal, String> {
    exec_env.execute(&self.proc_name, &self.args)
  }
}
