use std::collections::HashMap;

use crate::structs::{Literal, ProcedureOrVar};

fn type_error_msg(proc_name: &str, index: usize, actually: &Literal, expected: &str) -> String {
  format!(
    "Procedure {}: $arg[{}] must be {}. (Got {})",
    proc_name,
    index,
    expected,
    actually.to_string()
  )
}

fn block_type_error_msg(proc_name: &str, index: usize, actually: &Literal, expected: &str) -> String {
  format!(
    "Procedure {}: Executed result of $arg[{}] must be {}. (Got {})",
    proc_name,
    index,
    expected,
    actually.to_string()
  )
}

fn list_type_error_msg(
  proc_name: &str,
  arg_index: usize,
  list_index: usize,
  actually: &Literal,
  expected: &str,
) -> String {
  format!(
    "Procedure {}: [{}] of $arg[{}] must be {}. (Got {})",
    proc_name,
    list_index,
    arg_index,
    expected,
    actually.to_string()
  )
}

#[allow(unused_variables, unused_mut)]
pub fn predefined_procs() -> HashMap<String, ProcedureOrVar> {
  let mut map: HashMap<String, ProcedureOrVar> = HashMap::new();

  macro_rules! add_map {
    ($name:expr, $callback:block; $($tail:ident:$type:tt),* ) => {{
      map.insert($name.to_string(), ProcedureOrVar::FnProcedure(|_exec_env, args| {
        initialize_vars!($name, args, $($tail:$type),*);
        $callback
      }))
    }};
    ($name:expr, $callback:block, $exec_env:ident, $args:ident; $($tail:ident:$type:tt),* ) => {{
      map.insert($name.to_string(), ProcedureOrVar::FnProcedure(|$exec_env, $args| {
        initialize_vars!($name, $args, $($tail:$type),*);
        $callback
      }))
    }};
    ($name:expr, $callback:block, $exec_env:ident, $args:ident; $($tail:ident:$type:tt),*; $list:ident:list ) => {{
      map.insert($name.to_string(), ProcedureOrVar::FnProcedure(|$exec_env, $args| {
        initialize_vars!($name, $args, $($tail:$type),*; $list:list);
        $callback
      }))
    }};
  }

  macro_rules! initialize_vars {
    ($name: expr, $vec:expr, $($tail:ident:$type:tt),*) => {
      if $vec.len() != count_idents!($($tail)*) {
        return Err(format!("Procedure {}: Length of args must be {}. (Got {})", $name, count_idents!($($tail)*), $vec.len()).into());
      }
      let mut iter = $vec.into_iter().enumerate();
      $(
        let (index, next) = match iter.next() {
          Some(val) => val,
          None => panic!(),
        };
        declare!(index, $name, next, $tail:$type);
      )*
    };
    ($name: expr, $vec:expr, $($tail:ident:$type:tt),*; $list:ident:list) => {
      let mut iter = $vec.into_iter().enumerate();
      $(
        let (index, next) = match iter.next() {
          Some(val) => val,
          None => panic!(),
        };
        declare!(index, $name, next, $tail:$type);
      )*
      let $list: Vec<Literal> = iter.map(|(_index, lit)|lit).cloned().collect();
    }
  }

  macro_rules! declare {
    ($index: expr, $name: expr, $literal:expr, $tail:ident:any) => {
      let $tail = $literal.clone();
    };
    ($index: expr, $name: expr, $literal:expr, $tail:ident:int) => {
      let Literal::Int($tail) = $literal else {
        return Err(type_error_msg($name, $index, $literal, "int").into());
      };
      let $tail = $tail.clone();
    };
    ($index: expr, $name: expr, $literal:expr, $tail:ident:str) => {
      let Literal::String($tail) = $literal else {
        return Err(type_error_msg($name, $index, $literal, "str").into());
      };
      let $tail = $tail.clone();
    };
    ($index: expr, $name: expr, $literal:expr, $tail:ident:boolean) => {
      let Literal::Boolean($tail) = $literal else {
        return Err(type_error_msg($name, $index, $literal, "boolean").into());
      };
      let $tail = $tail.clone();
    };
    ($index: expr, $name: expr, $literal:expr, $tail:ident:block) => {
      let Literal::Block($tail) = $literal else {
        return Err(type_error_msg($name, $index, $literal, "block").into());
      };
      let $tail = $tail.clone();
    };
    ($index: expr, $name: expr, $literal:expr, $tail:ident:list) => {
      let Literal::List($tail) = $literal else {
        return Err(type_error_msg($name, $index, $literal, "list").into());
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
  add_map!("=", {Ok(Literal::Boolean(a == b))}; a:any, b:any);
  add_map!("and", {Ok(Literal::Boolean(a & b))}; a:boolean, b:boolean);
  add_map!("or", {Ok(Literal::Boolean(a | b))}; a:boolean, b:boolean);
  add_map!("xor", {Ok(Literal::Boolean(a ^ b))}; a:boolean, b:boolean);
  add_map!("<", {Ok(Literal::Boolean(a < b))}; a:int, b:int);
  add_map!(">", {Ok(Literal::Boolean(a > b))}; a:int, b:int);
  add_map!("<=", {Ok(Literal::Boolean(a <= b))}; a:int, b:int);
  add_map!(">=", {Ok(Literal::Boolean(a >= b))}; a:int, b:int);
  add_map!("strcat", {Ok(Literal::String(format!("{}{}", a, b)))}; a:str, b:str);
  add_map!("to str", {Ok(Literal::String(a.to_string()))}; a:any);
  add_map!("str to int", {
    Ok(Literal::Int(a.parse::<i64>().map_err(|e|e.to_string())?))
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
    Ok(Literal::List(origin.split(&spliter).filter(|str| !str.is_empty()).map(|str|Literal::String(str.to_owned())).collect()))
  }; origin: str, spliter: str);
  add_map!("str to bytes", {
    Ok(Literal::List(string.as_bytes().iter().map(|b|Literal::Int((*b).into())).collect()))
  }; string:str);
  add_map!("bytes to str", {
    let mut data = vec![];
    for (index, byte) in bytes.iter().enumerate() {
      if let Literal::Int(b) = byte {
        data.push(u8::try_from(b.to_owned()).map_err(|e| e.to_string())?); 
      } else {
        return Err(list_type_error_msg("bytes to str", index, 0, byte, "int").into());
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
    list.get(index_usize).cloned().ok_or(format!("Index ({}) out of range. (Length = {})", index, list.len()).into())
  };list:list, index:int);
  add_map!("len", {
    Ok(Literal::Int(i64::try_from(list.len()).map_err(|err|err.to_string())?))
  };list:list);

  add_map!("seq", {
    Ok(list.last().unwrap_or(&Literal::Void).clone())
  }, _exec_env, args;;list:list);
  add_map!("for", {
    for i in 0..times {
      child.execute_without_scope(exec_env, |exec_env|{exec_env.defset_var_into_last_scope(&var, &Literal::Int(i))})?;
    }
    Ok(Literal::Void)
  }, exec_env, args; times:int, var:str, child:block);
  add_map!("while", {
    loop {
      let cond_res = {
        match cond.execute_without_scope(exec_env, |_|{}) {
          Ok(res) => {
            if let Literal::Boolean(res_bool) = res {
              res_bool
            } else {
              return Err(block_type_error_msg("while", 0, &res, "boolean").into());
            }
          },
          Err(err) => {return Err(err.into());}
        }
      };
      if !cond_res {break;} 
      child.execute_without_scope(exec_env, |_|{})?;
    }
    Ok(Literal::Void)
  }, exec_env, args; cond:block, child:block);
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
  add_map!("if", {
    Ok(if cond {
        then 
      } else {
        els 
      }
    )
  }; cond:boolean, then:any, els:any);
  add_map!("defproc", {
    exec_env.def_proc(&name, &block);
    Ok(Literal::Void)
  }, exec_env, args; name: str, block:block);
  add_map!("exec", {
    block.execute_without_scope(exec_env, |exec_env| exec_env.defset_args(&list)).map_err(|err|err.into())
  }, exec_env, args; block:block; list:list);
  add_map!("export", {
    exec_env.export(&name)?;
    Ok(Literal::Void)
  }, exec_env, args; name:str );
  add_map!("reexport", {
    exec_env.reexport();

    Ok(child)
  }, exec_env, args; child: any);

  add_map!("cmd", {
    let mut args = vec![];
    for (index, l) in list.iter().enumerate() {
      if let Literal::String(s) = l {
        args.push( s.to_owned()); 
      } else {
        return Err(list_type_error_msg("cmd", index, 1, l, "str").into());
      }
    }
    exec_env.cmd(cmd, args).map(Literal::String).map_err(|err|err.into())
  }, exec_env, args; cmd:str; list:list );

  add_map!("include", {
    exec_env.include(path)
  }, exec_env, args; path:str);

  map
}
