use super::{ExecuteEnv, Literal};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Block {
  pub proc_name: String,
  pub args: Vec<(bool, Box<Block>)>,
  pub quote: bool,
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
