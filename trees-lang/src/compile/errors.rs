use std::{error::Error, fmt};

use super::{ArgPlug, CompilingBlock, EdgeFragment};

#[derive(Debug)]
pub enum CompileError {
  NonUniqueStartBlock(Box<NonUniqueStartBlockError>),
  DanglingArgEdge(Box<DanglingArgEdgeError>),
}

#[derive(Debug)]
pub struct NonUniqueStartBlockError {
  pub candinates: Vec<CompilingBlock>,
}

#[derive(Debug)]
pub struct DanglingArgEdgeError {
  pub block_of_arg_plug: CompilingBlock,
  pub arg_plug: ArgPlug,
  pub edge_fragments: Vec<EdgeFragment>,
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
