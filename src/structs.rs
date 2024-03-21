use regex::Regex;
use std::{collections::HashMap, sync::OnceLock};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Literal {
  Int(u64),
  String(String),
  Block(Block),
  Void,
}

impl ToString for Literal {
  fn to_string(&self) -> String {
    match self {
      Literal::Int(i) => i.to_string(),
      Literal::String(s) => s.clone(),
      Literal::Block(b) => format!("Block {}", b.proc_name),
      Literal::Void => "<Void>".to_string(),
    }
  }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Block {
  pub proc_name: String,
  pub args: Vec<Box<Block>>,
  pub quote: bool,
}

pub type Behavior = fn(&mut ExecuteEnv, &Vec<Literal>) -> Result<Literal, String>;

#[derive(Clone)]
pub enum BehaviorOrVar {
  Behavior(Behavior),
  BlockBehavior(Block),
  Var(Literal),
}

#[derive(Clone)]
struct ExecuteScope {
  namespace: HashMap<String, BehaviorOrVar>,
}

pub struct ExecuteEnv {
  scopes: Vec<ExecuteScope>,
  out_stream: Box<dyn FnMut(String)>,
  cmd_executor: Box<dyn FnMut(String, Vec<String>) -> Result<String, String>>,
  includer: Box<dyn FnMut(String) -> Result<Literal, String>>,
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
  pub fn new(
    namespace: HashMap<String, BehaviorOrVar>,
    out_stream: Box<dyn FnMut(String)>,
    cmd_executor: Box<dyn FnMut(String, Vec<String>) -> Result<String, String>>,
    includer: Box<dyn FnMut(String) -> Result<Literal, String>>,
  ) -> ExecuteEnv {
    ExecuteEnv {
      scopes: vec![ExecuteScope { namespace }],
      out_stream,
      cmd_executor,
      includer,
    }
  }

  pub fn new_scope(&mut self) {
    self.scopes.push(ExecuteScope { namespace: HashMap::new() });
  }
  pub fn back_scope(&mut self) {
    if self.scopes.len() <= 1 {
      panic!("Scopes were not enough.Please report the problem to developers.")
    }
    self.scopes.pop();
  }

  fn find_scope(&self, name: &String) -> Option<&ExecuteScope> {
    self.scopes.iter().rev().find(|scope| scope.namespace.contains_key(name))
  }
  fn find_scope_mut(&mut self, name: &String) -> Option<&mut ExecuteScope> {
    self.scopes.iter_mut().rev().find(|scope| scope.namespace.contains_key(name))
  }

  fn find_namespace(&self, name: &String) -> Option<&BehaviorOrVar> {
    self.find_scope(name).and_then(|c| c.namespace.get(name))
  }
  fn find_namespace_mut(&mut self, name: &String) -> Option<&mut BehaviorOrVar> {
    self.find_scope_mut(name).and_then(|c| c.namespace.get_mut(name))
  }

  pub fn execute(&mut self, name: &String, args: &Vec<Box<Block>>) -> Result<Literal, String> {
    self.new_scope();
    let mut exec_args = vec![];
    for arg in args {
      exec_args.push(arg.execute(self)?);
    }
    self.back_scope();

    if let Some(behavior_or_var) = self.find_namespace(name) {
      let behavior_or_var = behavior_or_var.clone();
      match behavior_or_var {
        BehaviorOrVar::Behavior(be) => be(self, &exec_args),
        BehaviorOrVar::BlockBehavior(block) => {
          for (i, arg) in exec_args.iter().enumerate() {
            self.defset_var(&format!("${}", i), arg);
          }

          block.execute(self)
        }
        BehaviorOrVar::Var(var) => Ok(var.clone()),
      }
    } else if name.starts_with("\"") && name.ends_with("\"") {
      Ok(Literal::String(name[1..(name.len() - 1)].to_string()))
    } else if let Some(int) = to_int(name) {
      Ok(Literal::Int(int))
    } else if name == "" {
      Ok(Literal::Void)
    } else {
      Err(format!("Undefined Proc Name {}", name))
    }
  }

  pub fn get_var(&mut self, name: &String) -> Result<Literal, String> {
    if let Some(BehaviorOrVar::Var(value)) = self.find_namespace_mut(name) {
      Ok(value.clone())
    } else {
      Err(format!("Variable {} is not defined", name))
    }
  }

  pub fn defset_var(&mut self, name: &String, value: &Literal) {
    self.scopes.last_mut().unwrap().namespace.insert(name.to_string(), BehaviorOrVar::Var(value.clone()));
  }

  pub fn set_var(&mut self, name: &String, value: &Literal) -> Result<(), String> {
    if let Some(scope) = self.find_scope_mut(name) {
      scope.namespace.insert(name.to_string(), BehaviorOrVar::Var(value.clone()));
      Ok(())
    } else {
      Err(format!("Variable {} is not defined", name))
    }
  }

  pub fn def_proc(&mut self, name: &String, block: &Block) {
    let behavior = BehaviorOrVar::BlockBehavior(block.clone());

    if let Some(scope) = self.find_scope_mut(name) {
      scope.namespace.insert(name.to_string(), behavior);
    } else {
      self.scopes.last_mut().unwrap().namespace.insert(name.to_string(), behavior);
    };
  }

  pub fn export(&mut self, name: &String) -> Result<(), String> {
    if let Some(value) = self.find_namespace(name) {
      let value = value.clone();
      let cont_index = (self.scopes.len() - 2).clone();
      if let Some(context) = self.scopes.get_mut(cont_index) {
        context.namespace.insert(name.clone(), value.clone());
      };
      Ok(())
    } else {
      Err(format!("Variable {} is not defined", name))
    }
  }

  pub fn print(&mut self, msg: String) {
    (self.out_stream)(msg);
  }

  pub fn cmd(&mut self, cmd: String, args: Vec<String>) -> Result<String, String> {
    (self.cmd_executor)(cmd, args)
  }

  pub fn include(&mut self, path: String) -> Result<Literal, String> {
    (self.includer)(path)
  }
}

impl Block {
  pub fn execute(&self, exec_env: &mut ExecuteEnv) -> Result<Literal, String> {
    if self.quote {
      let mut cloned = self.clone();
      cloned.quote = false;
      Ok(Literal::Block(cloned))
    } else {
      exec_env.execute(&self.proc_name, &self.args)
    }
  }
}
