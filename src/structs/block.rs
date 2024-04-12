use super::{ExecuteEnv, Literal};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Block {
  pub proc_name: String,
  pub args: Vec<(bool, Box<Block>)>,
  pub quote: bool,
}

impl Block {
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
      Ok(Literal::Block(
        exec_env.block_to_literal(cloned).map_err(|msg| self.create_error(None, msg, vec![]))?,
      ))
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
      exec_env.execute_procedure(&self.proc_name, &expanded_args).map_err(|proc_error| match proc_error {
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

#[derive(Debug)]
pub enum BlockResult {
  Success(Literal),
  Error,
  Unreached,
}

#[derive(Debug)]
pub struct BlockErrorTree {
  pub result: BlockResult,
  pub expand: bool,
  pub children: Vec<BlockErrorTree>,
  pub proc_name: String,
}

#[derive(Debug)]
pub struct BlockError {
  pub root: BlockErrorTree,
  pub caused_by: Option<Box<BlockError>>,
  pub msg: String,
}
