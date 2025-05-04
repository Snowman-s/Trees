use trees_lang::compile::{CompileConfig, CompilingBlock, QuoteStyle, connect_blocks, find_blocks, split_code};

use crate::structs::Block;

fn compiling_block_to_block(target: &CompilingBlock, blocks: &[CompilingBlock]) -> Block {
  Block {
    proc_name: target.proc_name.clone(),
    args: target
      .args
      .clone()
      .into_iter()
      .map(|edge| {
        (
          edge.arg_plug_info.expand,
          Box::new(compiling_block_to_block(
            &blocks[edge.block_index_of_block_plug],
            blocks,
          )),
        )
      })
      .collect(),
    quote: if let Some(p) = &target.block_plug {
      p.quote.clone()
    } else {
      QuoteStyle::None
    },
  }
}

pub fn compile(code: Vec<String>, config: &CompileConfig) -> Result<Block, String> {
  let splited_code = split_code(&code, config);

  let mut blocks = find_blocks(&splited_code, config);

  let head_compiling_block = connect_blocks(&splited_code, &mut blocks, config).map_err(|err| err.to_string())?;

  Ok(compiling_block_to_block(&head_compiling_block, &blocks))
}

#[cfg(test)]
mod tests {
  use trees_lang::compile::{CompileConfig, QuoteStyle};

  use crate::{compile::compile, structs::Block};

  #[test]
  fn one_block() {
    let block = compile(
      vec![
        "               ".to_owned(),
        "    ┌─────┐    ".to_owned(),
        "    │ abc │    ".to_owned(),
        "    └─────┘    ".to_owned(),
        "               ".to_owned(),
      ],
      &CompileConfig::DEFAULT,
    );

    assert_eq!(
      Ok(Block {
        proc_name: "abc".to_owned(),
        args: vec![],
        quote: QuoteStyle::None
      }),
      block
    );
  }

  #[test]
  fn one_block_half() {
    let mut config = CompileConfig::DEFAULT.clone();
    config.char_width = trees_lang::compile::CharWidthMode::Half;

    let block = compile(
      vec![
        "               ".to_owned(),
        "    ┌──────┐   ".to_owned(),
        "    │ あc  │   ".to_owned(),
        "    └──────┘   ".to_owned(),
        "               ".to_owned(),
      ],
      &config,
    );

    assert_eq!(
      Ok(Block {
        proc_name: "あc".to_owned(),
        args: vec![],
        quote: QuoteStyle::None
      }),
      block
    );
  }

  #[test]
  fn one_block_cjk() {
    let mut config = CompileConfig::DEFAULT.clone();
    config.char_width = trees_lang::compile::CharWidthMode::Full;

    let block = compile(
      vec![
        "               ".to_owned(),
        "    ┌───┐      ".to_owned(),
        "    │ abc  │   ".to_owned(),
        "    └───┘      ".to_owned(),
        "               ".to_owned(),
      ],
      &config,
    );

    assert_eq!(
      Ok(Block {
        proc_name: "abc".to_owned(),
        args: vec![],
        quote: QuoteStyle::None
      }),
      block
    );
  }

  #[test]
  fn one_block_complex() {
    let block = compile(
      vec![
        "    ".to_owned(),
        "    ┌───────┐".to_owned(),
        "    │ abc   │    ".to_owned(),
        "    │ def g │  ".to_owned(),
        "    └───────┘   ".to_owned(),
        "             ".to_owned(),
      ],
      &CompileConfig::DEFAULT,
    );

    assert_eq!(
      Ok(Block {
        proc_name: "abc\ndef g".to_owned(),
        args: vec![],
        quote: QuoteStyle::None
      }),
      block
    );
  }

  #[test]
  fn two_compile() {
    let block = compile(
      vec![
        "    ".to_owned(),
        "    ┌───────┐".to_owned(),
        "    │ abc   │    ".to_owned(),
        "    └───┬───┘   ".to_owned(),
        "    ┌───┴──┐".to_owned(),
        "    │ def  │    ".to_owned(),
        "    └──────┘   ".to_owned(),
      ],
      &CompileConfig::DEFAULT,
    );

    assert_eq!(
      Ok(Block {
        proc_name: "abc".to_owned(),
        args: vec![(
          false,
          Box::new(Block {
            proc_name: "def".to_owned(),
            args: vec![],
            quote: QuoteStyle::None
          })
        )],
        quote: QuoteStyle::None
      }),
      block
    );
  }
}
