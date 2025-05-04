mod errors;

use std::cmp::Ordering;

use errors::CompileError;
use unicode_width::UnicodeWidthStr;

/// Stores settings used during code compilation.
///
/// This struct is used to configure how character widths are interpreted during the
/// compilation process for now.
///
/// # Example
///
/// ```rust
/// use trees_lang::compile::{CompileConfig, CharWidthMode};
///
/// let config = CompileConfig {
///     char_width: CharWidthMode::Full,
/// };
/// ```
///
/// This configuration can then be passed to compilation functions such as `split_code`
/// or `find_blocks` to control how character positions and widths are calculated.
#[derive(Debug, Clone)]
pub struct CompileConfig {
  /// Character width mode used during compilation.
  pub char_width: CharWidthMode,
}

impl CompileConfig {
  /// Default setup for compile
  pub const DEFAULT: CompileConfig = CompileConfig {
    char_width: CharWidthMode::Mono,
  };
}

/// Determines how character widths are calculated during code parsing.
///
/// This enum controls how ambiguous-width characters (such as those in East Asian scripts)
/// are interpreted during layout calculations. It affects how each character contributes to
/// the horizontal spacing of visual elements.
#[derive(Debug, Clone)]
pub enum CharWidthMode {
  /// Treat all characters as width 1.
  Mono,
  /// Treat ambiguous-width characters as half-width.
  Half,
  /// Treat ambiguous-width characters as full-width.
  Full,
}

/// A single character in the source code with layout metadata.
///
/// Used internally to track the character, its x-position, and width in a parsed line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeCharacter {
  /// The character itself.
  pub char: String,
  /// The x position (column) of the character.
  pub x: usize,
  /// The display width of the character.
  pub len: usize,
}

/// A code that has been split into lines and characters for processing
///
/// # Example
/// ```
/// use trees_lang::compile::{SplitedCode, split_code, CharWidthMode, CompileConfig, CodeCharacter};
///
/// // Split each character
/// let mut splited_code: SplitedCode = split_code(
///   &vec![" ┌─".to_owned()],
///   &CompileConfig {
///     char_width: CharWidthMode::Mono
///   }
/// );
///
/// // Get each characters' position
/// // (It is useful if char_width is not Mono)
/// assert_eq!(splited_code.enumurate_x(0).collect::<Vec<_>>(), vec![0, 1, 2]);
///
/// // Get char of target position
/// assert_eq!(splited_code.get(1, 0), Some(CodeCharacter {
///   char: "┌".to_owned(), x: 1, len: 1
/// }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitedCode {
  body: Vec<Vec<CodeCharacter>>,
}

impl SplitedCode {
  /// Retrieves a specific character at position `(x, y)` from the split code.
  ///
  /// If the position is out of bounds or does not contain a character, returns `None`.
  pub fn get(&self, x: usize, y: usize) -> Option<CodeCharacter> {
    self.body.get(y)?.iter().find(|cc| cc.x == x).cloned()
  }

  /// Retrieves a slice of characters from the specified line between `x_min_exclusive` and `x_max_exclusive`.
  ///
  /// If the range is invalid or out of bounds, returns `None`.
  pub fn get_slice_of_line(&self, x_min_exclusive: usize, x_max_exclusive: usize, y: usize) -> Option<String> {
    let (mut index, _) = self.body.get(y)?.iter().enumerate().find(|(_index, cc)| cc.x == x_min_exclusive)?;
    let mut return_str = "".to_string();

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

  /// Retrieves the position of the character to the left of the given `(x, y)` position.
  ///
  /// If there is no character to the left, returns `None`.
  pub fn left_x(&self, x: usize, y: usize) -> Option<usize> {
    let index =
      self.body.get(y)?.iter().enumerate().find_map(|(index, cc)| if cc.x == x { Some(index) } else { None })?;
    self.body.get(y)?.get(index - 1).map(|cc| cc.x)
  }

  /// Retrieves the position of the character to the right of the given `(x, y)` position.
  ///
  /// If there is no character to the right, returns `None`.
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
  fn new_line(&mut self) {
    self.body.push(vec![]);
  }

  /// Returns the number of lines in the `SplitedCode`.
  pub fn len_y(&self) -> usize {
    self.body.len()
  }

  /// Enumerates the x-positions of all characters in the given line `y`.\
  pub fn enumurate_x(&self, y: usize) -> Box<dyn std::iter::Iterator<Item = usize> + '_> {
    Box::new(self.body[y].iter().map(|cc| cc.x))
  }
}

