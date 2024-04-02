use crate::Block;

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
