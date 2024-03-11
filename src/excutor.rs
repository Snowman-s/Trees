use std::collections::HashMap;

use crate::structs::{Behavior, Block, ExecuteEnv, Literal};

macro_rules! initialize_vars {
  ($env:expr, $vec:expr, $($tail:ident:$type:tt),*) => {
    if $vec.len() != count_idents!($($tail)*) {
      return Err(format!("Length of args must be {}.", count_idents!($($tail)*)));
    }
    let mut iter = $vec.into_iter();
    $(
        let next = match iter.next() {
            Some(val) => val,
            None => panic!(),
        };
        declare!($env, next, $tail:$type);
    )*
  };
}

macro_rules! declare {
    ($env:expr, $block:expr, $tail:ident:any) => {
        let res = $block.execute($env);
        let $tail = match res {
            Ok(r) => r,
            err => {
                return err;
            }
        };
    };
    ($env:expr, $block:expr, $tail:ident:int) => {
        let res = $block.execute($env);
        let $tail = match res {
            Ok(r) => {
                if let Literal::Int(t) = r {
                    t
                } else {
                    return Err(format!(
                        "Executed result of arg {} must be int.",
                        r.to_string()
                    ));
                }
            }
            err => {
                return err;
            }
        };
    };
    ($env:expr, $block:expr, $tail:ident:str) => {
        let res = $block.execute($env);
        let $tail = match res {
            Ok(r) => {
                if let Literal::String(t) = r {
                    t
                } else {
                    return Err(format!(
                        "Executed result of arg {} must be string.",
                        r.to_string()
                    ));
                }
            }
            err => {
                return err;
            }
        };
    };
    ($env:expr, $block:expr, $tail:ident:block) => {
        let $tail = $block;
    };
}

macro_rules! count_idents {
  () => { 0 };
  ($_head:ident $($tail:tt)*) => { 1 + count_idents!($($tail)*) };
}

macro_rules! two_ope_int {
    ($exp:expr) => {
        |exec_env, args| {
            initialize_vars!(exec_env, args, a:int, b:int);
            Ok(Literal::Int($exp(a, b)))
        }
    };
}

fn predefined_procs() -> HashMap<String, Behavior> {
    let mut map: HashMap<String, Behavior> = HashMap::new();

    map.insert("+".to_string(), two_ope_int!(|a, b| { a + b }));
    map.insert("-".to_string(), two_ope_int!(|a, b| { a - b }));
    map.insert("*".to_string(), two_ope_int!(|a, b| { a * b }));
    map.insert("/".to_string(), two_ope_int!(|a, b| { a / b }));
    map.insert("%".to_string(), two_ope_int!(|a, b| { a % b }));
    map.insert("print".to_string(), |exec_env, args| {
        initialize_vars!(exec_env, args, a:str);

        print!("{}", a);

        Ok(Literal::Void)
    });
    map.insert("seq".to_string(), |exec_env, args| {
        for arg in args {
            arg.execute(exec_env)?;
        }
        Ok(Literal::Void)
    });
    map.insert("for".to_string(), |exec_env, args| {
        initialize_vars!(exec_env, args, times:int, var:str, child:block);
        for i in 0..times {
            child.execute(exec_env)?;
        }
        Ok(Literal::Void)
    });
    map.insert("ifn0".to_string(), |exec_env, args| {
        initialize_vars!(exec_env, args, cond:any, then:block, els:block);
        if let Literal::Int(t) = cond {
            if t == 0 {
                return els.execute(exec_env);
            }
        }
        then.execute(exec_env)
    });

    map
}

pub fn execute(tree: Block) -> Result<Literal, String> {
    let procs = predefined_procs();
    let mut exec_env = ExecuteEnv::new(procs);

    tree.execute(&mut exec_env)
}

#[cfg(test)]
mod tests {
    use crate::structs::{Block, Literal};

    use super::execute;

    #[test]
    fn simple_summing() {
        let result = execute(Block {
            proc_name: "+".to_string(),
            args: vec![
                Box::new(Block {
                    proc_name: "3".to_string(),
                    args: vec![],
                }),
                Box::new(Block {
                    proc_name: "4".to_string(),
                    args: vec![],
                }),
            ],
        });

        assert_eq!(result, Ok(Literal::Int(7)))
    }
    #[test]
    fn too_much_args() {
        let result = execute(Block {
            proc_name: "*".to_string(),
            args: vec![
                Box::new(Block {
                    proc_name: "3".to_string(),
                    args: vec![],
                }),
                Box::new(Block {
                    proc_name: "4".to_string(),
                    args: vec![],
                }),
                Box::new(Block {
                    proc_name: "5".to_string(),
                    args: vec![],
                }),
            ],
        });

        assert!(result.is_err())
    }
}
