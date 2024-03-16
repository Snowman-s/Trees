use compile::compile;
use excutor::execute;

mod compile;
mod excutor;
mod structs;

fn main() {
  let block = compile(vec![]).unwrap();
  let result = execute(block).unwrap();

  print!("{}", result.to_string());
}
