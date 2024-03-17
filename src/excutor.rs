use std::collections::HashMap;

use crate::structs::{BehaviorOrVar, Block, ExecuteEnv, Literal};

fn predefined_procs() -> HashMap<String, BehaviorOrVar> {
  let mut map: HashMap<String, BehaviorOrVar> = HashMap::new();

  macro_rules! add_map {
    ($name:expr, $callback:block; $($tail:ident:$type:tt),* ) => {{
      map.insert($name.to_string(), BehaviorOrVar::Behavior(|exec_env, args| {
        exec_env.new_scope();
        initialize_vars!($name, exec_env, args, $($tail:$type),*);
        exec_env.back_scope();
        $callback
      }))
    }};
    ($name:expr, $callback:block, $exec_env:ident, $args:ident; $($tail:ident:$type:tt),* ) => {{
      map.insert($name.to_string(), BehaviorOrVar::Behavior(|$exec_env, $args| {
        $exec_env.new_scope();
        initialize_vars!($name, $exec_env, $args, $($tail:$type),*);
        $exec_env.back_scope();
        $callback
      }))
    }};
  }

  macro_rules! initialize_vars {
    ($name: expr, $env:expr, $vec:expr,) => {};
    ($name: expr, $env:expr, $vec:expr, $($tail:ident:$type:tt),+) => {
      if $vec.len() != count_idents!($($tail)*) {
        return Err(format!("Procesure {}: Length of args must be {}.", $name, count_idents!($($tail)*)));
      }
      let mut iter = $vec.into_iter();
      $(
        let next = match iter.next() {
          Some(val) => val,
          None => panic!(),
        };
        declare!($name, $env, next, $tail:$type);
      )*
    };
  }

  macro_rules! declare {
    ($name: expr, $env:expr, $block:expr, $tail:ident:any) => {
      let res = $block.execute($env);
      let $tail = match res {
        Ok(r) => r,
        err => {
          return err;
        }
      };
    };
    ($name: expr, $env:expr, $block:expr, $tail:ident:int) => {
      let res = $block.execute($env);
      let $tail = match res {
        Ok(r) => {
          if let Literal::Int(t) = r {
            t
          } else {
            return Err(format!("Procesure {}: Executed result of arg {} must be int.", $name, r.to_string()));
          }
        }
        err => {
          return err;
        }
      };
    };
    ($name: expr, $env:expr, $block:expr, $tail:ident:str) => {
      let res = $block.execute($env);
      let $tail = match res {
        Ok(r) => {
          if let Literal::String(t) = r {
            t
          } else {
            return Err(format!("Procesure {}: Executed result of arg {} must be str.", $name, r.to_string()));
          }
        }
        err => {
          return err;
        }
      };
    };
    ($name: expr, $env:expr, $block:expr, $tail:ident:block) => {
      let res = $block.execute($env);
      let $tail = match res {
        Ok(r) => {
          if let Literal::Block(b) = r {
            b
          } else {
            return Err(format!("Procesure {}: Executed result of arg {} must be block.", $name, r.to_string()));
          }
        }
        err => {
          return err;
        }
      };
    };
  }

  macro_rules! count_idents {
    () => { 0 };
    ($_head:ident $($tail:tt)*) => { 1 + count_idents!($($tail)*) };
  }

  add_map!("+", {Ok(Literal::Int(a + b))}; a:int, b:int);
  add_map!("-", {Ok(Literal::Int(a - b))}; a:int, b:int);
  add_map!("*", {Ok(Literal::Int(a * b))}; a:int, b:int);
  add_map!("/", {Ok(Literal::Int(a / b))}; a:int, b:int);
  add_map!("%", {Ok(Literal::Int(a % b))}; a:int, b:int);
  add_map!("=", {Ok(Literal::Int(if a == b { 1 } else { 0 }))}; a:any, b:any);
  add_map!("strcat", {Ok(Literal::String(format!("{}{}", a, b)))}; a:str, b:str);
  add_map!("to_str", {Ok(Literal::String(a.to_string()))}; a:any);
  add_map!("get", {exec_env.get_var(&name)}, exec_env, _args; name:str);
  add_map!("set", {
    exec_env.set_var(&name, &from);
    Ok(Literal::Void)
  }, exec_env, _args; name:str, from:any);
  add_map!("print", {
    exec_env.print(a.to_string());
    Ok(Literal::Void)
  }, exec_env, args; a:any);
  add_map!("seq", {
    exec_env.new_scope();

    let mut exec_args: Vec<Block> = vec![];
    for (i, args) in args.iter().enumerate() {
      let r = args.execute(exec_env)?;

      if let Literal::Block(b) = r {
        exec_args.push(b);
      } else {
        return Err(format!("Procesure {}: Executed result of arg {} must be int.", i, r.to_string()));
      }
    }
    exec_env.back_scope();
    let mut result = Literal::Void;
    for arg in exec_args {
      result = arg.execute(exec_env)?;
    }
    Ok(result)
  }, exec_env, args;);
  add_map!("for", {
    for i in 0..times {
      exec_env.set_var(&var, &Literal::Int(i));
      child.execute(exec_env)?;
    }
    Ok(Literal::Void)
  }, exec_env, args; times:int, var:str, child:block);
  add_map!("if0", {
    if let Literal::Int(0) = cond {
      then.execute(exec_env)
    } else {
      els.execute(exec_env)
    }
  }, exec_env, args; cond:any, then:block, els:block );
  add_map!("ifn0", {
    if let Literal::Int(0) = cond {
      els.execute(exec_env)
    } else {
      then.execute(exec_env)
    }
  }, exec_env, args; cond:any, then:block, els:block );
  add_map!("export", {
    exec_env.export(&name)?;
    Ok(Literal::Void)
  }, exec_env, args; name:str );

  map
}

