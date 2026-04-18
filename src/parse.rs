use crate::error::{ParseError, ParseResult};
use crate::input::{Input, TokenStream};
use crate::prelude::Span;
use std::{cmp::Reverse, mem};

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

/// Input support required for offset tracking through parse context.
///
/// This trait lives outside of [`Input`] so parser position tracking remains an
/// opt-in concern managed by parse orchestration.
pub trait ParseOffsetInput<'a>: Input<'a> {
    /// Returns an anchor describing the current input view.
    fn parse_offset_anchor(self) -> ParseOffsetAnchor;

    /// Returns the absolute offset from a root anchor when this input is inside it.
    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize>;
}

/// Parse-scoped offset context passed through parse calls.
///
/// This context is explicitly owned by the parse entry point and passed to
/// nested parsers, avoiding any global state while remaining thread-safe.
#[derive(Debug, Default, Clone)]
pub struct ParseOffsetContext {
    roots: Vec<ParseOffsetAnchor>,
}

impl ParseOffsetContext {
    #[must_use]
    pub fn new() -> Self {
        Self { roots: Vec::new() }
    }

    fn has_scope_for<'a, I>(&self, input: I) -> bool
    where
        I: ParseOffsetInput<'a>,
    {
        self.roots
            .iter()
            .copied()
            .any(|root| input.parse_offset_from(root).is_some())
    }
}

pub(crate) fn with_parse_offset_scope<'a, I, R>(
    context: &mut ParseOffsetContext,
    input: I,
    parse: impl FnOnce(&mut ParseOffsetContext) -> R,
) -> R
where
    I: ParseOffsetInput<'a>,
{
    context.roots.push(input.parse_offset_anchor());
    let result = parse(context);
    context.roots.pop();
    result
}

pub(crate) fn with_parse_offset_scope_if_missing<'a, I, R>(
    context: &mut ParseOffsetContext,
    input: I,
    parse: impl FnOnce(&mut ParseOffsetContext) -> R,
) -> R
where
    I: ParseOffsetInput<'a>,
{
    if context.has_scope_for(input) {
        parse(context)
    } else {
        with_parse_offset_scope(context, input, parse)
    }
}

pub(crate) fn current_parse_offset<'a, I>(context: &ParseOffsetContext, input: I) -> usize
where
    I: ParseOffsetInput<'a>,
{
    context
        .roots
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
    fn parse(input: I) -> ParseResult<(Self, I)> {
        let mut context = ParseOffsetContext::new();
        Self::parse_with_context(input, &mut context)
    }

    /// Parse a value from the input using an explicit offset context.
    fn parse_with_context(input: I, context: &mut ParseOffsetContext) -> ParseResult<(Self, I)>;
}

impl<'a, I> Parse<'a, I> for ()
where
    I: Input<'a>,
{
    fn parse_with_context(input: I, _context: &mut ParseOffsetContext) -> ParseResult<(Self, I)> {
        Ok(((), input))
    }
}

impl<'a, I, T> Parse<'a, I> for Option<T>
where
    I: Input<'a>,
    T: Parse<'a, I>,
{
    fn parse_with_context(input: I, context: &mut ParseOffsetContext) -> ParseResult<(Self, I)> {
        match T::parse_with_context(input, context) {
            Ok((value, remaining)) => Ok((Some(value), remaining)),
            Err(_) => Ok((None, input)),
        }
    }
}

impl<'a, I, T> Parse<'a, I> for Vec<T>
where
    I: Input<'a>,
    T: Parse<'a, I>,
{
    fn parse_with_context(
        mut input: I,
        context: &mut ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let mut items = Vec::new();
        while let Ok((item, remaining)) = T::parse_with_context(input, context) {
            items.push(item);
            input = remaining;
        }
        Ok((items, input))
    }
}

macro_rules! parse_pointer {
    ($ty:ty $(; $($bound:ident),*)?) => {
        impl<'a, T, I> Parse<'a, I> for $ty
        where
            I: Input<'a>,
            T: Parse<'a, I> $($(+ $bound)*)?,
        {
            fn parse_with_context(
                input: I,
                context: &mut ParseOffsetContext,
            ) -> ParseResult<(Self, I)> {
                let (value, remaining) = T::parse_with_context(input, context)?;
                Ok((Self::from(value), remaining))
            }
        }
    };
}

parse_pointer!(std::rc::Rc<T>);
parse_pointer!(std::sync::Arc<T>);

/// A wrapper type for nested parsing results,
///
/// allows for parsers to return nested structures without losing the ability to implement `Parse` for the inner type.
/// This was chosen instead of a blanket impl over `Box<T: Parse>`
/// since downstream users may want to implement `Parse` for `Box<T>` directly for some types, and this allows them to do so without conflicting with the blanket impl.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Nested<T>(Box<T>);

