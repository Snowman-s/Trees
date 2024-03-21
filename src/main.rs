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

  use crate::{compile, excutor::execute_with_mock, structs::Literal};

  #[test]
  fn a_plus_b() {
    let out = Rc::new(RefCell::new("".to_owned()));
    let out_ref = out.clone();
    let out_stream = Box::new(move |msg| {
      *out.borrow_mut() = msg;
    });
    let cmd_executor = Box::new(|_, _| panic!());

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
    .and_then(|b| execute_with_mock(b, out_stream, cmd_executor));

    assert_eq!(Ok(Literal::Void), result);
    assert_eq!("7", *out_ref.borrow());
  }

  fn exec_file(code: &str) -> (Result<Literal, String>, String, Vec<(String, Vec<String>)>) {
    let out = Rc::new(RefCell::new("".to_owned()));
    let out_ref = out.clone();
    let out_stream = Box::new(move |msg| {
      *out.borrow_mut() = msg;
    });
    let cmd_log: Rc<RefCell<Vec<(String, Vec<String>)>>> = Rc::new(RefCell::new(vec![]));
    let cmd_log_ref = cmd_log.clone();
    let cmd_executor = Box::new(move |cmd, args| {
      (*cmd_log.borrow_mut()).push((cmd, args));
      return Ok("".to_string());
    });

    let code_lines: Vec<String> = code.split("\n").map(|c| c.to_owned()).collect();
    let result = compile(code_lines).and_then(|b| execute_with_mock(b, out_stream, cmd_executor));

    let out = out_ref.borrow().clone();
    let cmd = cmd_log_ref.borrow().clone();
    (result, out, cmd)
  }

  #[test]
  fn fizzbuzz() {
    let (r, o, _) = exec_file(include_str!("test/fizzbuzz.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "12Fizz4BuzzFizz78FizzBuzz11Fizz1314FizzBuzz");
  }

  #[test]
  fn defproc() {
    let (r, o, _) = exec_file(include_str!("test/defproc.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "6");
  }

  #[test]
  fn modules() {
    let (r, o, _) = exec_file(include_str!("test/modules.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "6");
  }

  #[test]
  fn modules_err() {
    let (r, o, _) = exec_file(include_str!("test/modules_err.tr"));
    assert!(r.is_err());
    assert_eq!(o, "");
  }

  #[test]
  fn substance() {
    let (r, o, _) = exec_file(include_str!("test/substance.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "6");
  }

  #[test]
  fn cmd() {
    let (r, o, cmd) = exec_file(include_str!("test/cmd.tr"));
    assert_eq!(r, Ok(Literal::String("".to_string())));
    assert_eq!(o, "");
    assert_eq!(cmd, vec![("echo".to_string(), vec!["out".to_string()])]);
  }
}
