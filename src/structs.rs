use regex::Regex;
use std::{collections::HashMap, str::FromStr, sync::OnceLock};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Literal {
  Int(i64),
  String(String),
  Boolean(bool),
  Block(Block),
  List(Vec<Literal>),
  Void,
}

impl ToString for Literal {
  fn to_string(&self) -> String {
    match self {
      Literal::Int(i) => i.to_string(),
      Literal::String(s) => s.clone(),
      Literal::Boolean(b) => b.to_string(),
      Literal::Block(b) => format!("Block {}", b.proc_name),
      Literal::List(list) => {
        format!(
          "[{}]",
          list
            .iter()
            .map(|l| match l {
              Literal::String(s) => format!("{s:?}"),
              _ => l.to_string(),
            })
            .collect::<Vec<String>>()
            .join(", ")
        )
      }
      Literal::Void => "<Void>".to_string(),
    }
  }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Block {
  pub proc_name: String,
  pub args: Vec<(bool, Box<Block>)>,
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
  paths: Vec<String>,
  input_stream: Box<dyn FnMut() -> String>,
  out_stream: Box<dyn FnMut(String)>,
  cmd_executor: Box<dyn FnMut(String, Vec<String>) -> Result<String, String>>,
  includer: Box<dyn FnMut(&Vec<String>) -> Result<Block, String>>,
}

fn to_int(str: &String) -> Option<i64> {
  static REGEX: OnceLock<regex::Regex> = OnceLock::<Regex>::new();
  let regex = REGEX.get_or_init(|| Regex::new(r"^[0-9]+$").unwrap());
  if regex.is_match(str) {
    i64::from_str_radix(str, 10).ok()
  } else {
    None
  }
}

fn to_bool(str: &String) -> Option<bool> {
  match str.parse::<bool>() {
    Ok(arg) => Some(arg),
    Err(_) => None,
  }
}

impl ExecuteEnv {
  pub fn new(
    namespace: HashMap<String, BehaviorOrVar>,
    input_stream: Box<dyn FnMut() -> String>,
    out_stream: Box<dyn FnMut(String)>,
    cmd_executor: Box<dyn FnMut(String, Vec<String>) -> Result<String, String>>,
    includer: Box<dyn FnMut(&Vec<String>) -> Result<Block, String>>,
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

  pub fn execute(&mut self, name: &String, args: &Vec<(bool, Box<Block>)>) -> Result<Literal, String> {
    self.new_scope();
    let mut exec_args = vec![];
    for (expand, arg) in args {
      let result = arg.execute(self)?;
      if *expand {
        let Literal::List(res_list) = result else {
          return Err(format!("\"@\" needs the arg is a list literal. (Got {})", result.to_string()));
        };
        exec_args.extend(res_list);
      } else {
        exec_args.push(result);
      }
    }

    let result = if let Some(behavior_or_var) = self.find_namespace(name) {
      let behavior_or_var = behavior_or_var.clone();
      match behavior_or_var {
        BehaviorOrVar::Behavior(be) => be(self, &exec_args),
        BehaviorOrVar::BlockBehavior(block) => {
          self.defset_args(exec_args);

          block.execute(self)
        }
        BehaviorOrVar::Var(var) => Ok(var.clone()),
      }
    } else if name.starts_with("\"") && name.ends_with("\"") {
      Ok(Literal::String(name[1..(name.len() - 1)].to_string()))
    } else if let Some(int) = to_int(name) {
      Ok(Literal::Int(int))
    } else if let Some(boolean) = to_bool(name) {
      Ok(Literal::Boolean(boolean))
    } else if name == "" {
      Ok(Literal::Void)
    } else {
      Err(format!("Undefined Proc Name {}", name))
    }?;

    self.back_scope();

    Ok(result)
  }

  pub fn get_var(&mut self, name: &String) -> Result<Literal, String> {
    if let Some(BehaviorOrVar::Var(value)) = self.find_namespace_mut(name) {
      Ok(value.clone())
    } else {
      Err(format!("Variable {} is not defined", name))
    }
  }

  pub fn defset_var(&mut self, name: &str, value: &Literal) {
    let target = (self.scopes.len() - 2).clone();
    self.scopes[target].namespace.insert(name.to_string(), BehaviorOrVar::Var(value.clone()));
  }

  pub fn set_var(&mut self, name: &String, value: &Literal) -> Result<(), String> {
    if let Some(scope) = self.find_scope_mut(name) {
      scope.namespace.insert(name.to_string(), BehaviorOrVar::Var(value.clone()));
      Ok(())
    } else {
      Err(format!("Variable {} is not defined", name))
    }
  }

  pub fn defset_args(&mut self, args: Vec<Literal>) {
    let target = &mut self.scopes.last_mut().unwrap().namespace;
    target.insert("$args".to_string(), BehaviorOrVar::Var(Literal::List(args.clone())));
    for (i, arg) in args.iter().enumerate() {
      target.insert(format!("${}", i), BehaviorOrVar::Var(arg.clone()));
    }
  }

  pub fn def_proc(&mut self, name: &String, block: &Block) {
    let behavior = BehaviorOrVar::BlockBehavior(block.clone());

    if let Some(scope) = self.find_scope_mut(name) {
      scope.namespace.insert(name.to_string(), behavior);
    } else {
      let target = (self.scopes.len() - 2).clone();
      self.scopes[target].namespace.insert(name.to_string(), behavior);
    };
  }

  pub fn export(&mut self, name: &String) -> Result<(), String> {
    if let Some(value) = self.find_namespace(name) {
      let value = value.clone();
      let cont_index = (self.scopes.len() - 3).clone();
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

  pub fn include(&mut self, path_str: String) -> Result<Literal, String> {
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
    let block = (self.includer)(&paths)?;

    // 実行
    self.paths.push(parent);
    let result = block.execute(self)?;
    self.paths.pop();

    Ok(result)
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
