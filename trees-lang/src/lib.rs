#![allow(dead_code)]
#![allow(unused_imports)]
#![warn(missing_docs)]
//! This crate provides official support for Trees code.
//! Now only exports parsing features, but it planned to export other parts in the future.

/// This module contains parser of the code.
///
/// Parse steps are:
/// 1. `fn split_code()` - Split code into characters.
/// 2. `fn find_blocks()` - Find blocks from the code.
/// 3. `fn connect_blocks()` - Connect blocks by following edges.
///
/// # Example:
/// ```
/// use trees_lang::compile::{split_code, find_blocks, connect_blocks, CompileConfig};
///
/// let splited_code = split_code(
///   &vec![
///     "    ".to_owned(),
///     "    ┌───────┐".to_owned(),
///     "    │ abc   │    ".to_owned(),
///     "    └───┬───┘   ".to_owned(),
///     "        │   ".to_owned(),
///     "    ┌───┴──┐".to_owned(),
///     "    │ def  │    ".to_owned(),
///     "    └──────┘   ".to_owned(),
///   ],
///   &CompileConfig::DEFAULT,
/// );
///  
/// let mut blocks = find_blocks(&splited_code, &CompileConfig::DEFAULT);
/// let head = connect_blocks(&splited_code, &mut blocks, &CompileConfig::DEFAULT).unwrap();
///
/// assert_eq!(head.proc_name, "abc".to_owned());
/// ```
pub mod compile;
