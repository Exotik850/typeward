use crate::error::{ParseError, ParseResult};
use crate::input::{Input, TokenStream};
use std::{
    cmp::Reverse,
    mem,
    sync::{OnceLock, RwLock},
};

/// Domain for offset tracking units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseOffsetDomain {
    /// Byte offsets (`&str`, `&[u8]`).
    Bytes,
    /// Element offsets (`TokenStream`).
    Tokens,
}

/// Anchor metadata used to compute absolute parser offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseOffsetAnchor {
    domain: ParseOffsetDomain,
    start: usize,
    len: usize,
    unit_size: usize,
}

impl ParseOffsetAnchor {
    #[must_use]
    pub const fn new(
        domain: ParseOffsetDomain,
        start: usize,
        len: usize,
        unit_size: usize,
    ) -> Self {
        Self {
            domain,
            start,
            len,
            unit_size,
        }
    }
}

/// Input support required for global offset tracking without thread-local state.
///
/// This trait lives outside of [`Input`] so parser position tracking remains an
/// opt-in concern managed by parse orchestration.
pub trait ParseOffsetInput<'a>: Input<'a> {
    /// Returns an anchor describing the current input view.
    fn parse_offset_anchor(self) -> ParseOffsetAnchor;

    /// Returns the absolute offset from a root anchor when this input is inside it.
    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize>;
}

fn parse_offset_registry() -> &'static RwLock<Vec<ParseOffsetAnchor>> {
    static REGISTRY: OnceLock<RwLock<Vec<ParseOffsetAnchor>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(Vec::new()))
}

struct ParseOffsetScope {
    anchor: ParseOffsetAnchor,
    pushed: bool,
}

impl ParseOffsetScope {
    fn enter(anchor: ParseOffsetAnchor) -> Self {
        let mut roots = parse_offset_registry()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        roots.push(anchor);
        Self {
            anchor,
            pushed: true,
        }
    }

    fn enter_if_missing<'a, I>(input: I) -> Self
    where
        I: ParseOffsetInput<'a>,
    {
        let anchor = input.parse_offset_anchor();
        let has_scope = {
            let roots = parse_offset_registry()
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            roots
                .iter()
                .copied()
                .any(|root| input.parse_offset_from(root).is_some())
        };

        if has_scope {
            Self {
                anchor,
                pushed: false,
            }
        } else {
            Self::enter(anchor)
        }
    }
}

impl Drop for ParseOffsetScope {
    fn drop(&mut self) {
        if !self.pushed {
            return;
        }

        let mut roots = parse_offset_registry()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(index) = roots.iter().rposition(|root| *root == self.anchor) {
            roots.remove(index);
        }
    }
}

pub(crate) fn with_parse_offset_scope<'a, I, R>(input: I, parse: impl FnOnce() -> R) -> R
where
    I: ParseOffsetInput<'a>,
{
    let _scope = ParseOffsetScope::enter(input.parse_offset_anchor());
    parse()
}

pub(crate) fn with_parse_offset_scope_if_missing<'a, I, R>(input: I, parse: impl FnOnce() -> R) -> R
where
    I: ParseOffsetInput<'a>,
{
    let _scope = ParseOffsetScope::enter_if_missing(input);
    parse()
}

pub(crate) fn current_parse_offset<'a, I>(input: I) -> usize
where
    I: ParseOffsetInput<'a>,
{
    let roots = parse_offset_registry()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    roots
        .iter()
        .copied()
        .filter_map(|root| input.parse_offset_from(root).map(|offset| (offset, root)))
        .min_by_key(|(_, root)| (root.len, Reverse(root.start)))
        .map_or(0, |(offset, _)| offset)
}

impl<'a> ParseOffsetInput<'a> for &'a str {
    fn parse_offset_anchor(self) -> ParseOffsetAnchor {
        ParseOffsetAnchor::new(
            ParseOffsetDomain::Bytes,
            self.as_ptr() as usize,
            self.len(),
            1,
        )
    }

    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize> {
        if root.domain != ParseOffsetDomain::Bytes || root.unit_size != 1 {
            return None;
        }

        let current_start = self.as_ptr() as usize;
        let current_end = current_start.saturating_add(self.len());
        let root_end = root.start.saturating_add(root.len);

        if current_start < root.start || current_end > root_end {
            return None;
        }

        Some(current_start.saturating_sub(root.start))
    }
}

