use super::{Block, BlockError, Literal};
use regex::Regex;
use std::{collections::HashMap, sync::OnceLock};

pub type FnProcedure = fn(&mut ExecuteEnv, &Vec<Literal>) -> Result<Literal, ProcedureError>;

#[derive(Clone)]
pub enum ProcedureOrVar {
  FnProcedure(FnProcedure),
  BlockProcedure(Block),
  Var(Literal),
}

#[derive(Clone)]
struct ExecuteScope {
  namespace: HashMap<String, ProcedureOrVar>,
}

pub type Includer = Box<dyn FnMut(&Vec<String>) -> Result<Block, String>>;
pub struct ExecuteEnv {
  scopes: Vec<ExecuteScope>,
  paths: Vec<String>,
  input_stream: Box<dyn FnMut() -> String>,
  out_stream: Box<dyn FnMut(String)>,
  cmd_executor: Box<dyn FnMut(String, Vec<String>) -> Result<String, String>>,
  includer: Includer,
}

fn to_int(str: &str) -> Option<i64> {
  static REGEX: OnceLock<regex::Regex> = OnceLock::<Regex>::new();
  let regex = REGEX.get_or_init(|| Regex::new(r"^(\+|-)?[0-9]+$").unwrap());
  if regex.is_match(str) {
    str.parse::<i64>().ok()
  } else {
    None
  }
}

fn to_bool(str: &str) -> Option<bool> {
  match str.parse::<bool>() {
    Ok(arg) => Some(arg),
    Err(_) => None,
  }
}

impl ExecuteEnv {
  pub fn new(
    namespace: HashMap<String, ProcedureOrVar>,
    input_stream: Box<dyn FnMut() -> String>,
    out_stream: Box<dyn FnMut(String)>,
    cmd_executor: Box<dyn FnMut(String, Vec<String>) -> Result<String, String>>,
    includer: Includer,
  ) -> ExecuteEnv {
    ExecuteEnv {
      scopes: vec![ExecuteScope { namespace }],
      paths: vec![],
      input_stream,
      out_stream,
      cmd_executor,
      includer,
    }
  }

  pub fn new_scope(&mut self) {
    self.scopes.push(ExecuteScope {
      namespace: HashMap::new(),
    });
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

  fn find_namespace(&self, name: &String) -> Option<&ProcedureOrVar> {
    self.find_scope(name).and_then(|c| c.namespace.get(name))
  }
  fn find_namespace_mut(&mut self, name: &String) -> Option<&mut ProcedureOrVar> {
    self.find_scope_mut(name).and_then(|c| c.namespace.get_mut(name))
  }

  pub fn defset_args(&mut self, args: &Vec<Literal>) {
    let namespace = &mut self.scopes.last_mut().unwrap().namespace;
    namespace.insert("$args".to_string(), ProcedureOrVar::Var(Literal::List(args.clone())));
    for (i, arg) in args.iter().enumerate() {
      namespace.insert(format!("${}", i), ProcedureOrVar::Var(arg.clone()));
    }
  }

  pub fn execute_procedure(&mut self, name: &String, exec_args: &Vec<Literal>) -> Result<Literal, ProcedureError> {
    if let Some(behavior_or_var) = self.find_namespace(name) {
      let behavior_or_var = behavior_or_var.clone();
      match behavior_or_var {
        ProcedureOrVar::FnProcedure(be) => be(self, exec_args),
        ProcedureOrVar::BlockProcedure(block) => {
          self.defset_args(exec_args);
          block.execute_without_scope(self).map_err(|err| ProcedureError::CausedByBlockExec(Box::new(err)))
        }
        ProcedureOrVar::Var(var) => Ok(var.clone()),
      }
    } else if name.starts_with('\"') && name.ends_with('\"') {
      Ok(Literal::String(name[1..(name.len() - 1)].to_string()))
    } else if let Some(int) = to_int(name) {
      Ok(Literal::Int(int))
    } else if let Some(boolean) = to_bool(name) {
      Ok(Literal::Boolean(boolean))
    } else if name.is_empty() {
      Ok(Literal::Void)
    } else {
      Err(ProcedureError::OtherError(format!("Undefined Proc Name {}", name)))
    }
  }

  pub fn get_var(&mut self, name: &String) -> Result<Literal, ProcedureError> {
    if let Some(ProcedureOrVar::Var(value)) = self.find_namespace_mut(name) {
      Ok(value.clone())
    } else {
      Err(ProcedureError::OtherError(format!("Variable {} is not defined", name)))
    }
  }

  pub fn defset_var(&mut self, name: &str, value: &Literal) {
    let target = self.scopes.len() - 2;
    self.scopes[target].namespace.insert(name.to_string(), ProcedureOrVar::Var(value.clone()));
  }

  pub fn set_var(&mut self, name: &String, value: &Literal) -> Result<(), String> {
    if let Some(scope) = self.find_scope_mut(name) {
      scope.namespace.insert(name.to_string(), ProcedureOrVar::Var(value.clone()));
      Ok(())
    } else {
      Err(format!("Variable {} is not defined", name))
    }
  }

  pub fn def_proc(&mut self, name: &String, block: &Block) {
    let behavior = ProcedureOrVar::BlockProcedure(block.clone());

    if let Some(scope) = self.find_scope_mut(name) {
      scope.namespace.insert(name.to_string(), behavior);
    } else {
      let target = self.scopes.len() - 2;
      self.scopes[target].namespace.insert(name.to_string(), behavior);
    };
  }

  pub fn export(&mut self, name: &String) -> Result<(), String> {
    if let Some(value) = self.find_namespace(name) {
      let value = value.clone();
      let cont_index = self.scopes.len() - 3;
      if let Some(context) = self.scopes.get_mut(cont_index) {
        context.namespace.insert(name.clone(), value.clone());
      };
      Ok(())
    } else {
      Err(format!("Variable {} is not defined", name))
    }
  }

  pub fn read_line(&mut self) -> String {
    (self.input_stream)()
  }

  pub fn print(&mut self, msg: String) {
    (self.out_stream)(msg);
  }

  pub fn cmd(&mut self, cmd: String, args: Vec<String>) -> Result<String, String> {
    (self.cmd_executor)(cmd, args)
  }

  pub fn include(&mut self, path_str: String) -> Result<Literal, ProcedureError> {
    // 祖先抽出
    let parent = if let Some(index) = path_str.find('/') {
      let truncated = &path_str[..index];
      truncated.to_string()
    } else {
      "".to_owned()
    };

    // コンパイル
    let mut paths = self.paths.clone();
    paths.push(path_str);
    let block = (self.includer)(&paths).map_err(ProcedureError::OtherError)?;

    // 実行
    self.paths.push(parent);
    let result = block.execute_without_scope(self).map_err(|err| ProcedureError::CausedByBlockExec(Box::new(err)))?;
    self.paths.pop();

    Ok(result)
  }
}

#[derive(Debug)]
pub enum ProcedureError {
  CausedByBlockExec(Box<BlockError>),
  OtherError(String),
}

impl From<String> for ProcedureError {
  fn from(value: String) -> Self {
    ProcedureError::OtherError(value)
  }
}

impl From<BlockError> for ProcedureError {
  fn from(value: BlockError) -> Self {
    ProcedureError::CausedByBlockExec(Box::new(value))
  }
}
