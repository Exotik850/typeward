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
//! - [`token`] — Token trait ([`Token`])
//! - [`input`] — Input abstraction trait ([`Input`])
//! - [`parse`] — The core [`Parse`] trait and parse helpers
//! - [`primitives`] — `Parse` implementations for built-in types
//! - [`combinators`] — Parser combinators like [`Delimited`]
//! - [`collections`] — Repetition/collection combinators like [`Many0`] and [`Repeat`]

extern crate self as typeward;

#[cfg(feature = "macros")]
pub use typeward_macros::Parse;

pub mod collections;
pub mod combinators;
pub mod error;
pub mod input;
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
    pub use crate::collections::prelude::*;
    pub use crate::combinators::prelude::*;
    pub use crate::error::{ParseError, ParseResult, SourceSpan};
    pub use crate::input::{Input, TokenStream};
    pub use crate::literals::*;
    pub use crate::parse::{
        Parse, ParseOffsetContext, ParseOffsetInput, parse_complete, parse_complete_input,
        parse_complete_input_spanned, parse_complete_spanned,
    };
    pub use crate::primitives::prelude::*;
    pub use crate::token::Token;
    pub use crate::{and, new_and, or, or_match, unpack_and};
    #[cfg(feature = "macros")]
    pub use typeward_macros::Parse;
}
