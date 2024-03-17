use std::cmp::Ordering;

use crate::structs::Block;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompilingBlock {
  proc_name: String,
  x: usize,
  y: usize,
  width: usize,
  height: usize,
  block_plug: Option<BlockPlug>,
  arg_plugs: Vec<Plug>,
  args: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Plug {
  x: usize,
  y: usize,
  ori: Orientation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BlockPlug {
  x: usize,
  y: usize,
  quote: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Orientation {
  Up,
  Left,
  Right,
  Down,
}

impl CompilingBlock {
  fn to_block(&self, blocks: &Vec<CompilingBlock>) -> Block {
    Block {
      proc_name: self.proc_name.clone(),
      args: self.args.clone().into_iter().map(|t| Box::new(blocks[t].to_block(blocks))).collect(),
      quote: if let Some(p) = &self.block_plug { p.quote } else { false },
    }
  }
}

fn find_a_block(code: &Vec<Vec<String>>, x: usize, y: usize) -> Option<CompilingBlock> {
  macro_rules! char {
    ($dx:expr, $dy:expr) => {{
      code.get(y + $dy)?.get(x + $dx)?
    }};
  }

  let mut up_plug = None;
  let mut arg_plugs: Vec<_> = vec![];

  if char!(0, 0) != "┌" {
    return None;
  };
  // 右回り
  // 1から始める
  let mut width1 = 1;
  while char!(width1, 0) == "─" || char!(width1, 0) == "┴" || char!(width1, 0) == "•" {
    if char!(width1, 0) == "┴" {
      up_plug = Some(BlockPlug {
        x: x + width1,
        y,
        quote: false,
      });
    } else if char!(width1, 0) == "•" {
      up_plug = Some(BlockPlug { x: x + width1, y, quote: true });
    }
    width1 += 1;
  }
  if char!(width1, 0) != "┐" {
    return None;
  };

  let mut height1 = 1;
  while char!(width1, height1) == "│" || char!(width1, height1) == "├" {
    if char!(width1, height1) == "├" {
      arg_plugs.push(Plug {
        x: x + width1,
        y: y + height1,
        ori: Orientation::Right,
      });
    }
    height1 += 1;
  }
  if char!(width1, height1) != "┘" {
    return None;
  };

  let mut under_width1 = 1;
  while char!(under_width1, height1) == "─" || char!(under_width1, height1) == "┬" {
    if char!(under_width1, height1) == "┬" {
      arg_plugs.push(Plug {
        x: x + under_width1,
        y: y + height1,
        ori: Orientation::Down,
      });
    }
    under_width1 += 1;
  }
  if char!(0, height1) != "└" || under_width1 != width1 {
    return None;
  };

  let mut under_height1 = 1;
  while char!(0, under_height1) == "│" || char!(0, under_height1) == "┤" {
    if char!(0, under_height1) == "┤" {
      arg_plugs.push(Plug {
        x,
        y: y + under_height1,
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
    proc_name += &code[y + inside_y].get(x + 1..x + width1)?.join("").trim();
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
    width: width1 + 1,
    height: height1 + 1,
    block_plug: up_plug,
    arg_plugs,
  })
}

fn find_blocks(code_splited: &Vec<Vec<String>>) -> Vec<CompilingBlock> {
  let mut blocks: Vec<CompilingBlock> = vec![];

  for y in 0..code_splited.len() {
    for x in 0..code_splited[y].len() {
      if let Some(b) = find_a_block(&code_splited, x, y) {
        blocks.push(b);
      }
    }
  }

  blocks
}

fn find_next_edge(code: &Vec<Vec<String>>, x: &usize, y: &usize, ori: &Orientation) -> Result<Plug, Plug> {
  let update_and_check = |new_x: usize, new_y: usize, up: &str, left: &str, right: &str, down: &str| -> Result<Plug, Plug> {
    let t = code
      .get(new_y)
      .and_then(|l| l.get(new_x))
      .ok_or(Plug {
        x: new_x,
        y: new_y,
        ori: ori.clone(),
      })?
      .as_str();
    if t == up {
      Ok(Plug {
        x: new_x,
        y: new_y,
        ori: Orientation::Up,
      })
    } else if t == left {
      Ok(Plug {
        x: new_x,
        y: new_y,
        ori: Orientation::Left,
      })
    } else if t == right {
      Ok(Plug {
        x: new_x,
        y: new_y,
        ori: Orientation::Right,
      })
    } else if t == down {
      Ok(Plug {
        x: new_x,
        y: new_y,
        ori: Orientation::Down,
      })
    } else {
      Err(Plug {
        x: new_x,
        y: new_y,
        ori: ori.clone(),
      })
    }
  };

  match ori {
    Orientation::Up => update_and_check(*x, y - 1, "│", "┐", "┌", ""),
    Orientation::Left => update_and_check(x - 1, *y, "└", "─", "", "┌"),
    Orientation::Right => update_and_check(x + 1, *y, "┘", "", "─", "┐"),
    Orientation::Down => update_and_check(*x, y + 1, "", "┘", "└", "│"),
  }
}

fn connect_blocks(code: &Vec<Vec<String>>, blocks: &Vec<CompilingBlock>) -> Result<Block, String> {
  let mut blocks_clone = blocks.clone();
  let head_candinates: Vec<usize> = blocks.into_iter().enumerate().filter_map(|(i, block)| if block.block_plug.is_some() { None } else { Some(i) }).collect();

  if head_candinates.len() != 1 {
    return Err(format!(
      "The code must have exact one block which has no block-plug. Found {}.",
      head_candinates.len()
    ));
  }
  let head = head_candinates[0];

  for block in blocks_clone.iter_mut() {
    for Plug { x, y, ori } in block.arg_plugs.iter() {
      let mut mut_x = *x;
      let mut mut_y = *y;
      let mut mut_ori = ori.clone();

      loop {
        match find_next_edge(code, &mut_x, &mut_y, &mut_ori) {
          Ok(edge) => {
            mut_x = edge.x;
            mut_y = edge.y;
            mut_ori = edge.ori;
          }
          Err(edge) => {
            mut_x = edge.x;
            mut_y = edge.y;
            break;
          }
        }
      }

      let (index, _) = blocks
        .into_iter()
        .enumerate()
        .find(|(_, b)| {
          if let Some(p) = &b.block_plug {
            p.x == mut_x.clone() && p.y == mut_y.clone()
          } else {
            false
          }
        })
        .ok_or(format!("No block-plug found at ({}, {})", mut_x, mut_y))?;

      block.args.push(index);
    }
  }

  Ok(blocks_clone[head].to_block(&blocks_clone.clone()))
}

fn split_code(code: &Vec<String>) -> Vec<Vec<String>> {
  code.iter().map(|s| s.split("").into_iter().filter_map(|s| if s == "" { None } else { Some(s.to_owned()) }).collect()).collect()
}

pub fn compile(code: Vec<String>) -> Result<Block, String> {
  let code_splited: Vec<Vec<String>> = split_code(&code);

  let blocks = find_blocks(&code_splited);

  connect_blocks(&code_splited, &blocks)
}

#[cfg(test)]
mod tests {
  use crate::{
    compile::{find_blocks, BlockPlug, CompilingBlock, Orientation, Plug},
    structs::Block,
  };

  use super::{compile, split_code};

  #[test]
  fn test_split_code() {
    let code = vec![" ┌┐".to_owned()];
    let splited = split_code(&code);
    let target: Vec<Vec<String>> = vec![vec![" ".to_owned(), "┌".to_owned(), "┐".to_owned()]];
    assert_eq!(splited, target);
  }

  #[test]
  fn one_block() {
    let block = compile(vec![
      "               ".to_owned(),
      "    ┌─────┐    ".to_owned(),
      "    │ abc │    ".to_owned(),
      "    └─────┘    ".to_owned(),
      "               ".to_owned(),
    ]);

    assert_eq!(
      Ok(Block {
        proc_name: "abc".to_owned(),
        args: vec![],
        quote: false
      }),
      block
    );
  }

  #[test]
  fn one_block_complex() {
    let block = compile(vec![
      "    ".to_owned(),
      "    ┌───────┐".to_owned(),
      "    │ abc   │    ".to_owned(),
      "    │ def g │  ".to_owned(),
      "    └───────┘   ".to_owned(),
      "             ".to_owned(),
    ]);

    assert_eq!(
      Ok(Block {
        proc_name: "abc\ndef g".to_owned(),
        args: vec![],
        quote: false
      }),
      block
    );
  }
  #[test]
  fn check_find_blocks() {
    let blocks = find_blocks(&split_code(&vec![
      "    ".to_owned(),
      "    ┌───────┐".to_owned(),
      "    │ abc   │    ".to_owned(),
      "    └───┬───┘   ".to_owned(),
      "    ┌───┴──┐".to_owned(),
      "    │ def  │    ".to_owned(),
      "    └──────┘   ".to_owned(),
    ]));

    assert_eq!(
      vec![
        CompilingBlock {
          proc_name: "abc".to_owned(),
          x: 4,
          y: 1,
          width: 9,
          height: 3,
          block_plug: None,
          arg_plugs: vec![Plug {
            x: 8,
            y: 3,
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
          block_plug: Some(BlockPlug { x: 8, y: 4, quote: false }),
          arg_plugs: vec![],
          args: vec![]
        }
      ],
      blocks
    );
  }

  #[test]
  fn two_connect() {
    let block = compile(vec![
      "    ".to_owned(),
      "    ┌───────┐".to_owned(),
      "    │ abc   │    ".to_owned(),
      "    └───┬───┘   ".to_owned(),
      "    ┌───┴──┐".to_owned(),
      "    │ def  │    ".to_owned(),
      "    └──────┘   ".to_owned(),
    ]);

    assert_eq!(
      Ok(Block {
        proc_name: "abc".to_owned(),
        args: vec![Box::new(Block {
          proc_name: "def".to_owned(),
          args: vec![],
          quote: false
        })],
        quote: false
      }),
      block
    );
  }
}
