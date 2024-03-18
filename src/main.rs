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

  #[test]
  fn fizzbuzz() {
    let out = Rc::new(RefCell::new("".to_owned()));
    let out_ref = out.clone();
    let out_stream = Box::new(move |msg| {
      *out.borrow_mut() = msg;
    });

    let code: Vec<String> = include_str!("test/fizzbuzz.tr").split("\n").map(|c| c.to_owned()).collect();
    let result = compile(code).and_then(|b| execute_with_out_stream(b, out_stream));

    assert_eq!(Ok(Literal::Void), result);
    assert_eq!("12Fizz4BuzzFizz78FizzBuzz11Fizz1314FizzBuzz", *out_ref.borrow());
  }

  #[test]
  fn defproc() {
    let out = Rc::new(RefCell::new("".to_owned()));
    let out_ref = out.clone();
    let out_stream = Box::new(move |msg| {
      *out.borrow_mut() = msg;
    });

    let code: Vec<String> = include_str!("test/defproc.tr").split("\n").map(|c| c.to_owned()).collect();
    let result = compile(code).and_then(|b| execute_with_out_stream(b, out_stream));

    assert_eq!(Ok(Literal::Void), result);
    assert_eq!("6", *out_ref.borrow());
  }
}
