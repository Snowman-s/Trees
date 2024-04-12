use super::{exec_env::ProcBind, BlockError, BlockErrorTree, BlockResult, ExecuteEnv};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Literal {
  Int(i64),
  String(String),
  Boolean(bool),
  Block(BlockLiteral),
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
pub struct BlockLiteral {
  pub proc_name: String,
  pub bind: Option<Box<ProcBind>>,
  pub args: Vec<(bool, Box<BlockLiteral>)>,
  pub quote: bool,
}

impl BlockLiteral {
  pub fn execute(&self, exec_env: &mut ExecuteEnv) -> Result<Literal, BlockError> {
    exec_env.new_scope();
    let result = self.execute_without_scope(exec_env)?;
    exec_env.back_scope();

    Ok(result)
  }

  pub fn execute_without_scope(&self, exec_env: &mut ExecuteEnv) -> Result<Literal, BlockError> {
    if self.quote {
      let mut cloned = self.clone();
      cloned.quote = false;
      Ok(Literal::Block(cloned))
    } else {
      let mut pure_exec_args: Vec<Literal> = vec![];
      for (expand, arg) in &self.args {
        let result = match arg.execute(exec_env) {
          Ok(res) => res,
          Err(err) => return Err(self.create_inherite_error(err, pure_exec_args)),
        };

        if *expand {
          if let Literal::List(_) = result {
          } else {
            return Err(self.create_error(
              None,
              format!("\"@\" needs the arg is a list literal. (Got {})", result.to_string()),
              pure_exec_args,
            ));
          };
        }
        pure_exec_args.push(result);
      }

      let expanded_args = pure_exec_args
        .iter()
        .enumerate()
        .flat_map(|(i, arg)| {
          let arg = arg.clone();
          if self.args[i].0 {
            let Literal::List(list) = arg else { unreachable!() };
            list
          } else {
            vec![arg]
          }
        })
        .collect();

      if let Some(bind) = &self.bind {
        exec_env.execute_procedure_with_bind(&self.proc_name, &expanded_args, *bind.clone())
      } else {
        exec_env.execute_procedure(&self.proc_name, &expanded_args)
      }
      .map_err(|proc_error| match proc_error {
        super::ProcedureError::CausedByBlockExec(block_error) => {
          let new_msg = block_error.msg.clone();
          self.create_error(Some(block_error), new_msg, pure_exec_args)
        }
        super::ProcedureError::OtherError(msg) => self.create_error(None, msg, pure_exec_args),
      })
    }
  }

  fn create_inherite_error(&self, mut err: BlockError, pure_exec_args: Vec<Literal>) -> BlockError {
    err.root.expand = self.args[self.args.len() - 1].0;

    let mut children = vec![];
    for (i, result) in pure_exec_args.iter().enumerate() {
      let (expand, block) = &self.args[i];

      children.push(BlockErrorTree {
        result: BlockResult::Success(result.clone()),
        children: vec![],
        expand: *expand,
        proc_name: block.proc_name.to_string(),
      });
    }
    children.push(err.root);
    for i in pure_exec_args.len() + 1..self.args.len() {
      let (expand, block) = &self.args[i];

      children.push(BlockErrorTree {
        result: BlockResult::Unreached,
        children: vec![],
        expand: *expand,
        proc_name: block.proc_name.to_string(),
      });
    }

    BlockError {
      root: BlockErrorTree {
        result: BlockResult::Error,
        children,
        expand: false,
        proc_name: self.proc_name.clone(),
      },
      caused_by: err.caused_by,
      msg: err.msg,
    }
  }

  fn create_error(&self, caused_by: Option<Box<BlockError>>, msg: String, pure_exec_args: Vec<Literal>) -> BlockError {
    let mut children = vec![];
    for (i, (expand, block)) in self.args.iter().cloned().enumerate() {
      let proc_name = block.proc_name;
      children.push(BlockErrorTree {
        result: match pure_exec_args.get(i).cloned() {
          Some(arg) => BlockResult::Success(arg),
          None => BlockResult::Unreached,
        },
        children: vec![],
        expand,
        proc_name,
      })
    }
    BlockError {
      root: BlockErrorTree {
        result: BlockResult::Error,
        children,
        expand: false,
        proc_name: self.proc_name.clone(),
      },
      caused_by,
      msg,
    }
  }
}
