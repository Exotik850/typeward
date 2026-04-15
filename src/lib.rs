//! **typeward** — A parser combinator library that builds parsers by combining
//! smaller parsers within the type system.
//!
//! This crate is an alternative to `bon` for creating structs that can be parsed
//! by composing type-level parsers. Define tokens as types, implement `Parse` for
//! your structs, and combine them using generic combinators like [`Delimited`].
//!
//! # Module Overview
//!
//! - [`error`] — Parse error types and [`ParseResult`]
//! - [`token`] — Token traits ([`Token`], [`ParseToken`], [`ValueToken`])
//! - [`parse`] — The core [`Parse`] trait and [`parse_complete`] helper
//! - [`primitives`] — `Parse` implementations for built-in types
//! - [`combinators`] — Parser combinators like [`Delimited`]

pub mod combinators;
pub mod error;
pub mod literals;
pub mod parse;
pub mod primitives;
pub mod token;

// ============================================================================
// Prelude
// ============================================================================

/// Commonly used items for building parsers.
///
/// Import this module with `use typeward::prelude::*;` to get access
/// to the core traits, types, and functions.
pub mod prelude {
    pub use crate::combinators::prelude::*;
    pub use crate::error::{ParseError, ParseResult};
    pub use crate::parse::{Parse, parse_complete};
    pub use crate::token::Token;
}