impl<'a, T, I> Parse<'a, I> for Nested<T>
where
    I: Input<'a>,
    T: Parse<'a, I>,
{
    fn parse_with_context(input: I, context: &mut ParseOffsetContext) -> ParseResult<(Self, I)> {
        let (value, remaining) = T::parse_with_context(input, context)?;
        Ok((Nested(Box::new(value)), remaining))
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
pub fn parse_complete_spanned<'a, T: Parse<'a>>(input: &'a str) -> ParseResult<Span<T>> {
    parse_complete_input::<_, Span<T>>(input)
}

/// Convenience function to parse and fully consume an abstract input.
///
/// Leading and trailing whitespace handling is delegated to the input type.
pub fn parse_complete_input<'a, I, T>(input: I) -> ParseResult<T>
where
    I: ParseOffsetInput<'a>,
    T: Parse<'a, I>,
{
    let mut context = ParseOffsetContext::new();
    with_parse_offset_scope(&mut context, input, |context| {
        let (result, remaining) = T::parse_with_context(input, context)?;
        let remaining = remaining.trim_start();
        if remaining.is_empty() {
            Ok(result)
        } else {
            let start = current_parse_offset(context, remaining);
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
pub fn parse_complete_input_spanned<'a, I, T>(input: I) -> ParseResult<Span<T>>
where
    I: ParseOffsetInput<'a>,
    T: Parse<'a, I>,
{
    parse_complete_input::<_, Span<T>>(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combinators::{span::Span, ws::Ws};
    use crate::error::SourceSpan;
    use crate::input::TokenStream;
    use crate::lit_token;
    use crate::literals::KwNull;

    lit_token!(HelloParser, "hello");

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
    fn test_parse_complete_input_spanned_bytes_uses_byte_offsets() {
        let result = parse_complete_input_spanned::<_, char>("\u{00E9}".as_bytes()).unwrap();
        assert_eq!(result.inner, '\u{00E9}');
        assert_eq!(result.span, SourceSpan::new(0, 2));
    }

    #[test]
    fn test_parse_complete_input_trailing_bytes_reports_byte_span() {
        let err = parse_complete_input::<_, HelloParser>("hello\u{00E9}".as_bytes())
            .err()
            .expect("expected trailing byte input error");
        assert_eq!(err.span(), Some(SourceSpan::new(5, 7)));
    }

    #[test]
    fn test_parse_complete_input_trailing_tokens_reports_token_span() {
        let tokens = ["hello", "world"];
        let err = parse_complete_input::<_, HelloParser>(TokenStream::new(&tokens))
            .err()
            .expect("expected trailing token input error");
        assert_eq!(err.span(), Some(SourceSpan::new(1, 2)));
    }

    #[test]
    fn test_with_parse_offset_scope_if_missing_reuses_existing_scope() {
        let input = "abcdef";
        let sub = &input[2..];
        let mut context = ParseOffsetContext::new();

        with_parse_offset_scope(&mut context, input, |context| {
            let roots_before = context.roots.len();
            with_parse_offset_scope_if_missing(context, sub, |context| {
                assert_eq!(context.roots.len(), roots_before);
                assert_eq!(current_parse_offset(context, sub), 2);
            });
            assert_eq!(context.roots.len(), roots_before);
        });

        assert!(context.roots.is_empty());
    }

    #[test]
    fn test_with_parse_offset_scope_if_missing_pushes_when_unscoped() {
        let input = "abcdef";
        let sub = &input[2..];
        let mut context = ParseOffsetContext::new();

        with_parse_offset_scope_if_missing(&mut context, sub, |context| {
            assert_eq!(context.roots.len(), 1);
            assert_eq!(current_parse_offset(context, sub), 0);
        });

        assert!(context.roots.is_empty());
    }

    #[test]
    fn test_current_parse_offset_prefers_innermost_scope() {
        let input = "abcdef";
        let inner = &input[2..5];
        let inner_tail = &inner[1..];
        let mut context = ParseOffsetContext::new();

        with_parse_offset_scope(&mut context, input, |context| {
            assert_eq!(current_parse_offset(context, inner), 2);

            with_parse_offset_scope(context, inner, |context| {
                assert_eq!(current_parse_offset(context, inner), 0);
                assert_eq!(current_parse_offset(context, inner_tail), 1);
            });

            assert_eq!(current_parse_offset(context, inner_tail), 3);
        });
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
    fn test_parse_complete_tracks_eof_after_partial_consumption() {
        type Parser = crate::and!(char, char);
        let err = parse_complete::<Parser>("a").unwrap_err();
        assert_eq!(err.span(), Some(SourceSpan::point(1)));
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

    #[test]
    fn test_nested_parses_recursive() {
        struct Recursive {
            value: Ws<i64>,
            inner: Option<Nested<Recursive>>,
        }

        impl<'a, I> Parse<'a, I> for Recursive
        where
            I: Input<'a>,
        {
            fn parse_with_context(
                input: I,
                context: &mut ParseOffsetContext,
            ) -> ParseResult<(Self, I)> {
                let (value, remaining) = Ws::<i64>::parse_with_context(input, context)?;
                let (inner, remaining) =
                    Option::<Nested<Recursive>>::parse_with_context(remaining, context)?;
                Ok((Recursive { value, inner }, remaining))
            }
        }

        let input = "1 2 3";
        let (parsed, remaining) = Recursive::parse(input).unwrap();
        assert_eq!(parsed.value, 1);
        let inner1 = parsed.inner.unwrap().0;
        assert_eq!(inner1.value, 2);
        let inner2 = inner1.inner.unwrap().0;
        assert_eq!(inner2.value, 3);
        assert!(inner2.inner.is_none());
        assert!(remaining.trim().is_empty());
    }
}
