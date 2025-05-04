use std::{error::Error, fmt};

use super::{ArgPlug, CompilingBlock, EdgeFragment};

#[derive(Debug, PartialEq, Eq)]
/// Errors that can occur during the compilation process.
pub enum CompileError {
  /// Error indicating that there are multiple blocks or no blocks without a block-plug.
  NonUniqueStartBlock(Box<NonUniqueStartBlockError>),
  /// Error indicating that an argument plug is not connected to any block.
  DanglingArgEdge(Box<DanglingArgEdgeError>),
}

#[derive(Debug, PartialEq, Eq)]
/// Represents an error where there are multiple or no blocks without a block-plug.
pub struct NonUniqueStartBlockError {
  /// A list of candidate blocks (i.e., blocks without a block-plug).
  pub candinates: Vec<CompilingBlock>,
}

#[derive(Debug, PartialEq, Eq)]
/// Represents an error where an argument plug is not connected to any block.
pub struct DanglingArgEdgeError {
  /// The block associated with the argument plug that is dangling.
  pub block_of_arg_plug: CompilingBlock,
  /// The argument plug that is dangling and not connected to any block.
  pub arg_plug: ArgPlug,
  /// A list of edge fragments associated with the dangling argument plug.
  pub edge_fragments: Vec<EdgeFragment>,
  /// The position (x, y) of the dangling edge's endpoint.
  ///
  /// This position is expected to be connected to a block.
  pub dangling_position: (usize, usize),
}

impl fmt::Display for CompileError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      CompileError::NonUniqueStartBlock(err) => write!(
        f,
        "The code must have exact one block which has no block-plug. Found: {}",
        err.candinates.len() // Accessing through Box
      ),
      CompileError::DanglingArgEdge(err) => write!(
        f,
        "The arg-plug on ({}, {}) has an arg-plug which is not connected to any block. Expected position: ({}, {})",
        err.arg_plug.x,
        err.arg_plug.y,
        err.dangling_position.0,
        err.dangling_position.1 // Accessing through Box
      ),
    }
  }
}

impl Error for CompileError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }
}