pub fn execute(tree: Block) -> Result<Literal, String> {
  execute_with_out_stream(tree, Box::new(|msg| print!("{}", msg)))
}

pub fn execute_with_out_stream(tree: Block, out_stream: Box<dyn FnMut(String)>) -> Result<Literal, String> {
  let procs = predefined_procs();
  let mut exec_env = ExecuteEnv::new(procs, out_stream);

  exec_env.new_scope();
  let result = tree.execute(&mut exec_env);
  exec_env.back_scope();

  result
}

#[cfg(test)]
mod tests {
  use crate::structs::{Block, Literal};

  use super::execute;

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
        args: $args,
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
        args: $args,
        quote: true,
      })
    };
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
        bq!("set", vec![b!("\"out\""), b!("\"\"")]),
        bq!(
          "for",
          vec![
            b!("15"),
            b!("\"i\""),
            bq!(
              "set",
              vec![
                b!("\"out\""),
                b!(
                  "strcat",
                  vec![
                    b!("out"),
                    b!(
                      "if0",
                      vec![
                        b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("15")]),
                        bq!("\"FizzBuzz\""),
                        bq!(
                          "if0",
                          vec![
                            b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("3")]),
                            bq!("\"Fizz\""),
                            bq!(
                              "if0",
                              vec![
                                b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("5")]),
                                bq!("\"Buzz\""),
                                bq!("to_str", vec![b!("+", vec![b!("i"), b!("1")])])
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
        bq!("out")
      ]
    ));

    assert_eq!(result, Ok(Literal::String("12Fizz4BuzzFizz78FizzBuzz11Fizz1314FizzBuzz".to_string())))
  }

  #[test]
  fn fizzbuzz2() {
    let result = execute(*b!(
      "seq",
      vec![
        bq!("set", vec![b!("\"out\""), b!("\"\"")]),
        bq!(
          "for",
          vec![
            b!("15"),
            b!("\"i\""),
            bq!(
              "seq",
              vec![
                bq!(
                  "set",
                  vec![
                    b!("\"tmp\""),
                    b!(
                      "strcat",
                      vec![
                        b!(
                          "ifn0",
                          vec![b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("3")]), bq!("\"\""), bq!("\"Fizz\"")]
                        ),
                        b!(
                          "ifn0",
                          vec![b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("5")]), bq!("\"\""), bq!("\"Buzz\"")]
                        )
                      ]
                    )
                  ]
                ),
                bq!(
                  "set",
                  vec![
                    b!("\"tmp\""),
                    b!(
                      "ifn0",
                      vec![
                        b!("=", vec![b!("tmp"), b!("\"\"")]),
                        bq!("to_str", vec![b!("+", vec![b!("i"), b!("1")])]),
                        bq!("tmp")
                      ]
                    )
                  ]
                ),
                bq!("set", vec![b!("\"out\""), b!("strcat", vec![b!("out"), b!("tmp")])])
              ]
            )
          ]
        ),
        bq!("out")
      ]
    ));

    assert_eq!(result, Ok(Literal::String("12Fizz4BuzzFizz78FizzBuzz11Fizz1314FizzBuzz".to_string())))
  }

  #[test]
  fn cannot_refer_inside() {
    let result = execute(*b!("seq", vec![b!("seq", vec![b!("set", vec![b!("\"out\""), b!("3")])]), b!("out")]));

    assert!(result.is_err());
  }

  #[test]
  fn simple_export() {
    let result = execute(*b!(
      "seq",
      vec![
        bq!("seq", vec![bq!("set", vec![b!("\"out\""), b!("3")]), bq!("export", vec![b!("\"out\"")])]),
        bq!("out")
      ]
    ));

    assert_eq!(result, Ok(Literal::Int(3)))
  }
}
