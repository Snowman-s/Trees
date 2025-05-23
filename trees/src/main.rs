mod compile;
mod executor;
mod intermed_repr;
mod structs;

use std::{
  fs::File,
  io::{Read, Write},
  path::{Path, PathBuf},
  process::exit,
  rc::Rc,
};

use compile::compile;

use walkdir::WalkDir;

use clap::{Parser, ValueEnum};

use trees_lang::compile::CompileConfig;

use executor::execute;

use structs::{Block, BlockError, BlockErrorTree, BlockResult};

#[derive(Parser)]
#[command(
  name = "Trees",
  version = "0.2.3",
  author = "SnowEsamosc <snowman.snowsnow@gmail.com>"
)]
struct Cli {
  #[arg(short, long, value_enum, default_value_t=CommandMode::Auto)]
  mode: CommandMode,

  #[arg(short, long, value_enum, default_value_t=CharWidthMode::Mono)]
  char_width: CharWidthMode,

  input: PathBuf,
}

impl Cli {
  fn create_compile_config(&self) -> CompileConfig {
    CompileConfig {
      char_width: match self.char_width {
        CharWidthMode::Mono => trees_lang::compile::CharWidthMode::Mono,
        CharWidthMode::Half => trees_lang::compile::CharWidthMode::Half,
        CharWidthMode::Full => trees_lang::compile::CharWidthMode::Full,
      },
    }
  }
}

#[derive(Clone, PartialEq, Eq, ValueEnum)]
enum CommandMode {
  // ファイル拡張子を見て自動でコマンドを実行
  Auto,
  // コンパイル
  Compile,
  // 中間コード実行
  Exec,
  // 直接実行(Execute Directly)
  ExecD,
}

#[derive(Clone, PartialEq, Eq, ValueEnum)]
enum CharWidthMode {
  // 全部長さ1
  Mono,
  // Ambiguousなものは半角
  Half,
  // Ambiguousなものは全角
  Full,
}

fn main() {
  let cli = Cli::parse();

  let mut cmd_mode = cli.mode.clone();

  let compile_config = cli.create_compile_config();

  if cmd_mode == CommandMode::Auto {
    if cli.input.is_dir() {
      cmd_mode = CommandMode::Compile
    } else {
      match cli.input.extension() {
        Some(str) => {
          if str == "tr" {
            cmd_mode = CommandMode::ExecD;
          } else if str == "trm" {
            cmd_mode = CommandMode::Exec;
          }
        }
        None => {
          eprintln!("Cannot determine mode from that file name. Please specify `--mode`.");
          exit(-1);
        }
      }
    }
  }

  let copy_config = compile_config.clone();
  //Includer を設定
  let includer = |parent: Rc<PathBuf>| {
    Box::new(move |name: &Vec<String>| {
      let target = name.iter().fold(parent.to_path_buf(), |a, b| a.join(b));
      match target.extension() {
        Some(ext) => {
          if ext == "tr" {
            compile_file(&target, &copy_config)
          } else {
            // 中間コード
            let file = File::open(target).map_err(|e| e.to_string())?;
            let block = Block::from_intermed_repr(&mut file.bytes().map(|res| res.unwrap()));
            Ok(block)
          }
        }
        None => {
          // 中間コード
          let file = File::open(target).map_err(|e| e.to_string())?;
          let block = Block::from_intermed_repr(&mut file.bytes().map(|res| res.unwrap()));
          Ok(block)
        }
      }
    })
  };

  match cmd_mode {
    CommandMode::Auto => unreachable!(),
    CommandMode::Compile => {
      if cli.input.is_dir() {
        for path in WalkDir::new(cli.input).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {
          if let Some(ext) = path.path().extension() {
            if ext == "tr" {
              if let Err(err) = write_compiled_file(path.path(), &compile_config) {
                eprintln!("Error in {}: {}", path.path().to_str().unwrap_or("?"), err)
              }
            }
          }
        }
      } else if let Err(err) = write_compiled_file(&cli.input, &compile_config) {
        eprintln!("Error in {}: {}", cli.input.to_str().unwrap_or("?"), err)
      }
    }
    CommandMode::Exec => {
      let file = File::open(&cli.input).unwrap();
      let block = Block::from_intermed_repr(&mut file.bytes().map(|res| res.unwrap()));
      let parent = Rc::new(cli.input.parent().unwrap().to_path_buf());
      match execute(block, includer(parent)) {
        Ok(_) => {}
        Err(err) => print_error(&err),
      };
    }
    CommandMode::ExecD => {
      let block = compile_file(cli.input.as_path(), &compile_config).unwrap();
      let parent = Rc::new(cli.input.parent().unwrap().to_path_buf());
      match execute(block, includer(parent)) {
        Ok(_) => {}
        Err(err) => print_error(&err),
      };
    }
  }
}

