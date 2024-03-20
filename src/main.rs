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

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use crate::{compile, excutor::execute_with_out_stream, structs::Literal};

  #[test]
  fn a_plus_b() {
    let out = Rc::new(RefCell::new("".to_owned()));
    let out_ref = out.clone();
    let out_stream = Box::new(move |msg| {
      *out.borrow_mut() = msg;
    });

    let result = compile(vec![
      "        ┌─────┐      ".to_owned(),
      "        │print│      ".to_owned(),
      "        └───┬─┘      ".to_owned(),
      "        ┌───┴─┐      ".to_owned(),
      "    ┌───┤  +  ├──┐   ".to_owned(),
      "    │   └─────┘  │   ".to_owned(),
      "┌───┴─┐      ┌───┴─┐ ".to_owned(),
      "│  3  │      │  4  │ ".to_owned(),
      "└─────┘      └─────┘ ".to_owned(),
    ])
    .and_then(|b| execute_with_out_stream(b, out_stream));

    assert_eq!(Ok(Literal::Void), result);
    assert_eq!("7", *out_ref.borrow());
  }

  fn exec_file(code: &str) -> (Result<Literal, String>, String) {
    let out = Rc::new(RefCell::new("".to_owned()));
    let out_ref = out.clone();
    let out_stream = Box::new(move |msg| {
      *out.borrow_mut() = msg;
    });

    let code_lines: Vec<String> = code.split("\n").map(|c| c.to_owned()).collect();
    let result = compile(code_lines).and_then(|b| execute_with_out_stream(b, out_stream));

    let c = out_ref.borrow().clone();
    (result, c)
  }

  #[test]
  fn fizzbuzz() {
    let (r, o) = exec_file(include_str!("test/fizzbuzz.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "12Fizz4BuzzFizz78FizzBuzz11Fizz1314FizzBuzz");
  }

  #[test]
  fn defproc() {
    let (r, o) = exec_file(include_str!("test/defproc.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "6");
  }

  #[test]
  fn modules() {
    let (r, o) = exec_file(include_str!("test/modules.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "6");
  }

  #[test]
  fn modules_err() {
    let (r, o) = exec_file(include_str!("test/modules_err.tr"));
    assert!(r.is_err());
    assert_eq!(o, "");
  }

  #[test]
  fn substance() {
    let (r, o) = exec_file(include_str!("test/substance.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "6");
  }
}
