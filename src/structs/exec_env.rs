use super::{literal::BlockLiteral, Block, BlockError, Literal};
use regex::Regex;
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::OnceLock};

pub type FnProcedure = fn(&mut ExecuteEnv, &Vec<Literal>) -> Result<Literal, ProcedureError>;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum ProcedureOrVar {
  FnProcedure(FnProcedure),
  BlockProcedure(BlockLiteral),
  Var(Literal),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ExecuteScopeBody {
  pub paths: Vec<String>,
  pub namespace: HashMap<String, ProcedureOrVar>,
}

pub type ExecuteScope = Rc<RefCell<ExecuteScopeBody>>;

pub type Includer = Box<dyn FnMut(&Vec<String>) -> Result<Block, String>>;
pub struct ExecuteEnv {
  scopes: Vec<Vec<ExecuteScope>>,
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
      scopes: vec![vec![Rc::new(RefCell::new(ExecuteScopeBody {
        paths: vec![],
        namespace,
      }))]],
      input_stream,
      out_stream,
      cmd_executor,
      includer,
    }
  }

  fn get_last_scopes(&self) -> &Vec<ExecuteScope> {
    self.scopes.last().unwrap()
  }
  fn get_last_scopes_mut(&mut self) -> &mut Vec<ExecuteScope> {
    self.scopes.last_mut().unwrap()
  }
  fn get_upper_scope(&mut self) -> ExecuteScope {
    let scopes = self.get_last_scopes_mut();
    scopes[scopes.len() - 2].clone()
  }
  fn get_upper2_scope(&mut self) -> Option<ExecuteScope> {
    let scopes = self.get_last_scopes_mut();
    scopes.get(scopes.len() - 3).cloned()
  }
  fn get_last_scope(&self) -> ExecuteScope {
    self.get_last_scopes().last().unwrap().clone()
  }

  pub fn new_scope(&mut self) {
    let paths = self.get_last_scope().borrow().paths.clone();

    self.get_last_scopes_mut().push(Rc::new(RefCell::new(ExecuteScopeBody {
      paths,
      namespace: HashMap::new(),
    })));
  }
  pub fn back_scope(&mut self) {
    if self.get_last_scopes_mut().len() <= 1 {
      panic!("Scopes were not enough.Please report the problem to developers.")
    }
    self.get_last_scopes_mut().pop();
  }
  pub fn reload_scope(&mut self, scope: ExecuteScope) {
    self.get_last_scopes_mut().push(scope);
  }
  pub fn freeze_scope(&mut self) -> ExecuteScope {
    if self.get_last_scopes_mut().len() <= 1 {
      panic!("Scopes were not enough.Please report the problem to developers.")
    }
    self.get_last_scopes_mut().pop().unwrap()
  }
  pub fn new_scopes(&mut self, scopes: Vec<ExecuteScope>) {
    self.scopes.push(scopes);
  }
  pub fn back_scopes(&mut self) {
    self.scopes.pop().unwrap();
  }

  fn find_scope(&self, name: &str) -> Option<ExecuteScope> {
    self.get_last_scopes().iter().rev().find(|scope| scope.borrow().namespace.contains_key(name)).cloned()
  }

  fn find_namespace(&self, name: &str) -> Option<ProcedureOrVar> {
    self.get_last_scopes().iter().rev().find_map(|scope| scope.borrow().namespace.get(name).cloned())
  }

  pub fn defset_args(&mut self, args: &Vec<Literal>) {
    let binding = self.get_last_scope();
    let namespace = &mut binding.borrow_mut().namespace;
    namespace.insert("$args".to_string(), ProcedureOrVar::Var(Literal::List(args.clone())));
    for (i, arg) in args.iter().enumerate() {
      namespace.insert(format!("${}", i), ProcedureOrVar::Var(arg.clone()));
    }
  }

  pub fn bind_name(&self, name: &str) -> Option<ProcBind> {
    if let Some(scope) = self.find_scope(name) {
      Some(ProcBind::Namespace(scope))
    } else {
      Some(ProcBind::Literal(if name.starts_with('\"') && name.ends_with('\"') {
        Literal::String(name[1..(name.len() - 1)].to_string())
      } else if let Some(int) = to_int(name) {
        Literal::Int(int)
      } else if let Some(boolean) = to_bool(name) {
        Literal::Boolean(boolean)
      } else if name.is_empty() {
        Literal::Void
      } else {
        return None;
      }))
    }
  }

  pub fn execute_procedure(&mut self, name: &str, exec_args: &Vec<Literal>) -> Result<Literal, ProcedureError> {
    self.execute_procedure_with_bind(
      name,
      exec_args,
      self.bind_name(name).ok_or(format!("Undefined Proc Name {}", name))?,
    )
  }

  pub fn execute_procedure_with_bind(
    &mut self,
    name: &str,
    exec_args: &Vec<Literal>,
    bind: ProcBind,
  ) -> Result<Literal, ProcedureError> {
    match bind {
      ProcBind::Namespace(namespace) => {
        if let Some(behavior_or_var) = namespace.borrow().namespace.get(name) {
          let behavior_or_var = behavior_or_var.clone();
          match behavior_or_var {
            ProcedureOrVar::FnProcedure(be) => be(self, exec_args),
            ProcedureOrVar::BlockProcedure(block) => block
              .execute_without_scope(self, |exec_env| exec_env.defset_args(exec_args))
              .map_err(|err| ProcedureError::CausedByBlockExec(Box::new(err))),
            ProcedureOrVar::Var(var) => Ok(var.clone()),
          }
        } else {
          // 変数が削除できない限り到達不可
          unreachable!()
        }
      }
      ProcBind::Literal(literal) => Ok(literal),
    }
  }

  pub fn get_var(&mut self, name: &String) -> Result<Literal, ProcedureError> {
    if let Some(ProcedureOrVar::Var(value)) = self.find_namespace(name) {
      Ok(value.clone())
    } else {
      Err(ProcedureError::OtherError(format!("Variable {} is not defined", name)))
    }
  }

  pub fn defset_var(&mut self, name: &str, value: &Literal) {
    self.get_upper_scope().borrow_mut().namespace.insert(name.to_string(), ProcedureOrVar::Var(value.clone()));
  }
  pub fn defset_var_into_last_scope(&mut self, name: &str, value: &Literal) {
    self.get_last_scope().borrow_mut().namespace.insert(name.to_string(), ProcedureOrVar::Var(value.clone()));
  }

  pub fn set_var(&mut self, name: &String, value: &Literal) -> Result<(), String> {
    if let Some(scope) = self.find_scope(name) {
      scope.borrow_mut().namespace.insert(name.to_string(), ProcedureOrVar::Var(value.clone()));
      Ok(())
    } else {
      Err(format!("Variable {} is not defined", name))
    }
  }

  pub fn def_proc(&mut self, name: &String, block: &BlockLiteral) {
    let behavior = ProcedureOrVar::BlockProcedure(block.clone());

    self.get_upper_scope().borrow_mut().namespace.insert(name.to_string(), behavior);
  }

  pub fn export(&mut self, name: &String) -> Result<(), String> {
    if let Some(value) = self.find_namespace(name) {
      let value = value.clone();
      if let Some(context) = self.get_upper2_scope() {
        context.borrow_mut().namespace.insert(name.clone(), value.clone());
      };
      Ok(())
    } else {
      Err(format!("Variable {} is not defined", name))
    }
  }

  pub fn reexport(&mut self) {
    let scope_len = self.scopes.len();
    for (key, proc_or_var) in self.get_last_scope().borrow().namespace.clone().iter() {
      self.get_upper_scope().borrow_mut().namespace.insert(key.clone(), proc_or_var.clone());
      if let Some(exp_scope) = self.get_upper2_scope() {
        exp_scope.borrow_mut().namespace.insert(key.clone(), proc_or_var.clone());
      }
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
    let parent = if let Some(index) = path_str.rfind('/') {
      let truncated = &path_str[..index];
      truncated.to_string()
    } else {
      "".to_owned()
    };

    // コンパイル
    let mut paths = self.get_last_scope().borrow().paths.clone();
    paths.push(path_str);
    let block = (self.includer)(&paths).map_err(ProcedureError::OtherError)?;

    // 実行
    let freezed = self.freeze_scope();
    self.new_scope();
    self.get_last_scope().borrow_mut().paths.push(parent);
    let result = block.execute_without_scope(self).map_err(|err| ProcedureError::CausedByBlockExec(Box::new(err)))?;
    self.back_scope();
    self.reload_scope(freezed);

    Ok(result)
  }

  pub fn make_closure(&mut self, block: Block) -> Result<BlockLiteral, String> {
    Ok(BlockLiteral {
      scopes: self.get_last_scopes_mut().clone(),
      block,
    })
  }

  pub fn get_scopes(&self) -> Vec<ExecuteScope> {
    self.get_last_scopes().clone()
  }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum ProcBind {
  Namespace(ExecuteScope),
  Literal(Literal),
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