fn compile_file(file_path: &Path, config: &CompileConfig) -> Result<Block, String> {
  let mut codes = File::open(file_path).map_err(|err| format!("failed to read {:?}: {}", &file_path.to_str(), err))?;
  let mut buf: String = String::new();
  codes.read_to_string(&mut buf).map_err(|err| format!("failed to read {:?}: {}", &file_path.to_str(), err))?;

  compile(buf.split('\n').map(|t| t.to_owned()).collect(), config)
}

fn write_compiled_file(path: &Path, config: &CompileConfig) -> Result<(), String> {
  let block = compile_file(path, config)?;
  let mut output = path.to_path_buf();
  output.set_extension("trm");
  let mut file = File::create(output).map_err(|e| e.to_string())?;
  file.write_all(&block.to_intermed_repr()).map_err(|e| e.to_string())?;

  Ok(())
}

fn print_error(error: &BlockError) {
  eprintln!("\n\nエラーが発生しました：{}\n◦", error.msg);
  print_error_rec(&error.root, &mut vec![false]);

  let mut before_error = error;
  while let Some(now_error) = &before_error.caused_by {
    eprintln!("\n\n起因：\n◦");
    print_error_rec(&now_error.root, &mut vec![false]);
    before_error = now_error;
  }

  eprintln!("\n名前空間：");
  for scope in &error.scopes {
    let keys: Vec<String> = scope
      .borrow()
      .namespace
      .iter()
      .map(|(k, v)| {
        format!(
          "{}{}",
          k,
          match v {
            structs::ProcedureOrVar::Var(var) => format!("={}", var.to_string()),
            _ => "".to_owned(),
          }
        )
      })
      .collect();
    eprintln!("[{}]", keys.join(", "));
  }
}