/// A parsed visual block in the code, including its position, size, and connections.
///
/// This is an intermediate representation used during compilation before converting to a `Block`.
///
/// # Example
/// ```rust
/// use trees_lang::compile::{CompilingBlock, ArgPlug, BlockPlug, Orientation,
///                             CompileConfig, CharWidthMode, split_code, find_blocks, connect_blocks};
///
/// let code = vec![
///     "    ".to_owned(),
///     "    ┌───────┐".to_owned(),
///     "    │ abc   │    ".to_owned(),
///     "    └───┬───┘   ".to_owned(),
///     "    ┌───┴──┐".to_owned(),
///     "    │ def  │    ".to_owned(),
///     "    └──────┘   ".to_owned(),
/// ];
///
/// let config = CompileConfig {
///   char_width: CharWidthMode::Mono
/// };
/// let splited_code = split_code(&code, &config);
/// let mut blocks = find_blocks(&splited_code, &config);
/// let head_block: CompilingBlock = connect_blocks(&splited_code, &mut blocks, &config).unwrap();
///
/// assert_eq!(head_block.proc_name, "abc");
/// assert_eq!(head_block.arg_plugs.len(), 1);
/// assert_eq!(head_block.args.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilingBlock {
  /// Procedure name in the block.
  pub proc_name: String,
  /// X position (column) of the top-left of the block.
  pub x: usize,
  /// Y position (row) of the top-left of the block.
  pub y: usize,
  /// Width of the block.
  pub width: usize,
  /// Height of the block.
  pub height: usize,
  /// Optional block plug for connecting this block to others.
  pub block_plug: Option<BlockPlug>,
  /// Edge connecting block-plug of this block to another block.
  ///
  /// This is setted by `connect_blocks` function. Before that, it is empty.
  pub connect_from: Option<Edge>,
  /// Argument plugs for this block.
  pub arg_plugs: Vec<ArgPlug>,
  /// Edges (connections) representing the arguments passed to this block.
  ///
  /// This is setted by `connect_blocks` function. Before that, it is empty.
  pub args: Vec<Edge>,
}

/// An argument plug in a `CompilingBlock`, indicating where an argument can be connected.
///
/// Used for tracking the connection points for arguments in a visual block, including the position and orientation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgPlug {
  /// X position of the plug.
  pub x: usize,
  /// Y position of the plug.
  pub y: usize,
  /// Whether the argument plug supports expansion (variadic).
  pub expand: bool,
  /// Orientation of the plug (direction from which argument connects).
  pub ori: Orientation,
}

/// A fragment of an edge in the code's flow, with position and direction.
///
/// Tracks a specific piece of an edge connection, indicating the direction and location of a flow path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeFragment {
  /// X coordinate of the edge fragment.
  pub x: usize,
  /// Y coordinate of the edge fragment.
  pub y: usize,
  /// Orientation of this edge fragment.
  pub ori: Orientation,
}

/// An edge connecting two blocks, including fragments and plug information.
///
/// Used to describe the connections between blocks and their arguments in a visual code flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
  /// Index of the block that owns the argument plug.
  pub block_index_of_arg_plug: usize,
  /// Information about the argument plug.
  pub arg_plug_info: ArgPlug,
  /// Sequence of fragments composing this edge.
  pub fragments: Vec<EdgeFragment>,
  /// Index of the block that the argument plug is connected to.
  pub block_index_of_block_plug: usize,
}

