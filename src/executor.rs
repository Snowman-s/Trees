mod predefined;

use crate::structs::{Block, BlockError, ExecuteEnv, Includer, Literal};
use std::process::Command;

use predefined::predefined_procs;

pub fn execute(tree: Block, includer: Includer) -> Result<Literal, BlockError> {
  execute_with_mock(
    tree,
    Box::new(|| {
      let mut str = String::new();
      std::io::stdin().read_line(&mut str).unwrap();
      str.trim().to_string()
    }),
    Box::new(|msg| print!("{}", msg)),
    Box::new(|cmd, args| {
      let acutual_cmd = format!("{} {}", cmd, args.join(" "));
      if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", &acutual_cmd]).output()
      } else {
        Command::new("sh").arg("-c").arg(acutual_cmd).output()
      }
      .map_err(|err| err.to_string())
      .and_then(|out| String::from_utf8(out.stdout).map_err(|e| e.to_string()))
    }),
    includer,
  )
}

pub fn execute_with_mock(
  tree: Block,
  input_stream: Box<dyn FnMut() -> String>,
  out_stream: Box<dyn FnMut(String)>,
  cmd_executor: Box<dyn FnMut(String, Vec<String>) -> Result<String, String>>,
  includer: Includer,
) -> Result<Literal, BlockError> {
  let procs = predefined_procs();
  let mut exec_env = ExecuteEnv::new(procs, input_stream, out_stream, cmd_executor, includer);

  exec_env.new_scope();
  let result = tree.execute(&mut exec_env);
  exec_env.back_scope();

  result
}

#[cfg(test)]
mod tests {
  use crate::structs::{Block, Literal};

  use super::execute_with_mock;

  macro_rules! b {
    ($name:expr) => {
      Box::new(Block {
        proc_name: $name.to_owned(),
        args: vec![],
        quote: false,
      })
    };
    ($name:expr, $args:expr) => {
      Box::new(Block {
        proc_name: $name.to_owned(),
        args: $args.into_iter().map(|a| (false, a)).collect(),
        quote: false,
      })
    };
  }

  macro_rules! bq {
    ($name:expr) => {
      Box::new(Block {
        proc_name: $name.to_owned(),
        args: vec![],
        quote: true,
      })
    };
    ($name:expr, $args:expr) => {
      Box::new(Block {
        proc_name: $name.to_owned(),
        args: $args.into_iter().map(|a| (false, a)).collect(),
        quote: true,
      })
    };
  }

  macro_rules! str {
    ($str:expr) => {
      format!("\"{}\"", $str)
    };
  }

  fn execute(tree: Block) -> Result<Literal, String> {
    execute_with_mock(
      tree,
      Box::new(|| panic!()),
      Box::new(|_| panic!()),
      Box::new(|_, _| panic!()),
      Box::new(|_| panic!()),
    )
    .map_err(|err| err.msg)
  }

  #[test]
  fn simple_summing() {
    let result = execute(*b!("+", vec![b!("3"), b!("4")]));

    assert_eq!(result, Ok(Literal::Int(7)))
  }
  #[test]
  fn too_much_args() {
    let result = execute(*b!("+", vec![b!("3"), b!("4"), b!("5")]));

    assert!(result.is_err())
  }

