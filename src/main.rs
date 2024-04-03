use std::{env, fs::File, io::Read, path::PathBuf, rc::Rc};

use compile::compile;
use executor::execute;
use structs::Block;

mod compile;
mod executor;
mod structs;

fn main() {
  let args: Vec<String> = env::args().collect();
  let code_file = &args[1];

  let path = Rc::new(env::current_dir().unwrap().join(code_file));
  let block = compile_file(path.to_path_buf()).unwrap();
  execute(
    block,
    Box::new(move |name| compile_file(name.iter().fold(path.parent().unwrap().to_path_buf(), |a, b| a.join(b)))),
  )
  .unwrap();
}

fn compile_file(file_path: PathBuf) -> Result<Block, String> {
  let mut codes = File::open(&file_path).map_err(|err| format!("failed to read {:?}: {}", &file_path.to_str(), err))?;
  let mut buf: String = String::new();
  codes.read_to_string(&mut buf).map_err(|err| format!("failed to read {:?}: {}", &file_path.to_str(), err))?;

  compile(buf.split('\n').map(|t| t.to_owned()).collect())
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use crate::{compile, executor::execute_with_mock, structs::Literal};

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
    .and_then(|b| execute_with_mock(b, Box::new(|| panic!()), out_stream, cmd_executor, Box::new(|_| panic!())));

    assert_eq!(Ok(Literal::Void), result);
    assert_eq!("7", *out_ref.borrow());
  }

  fn exec_file(code: &str) -> (Result<Literal, String>, String, Vec<(String, Vec<String>)>) {
    let out = Rc::new(RefCell::new("".to_owned()));
    let out_ref = out.clone();
    let out_stream = Box::new(move |msg| {
      (*out.borrow_mut()).extend([msg]);
    });
    let cmd_log: Rc<RefCell<Vec<(String, Vec<String>)>>> = Rc::new(RefCell::new(vec![]));
    let cmd_log_ref = cmd_log.clone();
    let cmd_executor = Box::new(move |cmd, args| {
      (*cmd_log.borrow_mut()).push((cmd, args));
      Ok("".to_string())
    });

    let code_lines: Vec<String> = code.split('\n').map(|c| c.to_owned()).collect();
    let result = compile(code_lines).and_then(|b| execute_with_mock(b, Box::new(|| panic!()), out_stream, cmd_executor, Box::new(|_| panic!())));

    let out = out_ref.borrow().clone();
    let cmd = cmd_log_ref.borrow().clone();
    (result, out, cmd)
  }

  #[test]
  fn minus() {
    let (r, o, _) = exec_file(include_str!("test/minus.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "-1\n");
  }

  #[test]
  fn println() {
    let (r, o, _) = exec_file(include_str!("test/println.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "7\n");
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

  #[test]
  fn lists() {
    let (r, o, _) = exec_file(include_str!("test/lists.tr"));
    assert_eq!(r, Ok(Literal::Int(7)));
    assert_eq!(o, "");
  }

  #[test]
  fn string_bytes() {
    let (r, o, _) = exec_file(include_str!("test/string_bytes.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "AAA\n");
  }

  #[test]
  fn recursion() {
    let (r, o, _) = exec_file(include_str!("test/recursion.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "6\n");
  }

  #[test]
  fn tr_while() {
    let (r, o, _) = exec_file(include_str!("test/tr_while.tr"));
    assert_eq!(r, Ok(Literal::Void));
    assert_eq!(o, "012");
  }
}
