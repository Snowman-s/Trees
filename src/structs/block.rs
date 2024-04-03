use std::collections::HashMap;

use super::{ExecuteEnv, Literal, ProcedureOrVar};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Block {
  pub proc_name: String,
  pub args: Vec<(bool, Box<Block>)>,
  pub quote: bool,
}

impl Block {
  pub fn execute(&self, exec_env: &mut ExecuteEnv) -> Result<Literal, String> {
    exec_env.new_scope();
    let result = self.execute_without_scope(exec_env)?;
    exec_env.back_scope();

    Ok(result)
  }

  pub fn execute_without_scope(&self, exec_env: &mut ExecuteEnv) -> Result<Literal, String> {
    let result = if self.quote {
      let mut cloned = self.clone();
      cloned.quote = false;
      Ok(Literal::Block(cloned))
    } else {
      exec_env.execute_procedure(&self.proc_name, &self.args)
    }?;

    Ok(result)
  }
}