  #[test]
  fn fizzbuzz() {
    let result = execute(*b!(
      "seq",
      vec![
        b!("defset", vec![b!(str!("out")), b!(str!(""))]),
        b!(
          "for",
          vec![
            b!("15"),
            b!(str!("i")),
            bq!(
              "set",
              vec![
                b!(str!("out")),
                b!(
                  "strcat",
                  vec![
                    b!("out"),
                    b!(
                      "if0",
                      vec![
                        b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("15")]),
                        b!(str!("FizzBuzz")),
                        b!(
                          "if0",
                          vec![
                            b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("3")]),
                            b!(str!("Fizz")),
                            b!(
                              "if0",
                              vec![
                                b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("5")]),
                                b!(str!("Buzz")),
                                b!("to str", vec![b!("+", vec![b!("i"), b!("1")])])
                              ]
                            )
                          ]
                        )
                      ]
                    )
                  ]
                )
              ]
            ),
          ]
        ),
        b!("out")
      ]
    ));

    assert_eq!(
      result,
      Ok(Literal::String(
        "12Fizz4BuzzFizz78FizzBuzz11Fizz1314FizzBuzz".to_string()
      ))
    )
  }

  #[test]
  fn fizzbuzz2() {
    let result = execute(*b!(
      "seq",
      vec![
        b!("defset", vec![b!(str!("out")), b!(str!(""))]),
        b!(
          "for",
          vec![
            b!("15"),
            b!(str!("i")),
            bq!(
              "seq",
              vec![
                b!(
                  "defset",
                  vec![
                    b!(str!("tmp")),
                    b!(
                      "strcat",
                      vec![
                        b!(
                          "ifn0",
                          vec![
                            b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("3")]),
                            b!(str!("")),
                            b!(str!("Fizz"))
                          ]
                        ),
                        b!(
                          "ifn0",
                          vec![
                            b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("5")]),
                            b!(str!("")),
                            b!(str!("Buzz"))
                          ]
                        )
                      ]
                    )
                  ]
                ),
                b!(
                  "set",
                  vec![
                    b!(str!("tmp")),
                    b!(
                      "if",
                      vec![
                        b!("=", vec![b!("tmp"), b!(str!(""))]),
                        b!("to str", vec![b!("+", vec![b!("i"), b!("1")])]),
                        b!("tmp")
                      ]
                    )
                  ]
                ),
                b!("set", vec![b!(str!("out")), b!("strcat", vec![b!("out"), b!("tmp")])]),
              ]
            )
          ]
        ),
        b!("out")
      ]
    ));

    assert_eq!(
      result,
      Ok(Literal::String(
        "12Fizz4BuzzFizz78FizzBuzz11Fizz1314FizzBuzz".to_string()
      ))
    )
  }

  #[test]
  fn compare_smaller() {
    let result = execute(*b!("ifn0", vec![b!("<", vec![b!("3"), b!("5")]), b!("1"), b!("0")]));

    assert_eq!(result, Ok(Literal::Int(1)))
  }

  #[test]
  fn compare_greater() {
    let result = execute(*b!("if", vec![b!(">", vec![b!("3"), b!("5")]), b!("1"), b!("0")]));

    assert_eq!(result, Ok(Literal::Int(0)))
  }

  #[test]
  fn compare_smaller_equal() {
    let result = execute(*b!("if", vec![b!("<=", vec![b!("3"), b!("3")]), b!("1"), b!("0")]));

    assert_eq!(result, Ok(Literal::Int(1)))
  }

  #[test]
  fn compare_greater_equal() {
    let result = execute(*b!("if", vec![b!("<=", vec![b!("5"), b!("5")]), b!("1"), b!("0")]));

    assert_eq!(result, Ok(Literal::Int(1)))
  }

  #[test]
  fn bool_and() {
    let result = execute(*b!("and", vec![b!("true"), b!("true")]));

    assert_eq!(result, Ok(Literal::Boolean(true)))
  }

  #[test]
  fn bool_or() {
    let result = execute(*b!("or", vec![b!("true"), b!("false")]));

    assert_eq!(result, Ok(Literal::Boolean(true)))
  }

  #[test]
  fn bool_xor() {
    let result = execute(*b!("xor", vec![b!("true"), b!("false")]));

    assert_eq!(result, Ok(Literal::Boolean(true)))
  }

  #[test]
  fn cannot_refer_inside() {
    let result = execute(*b!(
      "seq",
      vec![b!("seq", vec![b!("defset", vec![b!(str!("out")), b!("3")])]), b!("out")]
    ));

    assert!(result.is_err());
  }

  #[test]
  fn split_string() {
    let result = execute(*b!("split str", vec![b!(str!("abc def ghi")), b!(str!(" "))]));

    assert_eq!(
      result,
      Ok(Literal::List(vec![
        Literal::String("abc".to_string()),
        Literal::String("def".to_string()),
        Literal::String("ghi".to_string())
      ]))
    )
  }

  #[test]
  fn split_string_per_char() {
    let result = execute(*b!("split str", vec![b!(str!("abc")), b!(str!(""))]));

    assert_eq!(
      result,
      Ok(Literal::List(vec![
        Literal::String("a".to_string()),
        Literal::String("b".to_string()),
        Literal::String("c".to_string())
      ]))
    )
  }

  #[test]
  fn simple_export() {
    let result = execute(*b!(
      "seq",
      vec![
        b!(
          "seq",
          vec![
            b!("defset", vec![b!(str!("out")), b!("3")]),
            b!("export", vec![b!(str!("out"))])
          ]
        ),
        b!("out")
      ]
    ));

    assert_eq!(result, Ok(Literal::Int(3)))
  }
}
