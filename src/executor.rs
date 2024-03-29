use std::{collections::HashMap, process::Command};

use crate::structs::{BehaviorOrVar, Block, ExecuteEnv, Literal};

fn predefined_procs() -> HashMap<String, BehaviorOrVar> {
  let mut map: HashMap<String, BehaviorOrVar> = HashMap::new();

  macro_rules! add_map {
    ($name:expr, $callback:block; ) => {{
      map.insert($name.to_string(), BehaviorOrVar::Behavior(|_exec_env, _args| {
        $callback
      }))
    }};
    ($name:expr, $callback:block; $($tail:ident:$type:tt),+ ) => {{
      map.insert($name.to_string(), BehaviorOrVar::Behavior(|_exec_env, args| {
        initialize_vars!($name, _exec_env, args, $($tail:$type),*);
        $callback
      }))
    }};
    ($name:expr, $callback:block, $exec_env:ident, $args:ident; $($tail:ident:$type:tt),* ) => {{
      map.insert($name.to_string(), BehaviorOrVar::Behavior(|$exec_env, $args| {
        initialize_vars!($name, $exec_env, $args, $($tail:$type),*);
        $callback
      }))
    }};
    ($name:expr, $callback:block, $exec_env:ident, $args:ident; $($tail:ident:$type:tt),*; $list:ident:list ) => {{
      map.insert($name.to_string(), BehaviorOrVar::Behavior(|$exec_env, $args| {
        initialize_vars!($name, $exec_env, $args, $($tail:$type),*; $list:list);
        $callback
      }))
    }};
  }

  macro_rules! initialize_vars {
    ($name: expr, $env:expr, $vec:expr,) => {};
    ($name: expr, $env:expr, $vec:expr, $($tail:ident:$type:tt),+) => {
      if $vec.len() != count_idents!($($tail)*) {
        return Err(format!("Procesure {}: Length of args must be {}. (Got {})", $name, count_idents!($($tail)*), $vec.len()));
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
    ($name: expr, $env:expr, $vec:expr, $($tail:ident:$type:tt),*; $list:ident:list) => {
      let mut iter = $vec.into_iter();
      $(
        let next = match iter.next() {
          Some(val) => val,
          None => panic!(),
        };
        declare!($name, $env, next, $tail:$type);
      )*
      let $list: Vec<Literal> = iter.map(|c|c.clone()).collect();
    }
  }

  macro_rules! declare {
    ($name: expr, $env:expr, $block:expr, $tail:ident:any) => {
      let $tail = $block.clone();
    };
    ($name: expr, $env:expr, $block:expr, $tail:ident:int) => {
      let Literal::Int($tail) = $block else {
        return Err(format!("Procesure {}: Executed result of arg {} must be int.", $name, $block.to_string()));
      };
      let $tail = $tail.clone();
    };
    ($name: expr, $env:expr, $block:expr, $tail:ident:str) => {
      let Literal::String($tail) = $block else {
        return Err(format!("Procesure {}: Executed result of arg {} must be str.", $name, $block.to_string()));
      };
      let $tail = $tail.clone();
    };
    ($name: expr, $env:expr, $block:expr, $tail:ident:block) => {
      let Literal::Block($tail) = $block else {
        return Err(format!("Procesure {}: Executed result of arg {} must be block.", $name, $block.to_string()));
      };
      let $tail = $tail.clone();
    };
    ($name: expr, $env:expr, $block:expr, $tail:ident:list) => {
      let Literal::List($tail) = $block else {
        return Err(format!("Procesure {}: Executed result of arg {} must be list.", $name, $block.to_string()));
      };
      let $tail = $tail.clone();
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
  add_map!("to str", {Ok(Literal::String(a.to_string()))}; a:any);
  add_map!("str to int", {
    Ok(Literal::Int(i64::from_str_radix(&a, 10).map_err(|e|e.to_string())?))
  }; a:str);
  add_map!("get", {exec_env.get_var(&name)}, exec_env, _args; name:str);
  add_map!("defset", {
    exec_env.defset_var(&name, &from);
    Ok(Literal::Void)
  }, exec_env, _args; name:str, from:any);
  add_map!("set", {
    exec_env.set_var(&name, &from)?;
    Ok(Literal::Void)
  }, exec_env, _args; name:str, from:any);
  add_map!("print", {
    exec_env.print(a.to_string());
    Ok(Literal::Void)
  }, exec_env, args; a:any);
  add_map!("println", {
    exec_env.print(a.to_string() + "\n");
    Ok(Literal::Void)
  }, exec_env, args; a:any);
  add_map!("read line", { Ok(Literal::String(exec_env.read_line())) }, exec_env, args;);

  add_map!("split str", {
    Ok(Literal::List(origin.split(&spliter).map(|str|Literal::String(str.to_owned())).collect()))
  }; origin: str, spliter: str);
  add_map!("str to bytes", {
    Ok(Literal::List(string.as_bytes().iter().map(|b|Literal::Int((*b).into())).collect()))
  }; string:str);
  add_map!("bytes to str", {
    let mut data = vec![];
    for byte in bytes {
      if let Literal::Int(b) = byte {
        data.push(u8::try_from(b.to_owned()).map_err(|e| e.to_string())?); 
      } else {
        return Err(format!("Procesure {}: Executed result of arg {} must be int.", "bytes to str", byte.to_string()));
      }
    }
    Ok(Literal::String(String::from_utf8_lossy(&data).to_string()))
  }; bytes:list);
  add_map!(r"\n", {Ok(Literal::String("\n".to_owned()))};);
  add_map!(r"\r", {Ok(Literal::String("\r".to_owned()))};);
  add_map!(r"\t", {Ok(Literal::String("\t".to_owned()))};);
  add_map!(r"\0", {Ok(Literal::String("\0".to_owned()))};);
  add_map!("listing", {
    Ok(Literal::List(list))
  }, _exec_env, args;;list:list);
  add_map!("[]", {
    let index_usize:usize = usize::try_from( index).map_err(|e|e.to_string())?;
    list.get(index_usize).map(|lit|lit.clone()).ok_or("Index out of range".to_string())
  };list:list, index:int);
  add_map!("len", {
    Ok(Literal::Int(i64::try_from(list.len()).map_err(|err|err.to_string())?))
  };list:list);

  add_map!("seq", {
    Ok(list.last().unwrap_or(&Literal::Void).clone())
  }, _exec_env, args;;list:list);
  add_map!("for", {
    for i in 0..times {
      exec_env.defset_var(&var, &Literal::Int(i));
      child.execute(exec_env)?;
    }
    Ok(Literal::Void)
  }, exec_env, args; times:int, var:str, child:block);
  add_map!("if0", {
    Ok(if let Literal::Int(0) = cond {
      then
    } else {
      els
    })
  }; cond:any, then:any, els:any );
  add_map!("ifn0", {
    Ok(if let Literal::Int(0) = cond {
      els
    } else {
      then
    })
  }; cond:any, then:any, els:any );
  add_map!("defproc", {
    exec_env.def_proc(&name, &block);
    Ok(Literal::Void)
  }, exec_env, args; name: str, block:block);
  add_map!("exec", {
    exec_env.defset_var("$args", &Literal::List(list.clone()));
    for (i, arg) in list.iter().enumerate() {
      exec_env.defset_var(&format!("${}", i), arg);
    }

    block.execute(exec_env)
  }, exec_env, args; block:block; list:list);
  add_map!("export", {
    exec_env.export(&name)?;
    Ok(Literal::Void)
  }, exec_env, args; name:str );

  add_map!("cmd", {
    let mut args = vec![];
    for l in list {
      if let Literal::String(s) = l {
        args.push( s.to_owned()); 
      } else {
        return Err(format!("Procesure {}: Executed result of arg {} must be str.", "cmd", l.to_string()));
      }
    }
    exec_env.cmd(cmd, args).map(|responce|Literal::String(responce))
  }, exec_env, args; cmd:str; list:list );

  add_map!("include", {
    exec_env.include(path)
  }, exec_env, args; path:str);

  map
}

pub fn execute(tree: Block, includer: Box<dyn FnMut(String) -> Result<Literal, String>>) -> Result<Literal, String> {
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
  includer: Box<dyn FnMut(String) -> Result<Literal, String>>,
) -> Result<Literal, String> {
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

  #[test]
  fn simple_summing() {
    let result = execute(*b!("+", vec![b!("3"), b!("4")]), Box::new(|_| panic!()));

    assert_eq!(result, Ok(Literal::Int(7)))
  }
  #[test]
  fn too_much_args() {
    let result = execute(*b!("+", vec![b!("3"), b!("4"), b!("5")]), Box::new(|_| panic!()));

    assert!(result.is_err())
  }

  #[test]
  fn fizzbuzz() {
    let result = execute(
      *b!(
        "seq",
        vec![
          b!("defset", vec![b!("\"out\""), b!("\"\"")]),
          b!(
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
                          b!("\"FizzBuzz\""),
                          b!(
                            "if0",
                            vec![
                              b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("3")]),
                              b!("\"Fizz\""),
                              b!(
                                "if0",
                                vec![
                                  b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("5")]),
                                  b!("\"Buzz\""),
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
      ),
      Box::new(|_| panic!()),
    );

    assert_eq!(result, Ok(Literal::String("12Fizz4BuzzFizz78FizzBuzz11Fizz1314FizzBuzz".to_string())))
  }

  #[test]
  fn fizzbuzz2() {
    let result = execute(
      *b!(
        "seq",
        vec![
          b!("defset", vec![b!("\"out\""), b!("\"\"")]),
          b!(
            "for",
            vec![
              b!("15"),
              b!("\"i\""),
              bq!(
                "seq",
                vec![
                  b!(
                    "defset",
                    vec![
                      b!("\"tmp\""),
                      b!(
                        "strcat",
                        vec![
                          b!(
                            "ifn0",
                            vec![b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("3")]), b!("\"\""), b!("\"Fizz\"")]
                          ),
                          b!(
                            "ifn0",
                            vec![b!("%", vec![b!("+", vec![b!("i"), b!("1")]), b!("5")]), b!("\"\""), b!("\"Buzz\"")]
                          )
                        ]
                      )
                    ]
                  ),
                  b!(
                    "set",
                    vec![
                      b!("\"tmp\""),
                      b!(
                        "ifn0",
                        vec![
                          b!("=", vec![b!("tmp"), b!("\"\"")]),
                          b!("to str", vec![b!("+", vec![b!("i"), b!("1")])]),
                          b!("tmp")
                        ]
                      )
                    ]
                  ),
                  b!("set", vec![b!("\"out\""), b!("strcat", vec![b!("out"), b!("tmp")])]),
                ]
              )
            ]
          ),
          b!("out")
        ]
      ),
      Box::new(|_| panic!()),
    );

    assert_eq!(result, Ok(Literal::String("12Fizz4BuzzFizz78FizzBuzz11Fizz1314FizzBuzz".to_string())))
  }

  #[test]
  fn cannot_refer_inside() {
    let result = execute(
      *b!("seq", vec![b!("seq", vec![b!("defset", vec![b!("\"out\""), b!("3")])]), b!("out")]),
      Box::new(|_| panic!()),
    );

    assert!(result.is_err());
  }

  #[test]
  fn simple_export() {
    let result = execute(
      *b!(
        "seq",
        vec![
          b!("seq", vec![b!("defset", vec![b!("\"out\""), b!("3")]), b!("export", vec![b!("\"out\"")])]),
          b!("out")
        ]
      ),
      Box::new(|_| panic!()),
    );

    assert_eq!(result, Ok(Literal::Int(3)))
  }
}
