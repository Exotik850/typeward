mod bytes_input;
mod read_input;
mod shared;
mod str_input;

use std::borrow::Cow;

pub use read_input::{ReadInput, ReadInputBuf};

use crate::error::ParseResult;
use stable_pattern::Pattern;

/// Abstract parser input that supports consuming textual prefixes.
///
/// This trait is implemented for common borrowed input forms:
/// - `&str`
/// - `&[u8]` (UTF-8)
/// - [`ReadInput`] for read-backed byte buffers
pub trait Input<'a>: Copy + Sized {
    /// Returns the number of remaining units in the input.
    fn input_len(self) -> usize;

    /// Trims leading whitespace and returns the remaining input.
    fn trim_start(self) -> Self;

    /// Returns true when no input remains.
    fn is_empty(self) -> bool;

    /// Returns the remaining input as a debug-friendly string.
    fn display(self) -> Cow<'a, str>;

    /// Attempts to strip a literal prefix and returns remaining input on success.
    fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>>;

    /// Finds the next occurrence of `needle` and returns input starting at that match.
    fn find(self, needle: &str) -> ParseResult<Option<Self>>;

    /// Returns the input segment from `self` up to (but excluding) `end`.
    ///
    /// Both values must be suffixes of the same original input.
    fn slice_to(self, end: Self) -> ParseResult<Self>;

    /// Takes the first character and returns it with the remaining input.
    fn take_char(self) -> ParseResult<Option<(char, Self)>>;

    /// Consumes a maximal prefix matching `predicate`.
    ///
    /// Returns the matched prefix and remaining input. The matched prefix may be
    /// empty when no leading characters satisfy the predicate.
    fn take_while<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy;

    /// Consumes a prefix until `predicate` matches.
    ///
    /// Returns the consumed prefix and remaining input. The consumed prefix may be
    /// empty when the predicate matches at the start of the input.
    fn take_till<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy;
}

#[cfg(test)]
mod tests;