/// A plug point for a block, where it can be connected to other blocks.
///
/// Tracks the position of a plug and its associated quote style for connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockPlug {
  /// X position of the plug.
  pub x: usize,
  /// Y position of the plug.
  pub y: usize,
  /// The quoting style used at this plug.
  pub quote: QuoteStyle,
}

/// Direction/orientation used for argument and edge routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
  /// The upward direction.
  Up,
  /// The leftward direction.
  Left,
  /// The rightward direction.
  Right,
  /// The downward direction.
  Down,
}

/// Quote style of block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuoteStyle {
  /// Quote
  Quote,
  /// Closure
  Closure,
  /// No quote style applied
  None,
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
    if char_is_in(width1, 0, &["┴", "•", "/"])? {
      if up_plug.is_some() {
        return None;
      }
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
    connect_from: None,
    arg_plugs,
  })
}

/// Finds all the blocks in a given split code according to configuration.
///
/// Returns a vector of detected `CompilingBlock`s.
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

/// Connects the blocks by resolving edges between argument plugs and block plugs.
///
/// Returns the root `CompilingBlock` if successful.
pub fn connect_blocks(
  code: &SplitedCode,
  blocks: &mut [CompilingBlock],
  config: &CompileConfig,
) -> Result<CompilingBlock, CompileError> {
  let blocks_cloned = blocks.to_owned();

  let head_candinates: Vec<usize> = blocks
    .iter()
    .enumerate()
    .filter_map(|(i, block)| if block.block_plug.is_some() { None } else { Some(i) })
    .collect();

  if head_candinates.len() != 1 {
    return Err(CompileError::NonUniqueStartBlock(Box::new(
      errors::NonUniqueStartBlockError {
        candinates: head_candinates.iter().map(|i| blocks[*i].clone()).collect(),
      },
    )));
  }
  let head = head_candinates[0];

  // 借用権をかわすため、connect_fromは後から入れる。
  let mut deferred_connections = Vec::new();

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
        .ok_or(CompileError::DanglingArgEdge(Box::new(errors::DanglingArgEdgeError {
          block_of_arg_plug: block.clone(),
          arg_plug: arg_plug.clone(),
          edge_fragments: fragments.clone(),
          dangling_position: (mut_x, mut_y),
        })))?;

      let connect_edge = Edge {
        block_index_of_arg_plug: block_index,
        arg_plug_info: arg_plug.clone(),
        fragments,
        block_index_of_block_plug: arg_block_index,
      };

      block.args.push(connect_edge.clone());

      // connect_fromをセット
      deferred_connections.push((arg_block_index, connect_edge.clone()));
    }
  }

  for (arg_block_index, connect_edge) in deferred_connections {
    let block = &mut blocks[arg_block_index];
    block.connect_from = Some(connect_edge);
  }

  Ok(blocks[head].clone())
}

/// Splits a list of code lines into a `SplitedCode` representation.
///
/// Each character is measured based on `CharWidthMode`.
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

#[cfg(test)]
mod tests {
  use crate::compile::{
    ArgPlug, BlockPlug, CodeCharacter, CompileConfig, CompilingBlock, Edge, EdgeFragment, Orientation, QuoteStyle,
    SplitedCode,
    errors::{self, CompileError},
    find_a_block, find_blocks,
  };