impl<'a> ParseOffsetInput<'a> for &'a [u8] {
    fn parse_offset_anchor(self) -> ParseOffsetAnchor {
        ParseOffsetAnchor::new(
            ParseOffsetDomain::Bytes,
            self.as_ptr() as usize,
            self.len(),
            1,
        )
    }

    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize> {
        if root.domain != ParseOffsetDomain::Bytes || root.unit_size != 1 {
            return None;
        }

        let current_start = self.as_ptr() as usize;
        let current_end = current_start.saturating_add(self.len());
        let root_end = root.start.saturating_add(root.len);

        if current_start < root.start || current_end > root_end {
            return None;
        }

        Some(current_start.saturating_sub(root.start))
    }
}

impl<'a, T> ParseOffsetInput<'a> for TokenStream<'a, T>
where
    T: AsRef<str>,
{
    fn parse_offset_anchor(self) -> ParseOffsetAnchor {
        ParseOffsetAnchor::new(
            ParseOffsetDomain::Tokens,
            self.as_slice().as_ptr() as usize,
            self.as_slice().len(),
            mem::size_of::<T>(),
        )
    }

    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize> {
        if root.domain != ParseOffsetDomain::Tokens || root.unit_size != mem::size_of::<T>() {
            return None;
        }

        let current_tokens = self.as_slice();
        let current_start = current_tokens.as_ptr() as usize;
        let current_len = current_tokens.len();

        if root.unit_size == 0 {
            if current_start != root.start || current_len > root.len {
                return None;
            }
            return Some(root.len.saturating_sub(current_len));
        }

        let byte_diff = current_start.checked_sub(root.start)?;
        if byte_diff % root.unit_size != 0 {
            return None;
        }

        let consumed = byte_diff / root.unit_size;
        if consumed.saturating_add(current_len) > root.len {
            return None;
        }

        Some(consumed)
    }
}

/// A trait for types that can be parsed from an abstract input.
///
/// This is the main trait that structs should implement to become parseable.
/// The lifetime parameter `'a` represents the lifetime of the borrowed input.
///
/// The second generic parameter defaults to `&str`, which keeps string parsing
/// ergonomic while allowing additional input forms such as `&[u8]` and token
/// slices.
pub trait Parse<'a, I: Input<'a> = &'a str>: Sized {
    /// Parse a value from the input.
    ///
    /// Returns the parsed value and the remaining unconsumed input.
    fn parse(input: I) -> ParseResult<(Self, I)>;
}

impl<'a, I> Parse<'a, I> for ()
where
    I: Input<'a>,
{
    fn parse(input: I) -> ParseResult<(Self, I)> {
        Ok(((), input))
    }
}

impl<'a, I, T> Parse<'a, I> for Option<T>
where
    I: Input<'a>,
    T: Parse<'a, I>,
{
    fn parse(input: I) -> ParseResult<(Self, I)> {
        match T::parse(input) {
            Ok((value, remaining)) => Ok((Some(value), remaining)),
            Err(_) => Ok((None, input)),
        }
    }
}

/// Convenience function to parse complete input, ensuring everything is consumed.
///
/// This function parses the input and verifies that all meaningful content
/// has been consumed (trailing whitespace is allowed).
pub fn parse_complete<'a, T: Parse<'a>>(input: &'a str) -> ParseResult<T> {
    parse_complete_input::<_, T>(input)
}

/// Convenience function to parse complete string input into a spanned result.
///
/// This is equivalent to `parse_complete_input::<_, Span<T>>(input)`.
pub fn parse_complete_spanned<'a, T: Parse<'a>>(
    input: &'a str,
) -> ParseResult<crate::combinators::span::Span<T>> {
    parse_complete_input::<_, crate::combinators::span::Span<T>>(input)
}

/// Convenience function to parse and fully consume an abstract input.
///
/// Leading and trailing whitespace handling is delegated to the input type.
pub fn parse_complete_input<'a, I, T>(input: I) -> ParseResult<T>
where
    I: ParseOffsetInput<'a>,
    T: Parse<'a, I>,
{
    with_parse_offset_scope(input, || {
        let (result, remaining) = T::parse(input)?;
        let remaining = remaining.trim_start();
        if remaining.is_empty() {
            Ok(result)
        } else {
            let start = current_parse_offset(remaining);
            let span = crate::error::SourceSpan::new(start, start + remaining.input_len());
            Err(ParseError::custom(format!(
                "unexpected trailing input: '{}'",
                remaining.display()
            ))
            .with_span(span))
        }
    })
}

