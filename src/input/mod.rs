mod bytes_input;
mod read_input_buf;
mod read_input_stream;
mod shared;
mod str_input;

use std::borrow::Cow;

pub use read_input_buf::{ReadInput, ReadInputBuf};
pub use read_input_stream::{ReadInputStream, ReadInputStreamInput};

use crate::error::{ParseError, ParseResult};
use stable_pattern::Pattern;

/// Abstract parser input that supports consuming textual prefixes.
///
/// This trait is implemented for both borrowed and streaming input forms.
///
/// Implementations may return owned text fragments from `take_while` and
/// `take_till` when borrowed slices are not stable (for example, streaming
/// inputs backed by reusable buffers).
///
/// Built-in implementations exist for common borrowed input forms:
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
    /// borrowed or owned depending on the input implementation.
    fn take_while<P>(self, predicate: P) -> ParseResult<(Cow<'a, str>, Self)>
    where
        P: for<'b> Pattern<'b> + Copy;

    /// Consumes a prefix until `predicate` matches.
    ///
    /// Returns the consumed prefix and remaining input. The consumed prefix may be
    /// borrowed or owned depending on the input implementation.
    fn take_till<P>(self, predicate: P) -> ParseResult<(Cow<'a, str>, Self)>
    where
        P: for<'b> Pattern<'b> + Copy;
}

/// Marker trait for inputs whose matched text can be borrowed from the source.
///
/// Parsers that yield borrowed values (for example `&'a str`) should require
/// this trait instead of plain [`Input`].
pub trait BorrowInput<'a>: Input<'a> {
    /// Consumes a maximal prefix matching `predicate` and returns a borrowed match.
    fn take_while_borrowed<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: for<'b> Pattern<'b> + Copy;

    /// Consumes a prefix until `predicate` matches and returns a borrowed match.
    fn take_till_borrowed<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: for<'b> Pattern<'b> + Copy;
}

/// Converts a textual fragment produced by an [`Input`] into a parser output type.
///
/// This enables a single parser implementation to support both borrowed outputs
/// on [`BorrowInput`] and owned outputs on streaming-friendly [`Input`] types.
pub trait FromInputStr<'a, I>: AsRef<str> + Sized
where
    I: Input<'a>,
{
    fn from_input_str(value: Cow<'a, str>) -> ParseResult<Self>;
}

impl<'a, I> FromInputStr<'a, I> for String
where
    I: Input<'a>,
{
    fn from_input_str(value: Cow<'a, str>) -> ParseResult<Self> {
        Ok(value.into_owned())
    }
}

impl<'a, I> FromInputStr<'a, I> for Cow<'a, str>
where
    I: Input<'a>,
{
    fn from_input_str(value: Cow<'a, str>) -> ParseResult<Self> {
        Ok(value)
    }
}

impl<'a, I> FromInputStr<'a, I> for &'a str
where
    I: BorrowInput<'a>,
{
    fn from_input_str(value: Cow<'a, str>) -> ParseResult<Self> {
        match value {
            Cow::Borrowed(value) => Ok(value),
            Cow::Owned(_) => Err(ParseError::custom(
                "borrowed output requires borrow-capable input",
            )),
        }
    }
}

#[cfg(test)]
mod tests;
