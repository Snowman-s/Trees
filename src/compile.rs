use std::cmp::Ordering;

use unicode_width::UnicodeWidthStr;

use crate::structs::{Block, QuoteStyle};

#[derive(Debug, Clone)]
pub struct CompileConfig {
  pub char_width: CharWidthMode,
}

#[derive(Debug, Clone)]
pub enum CharWidthMode {
  // 全部長さ1
  Mono,
  // Ambiguousなものは半角
  Half,
  // Ambiguousなものは全角
  Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeCharacter {
  char: String,
  x: usize,
  len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitedCode {
  body: Vec<Vec<CodeCharacter>>,
}

impl SplitedCode {
  pub fn get(&self, x: usize, y: usize) -> Option<CodeCharacter> {
    self.body.get(y)?.iter().find(|cc| cc.x == x).cloned()
  }
  pub fn get_slice_of_line(&self, x_min_exclusive: usize, x_max_exclusive: usize, y: usize) -> Option<String> {
    let (mut index, _) = self.body.get(y)?.iter().enumerate().find(|(_index, cc)| cc.x == x_min_exclusive)?;
    let mut return_str = "".to_string();

    //exclusiveなので
    index += 1;

    while let Some(cc) = self.body.get(y)?.get(index) {
      match cc.x.cmp(&x_max_exclusive) {
        Ordering::Equal => break,
        Ordering::Greater => return None,
        Ordering::Less => {}
      }
      return_str += &cc.char;

      index += 1;
    }

    Some(return_str.to_string())
  }

  pub fn left_x(&self, x: usize, y: usize) -> Option<usize> {
    let index =
      self.body.get(y)?.iter().enumerate().find_map(|(index, cc)| if cc.x == x { Some(index) } else { None })?;
    self.body.get(y)?.get(index - 1).map(|cc| cc.x)
  }
  pub fn right_x(&self, x: usize, y: usize) -> Option<usize> {
    let index =
      self.body.get(y)?.iter().enumerate().find_map(|(index, cc)| if cc.x == x { Some(index) } else { None })?;
    self.body.get(y)?.get(index + 1).map(|cc| cc.x)
  }

  fn new() -> Self {
    SplitedCode { body: vec![vec![]] }
  }

  fn append(&mut self, char: &str, char_width: &CharWidthMode) {
    let now_line = self.body.last_mut().unwrap();

    let x = if now_line.is_empty() {
      0
    } else {
      now_line.last().unwrap().x + now_line.last().unwrap().len
    };

    let width = char.width();
    let width_cjk = char.width_cjk();

    now_line.push(CodeCharacter {
      char: char.to_string(),
      x,
      len: match char_width {
        CharWidthMode::Mono => 1,
        CharWidthMode::Half => width,
        CharWidthMode::Full => width_cjk,
      },
    });
  }
  pub fn new_line(&mut self) {
    self.body.push(vec![]);
  }

  pub fn len_y(&self) -> usize {
    self.body.len()
  }

  pub fn enumurate_x(&self, y: usize) -> Box<dyn std::iter::Iterator<Item = usize> + '_> {
    Box::new(self.body[y].iter().map(|cc| cc.x))
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilingBlock {
  pub proc_name: String,
  pub x: usize,
  pub y: usize,
  pub width: usize,
  pub height: usize,
  pub block_plug: Option<BlockPlug>,
  pub arg_plugs: Vec<ArgPlug>,
  pub args: Vec<Edge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgPlug {
  pub x: usize,
  pub y: usize,
  pub expand: bool,
  pub ori: Orientation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeFragment {
  pub x: usize,
  pub y: usize,
  pub ori: Orientation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
  pub block_index_of_arg_plug: usize,
  pub arg_plug_info: ArgPlug,
  pub fragments: Vec<EdgeFragment>,
  pub block_index_of_block_plug: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockPlug {
  pub x: usize,
  pub y: usize,
  pub quote: QuoteStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
  Up,
  Left,
  Right,
  Down,
}

impl CompilingBlock {
  fn to_block(&self, blocks: &[CompilingBlock]) -> Block {
    Block {
      proc_name: self.proc_name.clone(),
      args: self
        .args
        .clone()
        .into_iter()
        .map(|edge| {
          (
            edge.arg_plug_info.expand,
            Box::new(blocks[edge.block_index_of_block_plug].to_block(blocks)),
          )
        })
        .collect(),
      quote: if let Some(p) = &self.block_plug {
        p.quote.clone()
      } else {
        QuoteStyle::None
      },
    }
  }
}

fn find_a_block(code: &SplitedCode, x: usize, y: usize, _config: &CompileConfig) -> Option<CompilingBlock> {
  let cc = |dx: usize, dy: usize| -> Option<CodeCharacter> { code.get(x + dx, y + dy) };
  let char = |dx: usize, dy: usize| -> Option<String> { code.get(x + dx, y + dy).map(|x| x.char.clone()) };

  let char_is_in = |dx: usize, dy: usize, targets: &[&str]| -> Option<bool> {
    let c = char(dx, dy)?;

    let matched = targets.iter().any(|t| *t == c);

    Some(matched)
  };

  let mut up_plug = None;
  let mut arg_plugs: Vec<_> = vec![];

  if char(0, 0)? != "┌" {
    return None;
  };
  // 右回り
  // 1から始める
  let mut width1 = code.right_x(x, y)? - x;
  while char_is_in(width1, 0, &["─", "┴", "•", "/"])? {
    match char(width1, 0)?.as_str() {
      "┴" => {
        up_plug = Some(BlockPlug {
          x: x + width1,
          y,
          quote: QuoteStyle::None,
        });
      }
      "•" => {
        up_plug = Some(BlockPlug {
          x: x + width1,
          y,
          quote: QuoteStyle::Quote,
        });
      }
      "/" => {
        up_plug = Some(BlockPlug {
          x: x + width1,
          y,
          quote: QuoteStyle::Closure,
        });
      }
      _ => {}
    }
    width1 += cc(width1, 0)?.len;
  }
  if char(width1, 0)? != "┐" {
    return None;
  };

  let mut height1 = 1;
  while char_is_in(width1, height1, &["│", "├", "@"])? {
    match char(width1, height1)?.as_str() {
      "├" => {
        arg_plugs.push(ArgPlug {
          x: x + width1,
          y: y + height1,
          expand: false,
          ori: Orientation::Right,
        });
      }
      "@" => {
        arg_plugs.push(ArgPlug {
          x: x + width1,
          y: y + height1,
          expand: true,
          ori: Orientation::Right,
        });
      }
      _ => {}
    }
    height1 += 1;
  }
  if char(width1, height1)? != "┘" {
    return None;
  };

  let mut under_width1 = code.right_x(x, y + height1)? - x;
  while char_is_in(under_width1, height1, &["─", "┬", "@"])? {
    match char(under_width1, height1)?.as_str() {
      "┬" => {
        arg_plugs.push(ArgPlug {
          x: x + under_width1,
          y: y + height1,
          expand: false,
          ori: Orientation::Down,
        });
      }
      "@" => {
        arg_plugs.push(ArgPlug {
          x: x + under_width1,
          y: y + height1,
          expand: true,
          ori: Orientation::Down,
        });
      }
      _ => {}
    }
    under_width1 += cc(under_width1, height1)?.len;
  }
  if char(0, height1)? != "└" || under_width1 != width1 {
    return None;
  };

  let mut under_height1 = 1;
  while char_is_in(0, under_height1, &["│", "┤", "@"])? {
    if char(0, under_height1)? == "┤" {
      arg_plugs.push(ArgPlug {
        x,
        y: y + under_height1,
        expand: false,
        ori: Orientation::Left,
      });
    } else if char(0, under_height1)? == "@" {
      arg_plugs.push(ArgPlug {
        x,
        y: y + under_height1,
        expand: true,
        ori: Orientation::Left,
      });
    }
    under_height1 += 1;
  }
  if under_height1 != height1 {
    return None;
  };

  let mut proc_name = "".to_owned();

  for inside_y in 1..height1 {
    proc_name += code.get_slice_of_line(x, x + width1, y + inside_y)?.trim();
    proc_name += "\n";
  }

  arg_plugs.sort_by(|a, b| {
    if a.x != b.x {
      a.x.cmp(&b.x)
    } else if a.x == x {
      a.y.cmp(&b.y)
    } else if a.x == x + width1 {
      b.y.cmp(&a.y)
    } else {
      Ordering::Equal
    }
  });

  Some(CompilingBlock {
    proc_name: proc_name.trim().to_owned(),
    args: vec![],
    x,
    y,
    width: width1 + cc(width1, 0)?.len,
    height: height1 + 1,
    block_plug: up_plug,
    arg_plugs,
  })
}

pub fn find_blocks(splited_code: &SplitedCode, config: &CompileConfig) -> Vec<CompilingBlock> {
  let mut blocks: Vec<CompilingBlock> = vec![];

  for y in 0..splited_code.len_y() {
    for x in splited_code.enumurate_x(y) {
      if let Some(b) = find_a_block(splited_code, x, y, config) {
        blocks.push(b);
      }
    }
  }

  blocks
}

fn find_next_edge(code: &SplitedCode, x: &usize, y: &usize, ori: &Orientation) -> Result<EdgeFragment, EdgeFragment> {
  let update_and_check =
    |new_x: usize, new_y: usize, up: &str, left: &str, right: &str, down: &str| -> Result<EdgeFragment, EdgeFragment> {
      let cc = code.get(new_x, new_y).ok_or(EdgeFragment {
        x: new_x,
        y: new_y,
        ori: *ori,
      })?;

      let t = cc.char;
      if t == up {
        Ok(EdgeFragment {
          x: new_x,
          y: new_y,
          ori: Orientation::Up,
        })
      } else if t == left {
        Ok(EdgeFragment {
          x: new_x,
          y: new_y,
          ori: Orientation::Left,
        })
      } else if t == right {
        Ok(EdgeFragment {
          x: new_x,
          y: new_y,
          ori: Orientation::Right,
        })
      } else if t == down {
        Ok(EdgeFragment {
          x: new_x,
          y: new_y,
          ori: Orientation::Down,
        })
      } else {
        Err(EdgeFragment {
          x: new_x,
          y: new_y,
          ori: *ori,
        })
      }
    };

  match ori {
    Orientation::Up => update_and_check(*x, y - 1, "│", "┐", "┌", ""),
    Orientation::Left => update_and_check(code.left_x(*x, *y).unwrap_or(*x - 1), *y, "└", "─", "", "┌"),
    Orientation::Right => update_and_check(
      code.right_x(*x, *y).unwrap_or(*x + code.get(*x, *y).unwrap().len),
      *y,
      "┘",
      "",
      "─",
      "┐",
    ),
    Orientation::Down => update_and_check(*x, y + 1, "", "┘", "└", "│"),
  }
}

pub fn connect_blocks(
  code: &SplitedCode,
  blocks: &mut [CompilingBlock],
  config: &CompileConfig,
) -> Result<CompilingBlock, String> {
  let blocks_cloned = blocks.to_owned();

  let head_candinates: Vec<usize> = blocks
    .iter()
    .enumerate()
    .filter_map(|(i, block)| if block.block_plug.is_some() { None } else { Some(i) })
    .collect();

  if head_candinates.len() != 1 {
    return Err(format!(
      "The code must have exact one block which has no block-plug. Found {}.",
      head_candinates.len()
    ));
  }
  let head = head_candinates[0];

  for (block_index, block) in blocks.iter_mut().enumerate() {
    for arg_plug in block.arg_plugs.iter() {
      let ArgPlug { x, y, ori, .. } = arg_plug;

      let mut mut_x = *x;
      let mut mut_y = *y;
      let mut mut_ori = *ori;

      // Edge構成用
      let mut fragments = Vec::new();

      loop {
        match find_next_edge(code, &mut_x, &mut_y, &mut_ori) {
          Ok(edge) => {
            mut_x = edge.x;
            mut_y = edge.y;
            mut_ori = edge.ori;
            fragments.push(edge);
          }
          Err(edge) => {
            mut_x = edge.x;
            mut_y = edge.y;
            break;
          }
        }
      }

      let (arg_block_index, _) = blocks_cloned
        .iter()
        .enumerate()
        .find(|(_, b)| {
          if let Some(p) = &b.block_plug {
            p.x == mut_x && p.y == mut_y
          } else {
            false
          }
        })
        .ok_or(format!("No block-plug found at ({}, {})", mut_x, mut_y))?;

      block.args.push(Edge {
        block_index_of_arg_plug: block_index,
        arg_plug_info: arg_plug.clone(),
        fragments,
        block_index_of_block_plug: arg_block_index,
      });
    }
  }

  Ok(blocks[head].clone())
}

pub fn split_code(code: &Vec<String>, config: &CompileConfig) -> SplitedCode {
  let mut splited_code = SplitedCode::new();

  for line in code {
    for char in line.split("") {
      if !char.is_empty() {
        splited_code.append(char, &config.char_width);
      }
    }

    splited_code.new_line();
  }

  splited_code
}

pub(crate) fn compile(code: Vec<String>, config: &CompileConfig) -> Result<Block, String> {
  let splited_code = split_code(&code, config);

  let mut blocks = find_blocks(&splited_code, config);

  let head_compiling_block = connect_blocks(&splited_code, &mut blocks, config)?;

  Ok(head_compiling_block.to_block(&blocks))
}

#[cfg(test)]
mod tests {
  use crate::{
    compile::{
      find_a_block, find_blocks, ArgPlug, BlockPlug, CodeCharacter, CompileConfig, CompilingBlock, Edge, EdgeFragment,
      Orientation, SplitedCode,
    },
    structs::{Block, QuoteStyle},
  };

  use super::{compile, connect_blocks, split_code, CharWidthMode};

  impl CompileConfig {
    pub const DEFAULT: CompileConfig = CompileConfig {
      char_width: CharWidthMode::Mono,
    };
  }

  #[test]
  fn test_split_code() {
    let code = vec![" ┌┐".to_owned()];
    let splited = split_code(&code, &CompileConfig::DEFAULT);
    let target = SplitedCode {
      body: vec![
        vec![
          CodeCharacter {
            char: " ".to_owned(),
            x: 0,
            len: 1,
          },
          CodeCharacter {
            char: "┌".to_owned(),
            x: 1,
            len: 1,
          },
          CodeCharacter {
            char: "┐".to_owned(),
            x: 2,
            len: 1,
          },
        ],
        vec![],
      ],
    };
    assert_eq!(splited, target);
  }
  #[test]
  fn test_split_code_cjk() {
    let mut config = CompileConfig::DEFAULT.clone();
    config.char_width = crate::compile::CharWidthMode::Full;

    let code = vec![" ┌┐".to_owned()];
    let splited = split_code(&code, &config);
    let target = SplitedCode {
      body: vec![
        vec![
          CodeCharacter {
            char: " ".to_owned(),
            x: 0,
            len: 1,
          },
          CodeCharacter {
            char: "┌".to_owned(),
            x: 1,
            len: 2,
          },
          CodeCharacter {
            char: "┐".to_owned(),
            x: 3,
            len: 2,
          },
        ],
        vec![],
      ],
    };
    assert_eq!(splited, target);
  }

  #[test]
  fn test_find_a_block() {
    let config = CompileConfig::DEFAULT;

    let block = find_a_block(
      &split_code(
        &vec![
          "               ".to_owned(),
          "    ┌─────┐    ".to_owned(),
          "    │ abc │    ".to_owned(),
          "    └─────┘    ".to_owned(),
          "               ".to_owned(),
        ],
        &config,
      ),
      4,
      1,
      &config,
    );

    assert_eq!(
      Some(CompilingBlock {
        proc_name: "abc".to_string(),
        x: 4,
        y: 1,
        width: 7,
        height: 3,
        block_plug: None,
        arg_plugs: vec![],
        args: vec![]
      }),
      block
    );
  }

  #[test]
  fn test_find_a_block_cjk() {
    let mut config = CompileConfig::DEFAULT.clone();
    config.char_width = crate::compile::CharWidthMode::Full;

    let block = find_a_block(
      &split_code(
        &vec![
          "               ".to_owned(),
          "    ┌───┐    ".to_owned(),
          "    │ abc  │    ".to_owned(),
          "    └───┘    ".to_owned(),
          "               ".to_owned(),
        ],
        &config,
      ),
      4,
      1,
      &config,
    );

    assert_eq!(
      Some(CompilingBlock {
        proc_name: "abc".to_string(),
        x: 4,
        y: 1,
        width: 10,
        height: 3,
        block_plug: None,
        arg_plugs: vec![],
        args: vec![]
      }),
      block
    );
  }

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
    config.char_width = crate::compile::CharWidthMode::Half;

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
    config.char_width = crate::compile::CharWidthMode::Full;

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
  fn check_find_blocks() {
    let config = CompileConfig::DEFAULT;

    let blocks = find_blocks(
      &split_code(
        &vec![
          "    ".to_owned(),
          "    ┌───────┐".to_owned(),
          "    │ abc   │    ".to_owned(),
          "    └───┬───┘   ".to_owned(),
          "    ┌───┴──┐".to_owned(),
          "    │ def  │    ".to_owned(),
          "    └──────┘   ".to_owned(),
        ],
        &config,
      ),
      &config,
    );

    assert_eq!(
      vec![
        CompilingBlock {
          proc_name: "abc".to_owned(),
          x: 4,
          y: 1,
          width: 9,
          height: 3,
          block_plug: None,
          arg_plugs: vec![ArgPlug {
            x: 8,
            y: 3,
            expand: false,
            ori: Orientation::Down
          }],
          args: vec![]
        },
        CompilingBlock {
          proc_name: "def".to_owned(),
          x: 4,
          y: 4,
          width: 8,
          height: 3,
          block_plug: Some(BlockPlug {
            x: 8,
            y: 4,
            quote: QuoteStyle::None
          }),
          arg_plugs: vec![],
          args: vec![]
        }
      ],
      blocks
    );
  }

  #[test]
  fn check_find_blocks_half() {
    let mut config = CompileConfig::DEFAULT.clone();
    config.char_width = crate::compile::CharWidthMode::Half;

    let blocks = find_blocks(
      &split_code(
        &vec![
          "    ".to_owned(),
          "    ┌───────┐".to_owned(),
          "    │ あc   │    ".to_owned(),
          "    └───┬───┘   ".to_owned(),
          "    ┌───┴──┐".to_owned(),
          "    │ いf  │    ".to_owned(),
          "    └──────┘   ".to_owned(),
        ],
        &config,
      ),
      &config,
    );

    assert_eq!(
      vec![
        CompilingBlock {
          proc_name: "あc".to_owned(),
          x: 4,
          y: 1,
          width: 9,
          height: 3,
          block_plug: None,
          arg_plugs: vec![ArgPlug {
            x: 8,
            y: 3,
            expand: false,
            ori: Orientation::Down
          }],
          args: vec![]
        },
        CompilingBlock {
          proc_name: "いf".to_owned(),
          x: 4,
          y: 4,
          width: 8,
          height: 3,
          block_plug: Some(BlockPlug {
            x: 8,
            y: 4,
            quote: QuoteStyle::None
          }),
          arg_plugs: vec![],
          args: vec![]
        }
      ],
      blocks
    );
  }

  #[test]
  fn check_find_blocks_cjk() {
    let mut config = CompileConfig::DEFAULT.clone();
    config.char_width = crate::compile::CharWidthMode::Full;

    let blocks = find_blocks(
      &split_code(
        &vec![
          "    ".to_owned(),
          "    ┌────┐".to_owned(),
          "    │ abc    │    ".to_owned(),
          "    └─┬──┘   ".to_owned(),
          "    ┌─┴─┐".to_owned(),
          "    │ def  │    ".to_owned(),
          "    └───┘   ".to_owned(),
        ],
        &config,
      ),
      &config,
    );

    assert_eq!(
      vec![
        CompilingBlock {
          proc_name: "abc".to_owned(),
          x: 4,
          y: 1,
          width: 12,
          height: 3,
          block_plug: None,
          arg_plugs: vec![ArgPlug {
            x: 8,
            y: 3,
            expand: false,
            ori: Orientation::Down
          }],
          args: vec![]
        },
        CompilingBlock {
          proc_name: "def".to_owned(),
          x: 4,
          y: 4,
          width: 10,
          height: 3,
          block_plug: Some(BlockPlug {
            x: 8,
            y: 4,
            quote: QuoteStyle::None
          }),
          arg_plugs: vec![],
          args: vec![]
        }
      ],
      blocks
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

  #[test]
  fn two_connect() {
    let splited_code = split_code(
      &vec![
        "    ".to_owned(),
        "    ┌───────┐".to_owned(),
        "    │ abc   │    ".to_owned(),
        "    └───┬───┘   ".to_owned(),
        "        │   ".to_owned(),
        "    ┌───┴──┐".to_owned(),
        "    │ def  │    ".to_owned(),
        "    └──────┘   ".to_owned(),
      ],
      &CompileConfig::DEFAULT,
    );

    let mut blocks = find_blocks(&splited_code, &CompileConfig::DEFAULT);
    let head = connect_blocks(&splited_code, &mut blocks, &CompileConfig::DEFAULT).unwrap();

    assert_eq!(
      head,
      CompilingBlock {
        proc_name: "abc".to_owned(),
        x: 4,
        y: 1,
        width: 9,
        height: 3,
        block_plug: None,
        arg_plugs: vec![ArgPlug {
          x: 8,
          y: 3,
          expand: false,
          ori: Orientation::Down
        }],
        args: vec![Edge {
          block_index_of_arg_plug: 0,
          arg_plug_info: ArgPlug {
            x: 8,
            y: 3,
            expand: false,
            ori: Orientation::Down
          },
          fragments: vec![EdgeFragment {
            x: 8,
            y: 4,
            ori: Orientation::Down
          }],
          block_index_of_block_plug: 1
        }]
      }
    )
  }
}