/// Convenience function to parse and fully consume input into a spanned result.
pub fn parse_complete_input_spanned<'a, I, T>(
    input: I,
) -> ParseResult<crate::combinators::span::Span<T>>
where
    I: ParseOffsetInput<'a>,
    T: Parse<'a, I>,
{
    parse_complete_input::<_, crate::combinators::span::Span<T>>(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combinators::{span::Span, ws::Ws};
    use crate::error::SourceSpan;
    use crate::input::TokenStream;
    use crate::literals::KwNull;

    // A simple test parser that consumes "hello"
    struct HelloParser;
    impl<'a, I> Parse<'a, I> for HelloParser
    where
        I: Input<'a>,
    {
        fn parse(input: I) -> ParseResult<(Self, I)> {
            if let Some(remaining) = input.strip_prefix("hello")? {
                Ok((HelloParser, remaining))
            } else {
                Err(ParseError::custom("expected 'hello'"))
            }
        }
    }

    #[test]
    fn test_parse_complete_success() {
        let result = parse_complete::<HelloParser>("hello").unwrap();
        let _ = result; // HelloParser has no fields to check
    }

    #[test]
    fn test_parse_complete_with_whitespace() {
        let result = parse_complete::<HelloParser>("hello   ").unwrap();
        let _ = result;
    }

    #[test]
    fn test_parse_complete_spanned() {
        let result = parse_complete_spanned::<HelloParser>("hello").unwrap();
        assert_eq!(result.span.start, 0);
        assert_eq!(result.span.end, 5);
    }

    #[test]
    fn test_parse_complete_trailing_input() {
        let result = parse_complete::<HelloParser>("hello world");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_complete_input_bytes() {
        let result = parse_complete_input::<_, HelloParser>(b"hello".as_slice()).unwrap();
        let _ = result;
    }

    #[test]
    fn test_parse_complete_input_tokens() {
        let tokens = ["hello"];
        let result = parse_complete_input::<_, HelloParser>(TokenStream::new(&tokens)).unwrap();
        let _ = result;
    }

    #[test]
    fn test_parse_complete_input_spanned_tokens() {
        let tokens = ["hello"];
        let result =
            parse_complete_input_spanned::<_, HelloParser>(TokenStream::new(&tokens)).unwrap();
        assert_eq!(result.span.start, 0);
        assert_eq!(result.span.end, 1);
    }

    #[test]
    fn test_parse_complete_tracks_nested_span_offset() {
        type Parser = crate::and!(Ws<i64>, Span<Ws<KwNull>>);
        let result = parse_complete::<Parser>("42 null").unwrap();
        let null = result.right;
        assert_eq!(null.span, SourceSpan::new(2, 7));
    }

    #[test]
    fn test_parse_complete_tracks_error_offset() {
        type Parser = crate::and!(Ws<i64>, Ws<KwNull>);
        let err = parse_complete::<Parser>("42 nope").unwrap_err();
        assert_eq!(err.span(), Some(SourceSpan::point(3)));
    }

    #[test]
    fn test_parse_offset_tracking_is_thread_safe() {
        type Parser = crate::and!(Ws<i64>, Span<Ws<KwNull>>);
        let strs = ["1 null", "42 null", "123 null"];

        std::thread::scope(|s| {
            let handles = [
                s.spawn(|| parse_complete::<Parser>(&strs[0]).unwrap().right.span),
                s.spawn(|| parse_complete::<Parser>(&strs[1]).unwrap().right.span),
                s.spawn(|| parse_complete::<Parser>(&strs[2]).unwrap().right.span),
            ];

            let spans: Vec<_> = handles
                .into_iter()
                .map(|handle| handle.join().expect("threaded parse should succeed"))
                .collect();

            assert_eq!(spans[0], SourceSpan::new(1, 6));
            assert_eq!(spans[1], SourceSpan::new(2, 7));
            assert_eq!(spans[2], SourceSpan::new(3, 8));
        });
    }
}