  use super::{connect_blocks, split_code};

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
        connect_from: None,
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
        connect_from: None,
        arg_plugs: vec![],
        args: vec![]
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
          connect_from: None,
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
          connect_from: None,
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
          connect_from: None,
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
          connect_from: None,
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
          connect_from: None,
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
          connect_from: None,
          arg_plugs: vec![],
          args: vec![]
        }
      ],
      blocks
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

    let arg_edge = Edge {
      block_index_of_arg_plug: 0,
      arg_plug_info: ArgPlug {
        x: 8,
        y: 3,
        expand: false,
        ori: Orientation::Down,
      },
      fragments: vec![EdgeFragment {
        x: 8,
        y: 4,
        ori: Orientation::Down,
      }],
      block_index_of_block_plug: 1,
    };

    assert_eq!(
      head,
      CompilingBlock {
        proc_name: "abc".to_owned(),
        x: 4,
        y: 1,
        width: 9,
        height: 3,
        block_plug: None,
        connect_from: None,
        arg_plugs: vec![ArgPlug {
          x: 8,
          y: 3,
          expand: false,
          ori: Orientation::Down
        }],
        args: vec![arg_edge.clone()]
      }
    );

    assert_eq!(
      blocks[1],
      CompilingBlock {
        proc_name: "def".to_owned(),
        x: 4,
        y: 5,
        width: 8,
        height: 3,
        block_plug: Some(BlockPlug {
          x: 8,
          y: 5,
          quote: QuoteStyle::None
        }),
        connect_from: Some(arg_edge),
        arg_plugs: vec![],
        args: vec![]
      }
    );
  }

  #[test]
  fn error_non_unique_start_block() {
    let code = vec![
      "    ".to_owned(),
      "    ┌───────┐".to_owned(),
      "    │ abc   │    ".to_owned(),
      "    └───────┘   ".to_owned(),
      "    ┌──────┐".to_owned(),
      "    │ def  │    ".to_owned(),
      "    └──────┘   ".to_owned(),
    ];

    let splited_code = split_code(&code, &CompileConfig::DEFAULT);
    let mut blocks = find_blocks(&splited_code, &CompileConfig::DEFAULT);

    let result = connect_blocks(&splited_code, &mut blocks, &CompileConfig::DEFAULT);

    assert_eq!(
      result,
      Err(CompileError::NonUniqueStartBlock(Box::new(
        errors::NonUniqueStartBlockError {
          candinates: vec![blocks[0].clone(), blocks[1].clone()],
        }
      )))
    );
  }

  #[test]
  fn error_dangling_arg_edge() {
    let code = vec![
      "    ".to_owned(),
      "    ┌───────┐".to_owned(),
      "    │ abc   │    ".to_owned(),
      "    └───┬───┘   ".to_owned(),
      "        │   ".to_owned(),
      "               ".to_owned(),
      "    ┌───┴──┐".to_owned(),
      "    │ def  │    ".to_owned(),
      "    └──────┘   ".to_owned(),
    ];

    let splited_code = split_code(&code, &CompileConfig::DEFAULT);
    let mut blocks = find_blocks(&splited_code, &CompileConfig::DEFAULT);

    let result = connect_blocks(&splited_code, &mut blocks, &CompileConfig::DEFAULT);

    assert_eq!(
      result,
      Err(CompileError::DanglingArgEdge(Box::new(errors::DanglingArgEdgeError {
        block_of_arg_plug: blocks[0].clone(),
        arg_plug: blocks[0].arg_plugs[0].clone(),
        edge_fragments: vec![EdgeFragment {
          x: 8,
          y: 4,
          ori: Orientation::Down
        }],
        dangling_position: (8, 5)
      })))
    );
  }

  #[test]
  fn ignore_two_block_plug() {
    let code = vec![
      "    ".to_owned(),
      "    ┌───────┐".to_owned(),
      "    │ abc   │    ".to_owned(),
      "    └───────┘   ".to_owned(),
      "           ".to_owned(),
      "    ┌──┴┴──┐".to_owned(),
      "    │ def  │    ".to_owned(),
      "    └──────┘   ".to_owned(),
    ];

    let splited_code = split_code(&code, &CompileConfig::DEFAULT);
    let blocks = find_blocks(&splited_code, &CompileConfig::DEFAULT);

    assert_eq!(blocks.len(), 1);
  }
}