fn print_error_rec(tree: &BlockErrorTree, after_exists: &mut Vec<bool>) {
  // 上位の線を表示
  for a in after_exists[..after_exists.len() - 1].iter() {
    if *a {
      eprint!("│");
    } else {
      eprint!(" ");
    }
  }

  // 自身の線を表示
  eprintln!(
    "{}{} {}",
    if tree.expand {
      "@"
    } else if *after_exists.last().unwrap() {
      "├"
    } else {
      "└"
    },
    tree.proc_name,
    match &tree.result {
      BlockResult::Success(literal) => format!("= {}", literal.to_string()),
      BlockResult::Error => "<-".to_owned(),
      BlockResult::Unreached => "".to_owned(),
    }
  );

  after_exists.push(true);
  let last_index = after_exists.len() - 1;

  let child_len = tree.children.len();
  for (i, child) in tree.children.iter().enumerate() {
    if i == child_len - 1 {
      after_exists[last_index] = false;
    }
    print_error_rec(child, after_exists);
  }

  after_exists.pop();
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use trees_lang::compile::CompileConfig;

  use crate::{
    executor::execute_with_mock,
    structs::{BlockError, Literal},
  };

  #[test]
  fn a_plus_b() {
    let out = Rc::new(RefCell::new("".to_owned()));
    let out_ref = out.clone();
    let out_stream = Box::new(move |msg| {
      *out.borrow_mut() = msg;
    });
    let cmd_executor = Box::new(|_, _| panic!());

    let result = crate::compile::compile(
      vec![
        "        ┌─────┐      ".to_owned(),
        "        │print│      ".to_owned(),
        "        └───┬─┘      ".to_owned(),
        "        ┌───┴─┐      ".to_owned(),
        "    ┌───┤  +  ├──┐   ".to_owned(),
        "    │   └─────┘  │   ".to_owned(),
        "┌───┴─┐      ┌───┴─┐ ".to_owned(),
        "│  3  │      │  4  │ ".to_owned(),
        "└─────┘      └─────┘ ".to_owned(),
      ],
      &CompileConfig::DEFAULT,
    )
    .and_then(|b| {
      execute_with_mock(
        b,
        Box::new(|| panic!()),
        out_stream,
        cmd_executor,
        Box::new(|_| panic!()),
      )
      .map_err(|e: BlockError| e.msg)
    });

    assert_eq!(Ok(Literal::Void), result);
    assert_eq!("7", *out_ref.borrow());
  }

  type Returning = Result<Literal, String>;
  type StdoutResult = String;
  type RequiredCommands = Vec<(String, Vec<String>)>;
  struct ExectuedResult {
    returning: Returning,
    stdout: StdoutResult,
    cmd: RequiredCommands,
  }

  fn exec_file(code: &str) -> ExectuedResult {
    exec_file_with_config(code, &CompileConfig::DEFAULT)
  }

  fn exec_file_with_config(code: &str, config: &CompileConfig) -> ExectuedResult {
    let out = Rc::new(RefCell::new("".to_owned()));
    let out_ref = out.clone();
    let out_stream = Box::new(move |msg| {
      (*out.borrow_mut()).extend([msg]);
    });
    let cmd_log: Rc<RefCell<RequiredCommands>> = Rc::new(RefCell::new(vec![]));
    let cmd_log_ref = cmd_log.clone();
    let cmd_executor = Box::new(move |cmd, args| {
      (*cmd_log.borrow_mut()).push((cmd, args));
      Ok("".to_string())
    });

    let code_lines: Vec<String> = code.split('\n').map(|c| c.to_owned()).collect();
    let returning = crate::compile::compile(code_lines, config).and_then(|b| {
      execute_with_mock(
        b,
        Box::new(|| panic!()),
        out_stream,
        cmd_executor,
        Box::new(|_| panic!()),
      )
      .map_err(|e: BlockError| e.msg)
    });

    let stdout = out_ref.borrow().clone();
    let cmd = cmd_log_ref.borrow().clone();

    ExectuedResult { returning, stdout, cmd }
  }

  #[test]
  fn minus() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/minus.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "-1\n");
  }

  #[test]
  fn println() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/println.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "7\n");
  }

  #[test]
  fn fizzbuzz() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/fizzbuzz.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "12Fizz4BuzzFizz78FizzBuzz11Fizz1314FizzBuzz");
  }

  #[test]
  fn defproc() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/defproc.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "6");
  }

  #[test]
  fn substance() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/substance.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "6");
  }

  #[test]
  fn bind_var() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/bind_var.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "42");
  }

  #[test]
  fn bind_var2() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/bind_var2.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "42");
  }

  #[test]
  fn bind_var3() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/bind_var3.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "42");
  }

  #[test]
  fn generator() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/generator.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "12");
  }

  #[test]
  fn cmd() {
    let ExectuedResult {
      returning, stdout, cmd, ..
    } = exec_file(include_str!("test/cmd.tr"));
    assert_eq!(returning, Ok(Literal::String("".to_string())));
    assert_eq!(stdout, "");
    assert_eq!(cmd, vec![("echo".to_string(), vec!["out".to_string()])]);
  }

  #[test]
  fn lists() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/lists.tr"));
    assert_eq!(returning, Ok(Literal::Int(7)));
    assert_eq!(stdout, "");
  }

  #[test]
  fn string_bytes() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/string_bytes.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "AAA\n");
  }

  #[test]
  fn recursion() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/recursion.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "6\n");
  }

  #[test]
  fn recursion2() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/recursion2.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "6\n");
  }

  #[test]
  fn tr_while() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/tr_while.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "012");
  }

  #[test]
  fn secret_for() {
    let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/secret_for.tr"));
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "42\n");
  }

  #[test]
  fn char_half() {
    let mut config = CompileConfig::DEFAULT.clone();
    config.char_width = trees_lang::compile::CharWidthMode::Half;
    let ExectuedResult { returning, stdout, .. } =
      exec_file_with_config(include_str!("test/helloworld_half.tr"), &config);
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "こんにちは！世界！\n");
  }

  #[test]
  fn char_full() {
    let mut config = CompileConfig::DEFAULT.clone();
    config.char_width = trees_lang::compile::CharWidthMode::Full;
    let ExectuedResult { returning, stdout, .. } =
      exec_file_with_config(include_str!("test/helloworld_full.tr"), &config);
    assert_eq!(returning, Ok(Literal::Void));
    assert_eq!(stdout, "こんにちは！世界！\n");
  }

  mod modules {
    use crate::{
      structs::Literal,
      tests::{ExectuedResult, exec_file},
    };

    #[test]
    fn modules() {
      let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/modules/modules.tr"));
      assert_eq!(returning, Ok(Literal::Void));
      assert_eq!(stdout, "6");
    }

    #[test]
    fn modules_err() {
      let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/modules/modules_err.tr"));
      assert!(returning.is_err());
      assert_eq!(stdout, "");
    }

    #[test]
    fn reexport() {
      let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/modules/reexport.tr"));
      assert_eq!(returning, Ok(Literal::Void));
      assert_eq!(stdout, "12");
    }

    #[test]
    fn tereport_var() {
      let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/modules/tereport_var.tr"));
      assert_eq!(returning, Ok(Literal::Void));
      assert_eq!(stdout, "42");
    }

    #[test]
    fn tereport_var_2times() {
      let ExectuedResult { returning, stdout, .. } = exec_file(include_str!("test/modules/tereport_var_2times.tr"));
      assert_eq!(returning, Ok(Literal::Void));
      assert_eq!(stdout, "42");
    }
  }
}
