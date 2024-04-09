use std::collections::HashMap;

use crate::structs::{Literal, ProcedureError, ProcedureOrVar};

pub fn predefined_procs() -> HashMap<String, ProcedureOrVar> {
  let mut map: HashMap<String, ProcedureOrVar> = HashMap::new();

  macro_rules! add_map {
    ($name:expr, $callback:block; ) => {{
      map.insert($name.to_string(), ProcedureOrVar::FnProcedure(|_exec_env, _args| {
        $callback
      }))
    }};
    ($name:expr, $callback:block; $($tail:ident:$type:tt),+ ) => {{
      map.insert($name.to_string(), ProcedureOrVar::FnProcedure(|_exec_env, args| {
        initialize_vars!($name, _exec_env, args, $($tail:$type),*);
        $callback
      }))
    }};
    ($name:expr, $callback:block, $exec_env:ident, $args:ident; $($tail:ident:$type:tt),* ) => {{
      map.insert($name.to_string(), ProcedureOrVar::FnProcedure(|$exec_env, $args| {
        initialize_vars!($name, $exec_env, $args, $($tail:$type),*);
        $callback
      }))
    }};
    ($name:expr, $callback:block, $exec_env:ident, $args:ident; $($tail:ident:$type:tt),*; $list:ident:list ) => {{
      map.insert($name.to_string(), ProcedureOrVar::FnProcedure(|$exec_env, $args| {
        initialize_vars!($name, $exec_env, $args, $($tail:$type),*; $list:list);
        $callback
      }))
    }};
  }

  macro_rules! initialize_vars {
    ($name: expr, $env:expr, $vec:expr,) => {};
    ($name: expr, $env:expr, $vec:expr, $($tail:ident:$type:tt),+) => {
      if $vec.len() != count_idents!($($tail)*) {
        return Err(format!("Procesure {}: Length of args must be {}. (Got {})", $name, count_idents!($($tail)*), $vec.len()).into());
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
        return Err(format!("Procesure {}: Executed result of arg {} must be int.", $name, $block.to_string()).into());
      };
      let $tail = $tail.clone();
    };
    ($name: expr, $env:expr, $block:expr, $tail:ident:str) => {
      let Literal::String($tail) = $block else {
        return Err(format!("Procesure {}: Executed result of arg {} must be str.", $name, $block.to_string()).into());
      };
      let $tail = $tail.clone();
    };
    ($name: expr, $env:expr, $block:expr, $tail:ident:boolean) => {
      let Literal::Boolean($tail) = $block else {
        return Err(format!("Procedure {}: Executed result of arg {} must be boolean.", $name, $block.to_string()).into());
      };
      let $tail = $tail.clone();
    };
    ($name: expr, $env:expr, $block:expr, $tail:ident:block) => {
      let Literal::Block($tail) = $block else {
        return Err(format!("Procesure {}: Executed result of arg {} must be block.", $name, $block.to_string()).into());
      };
      let $tail = $tail.clone();
    };
    ($name: expr, $env:expr, $block:expr, $tail:ident:list) => {
      let Literal::List($tail) = $block else {
        return Err(format!("Procesure {}: Executed result of arg {} must be list.", $name, $block.to_string()).into());
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
    for byte in bytes {
      if let Literal::Int(b) = byte {
        data.push(u8::try_from(b.to_owned()).map_err(|e| e.to_string())?); 
      } else {
        return Err(format!("Procesure {}: Executed result of arg {} must be int.", "bytes to str", byte.to_string()).into());
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
    list.get(index_usize).cloned().ok_or("Index out of range".to_string().into())
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
  add_map!("while", {
    loop {
      let cond_res = {
        match cond.execute(exec_env) {
          Ok(res) => {
            if let Literal::Boolean(res_bool) = res {
              res_bool
            } else {
              return Err(format!("Procedure while: Executed result of arg {} must be boolean.",  res.to_string()).into());
            }
          },
          Err(err) => {return Err(err.into());}
        }
      };
      if !cond_res {break;} 
      child.execute(exec_env)?;
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
    exec_env.defset_args(&list);
    block.execute_without_scope(exec_env).map_err(|err|err.into())
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
        return Err(format!("Procesure {}: Executed result of arg {} must be str.", "cmd", l.to_string()).into());
      }
    }
    exec_env.cmd(cmd, args).map(Literal::String).map_err(|err|err.into())
  }, exec_env, args; cmd:str; list:list );

  add_map!("include", {
    exec_env.include(path)
  }, exec_env, args; path:str);

  map
}
