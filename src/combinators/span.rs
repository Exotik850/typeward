use crate::{
    error::{ParseResult, SourceSpan},
    parse::{
        Parse, ParseOffsetContext, ParseOffsetInput, current_parse_offset, with_parse_offset_scope,
        with_parse_offset_scope_if_missing,
    },
};

/// Wraps a parser output with the input span it consumed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Default)]
pub struct Span<P> {
    pub inner: P,
    pub span: SourceSpan,
}

impl<P> Span<P> {
    #[must_use]
    pub fn new(inner: P, span: SourceSpan) -> Self {
        Self { inner, span }
    }

    #[must_use]
    pub fn into_inner(self) -> P {
        self.inner
    }

    #[must_use]
    pub fn inner(&self) -> &P {
        &self.inner
    }

    #[must_use]
    pub fn into_parts(self) -> (P, SourceSpan) {
        (self.inner, self.span)
    }

    #[must_use]
    pub fn map<T, F>(self, map: F) -> Span<T>
    where
        F: FnOnce(P) -> T,
    {
        Span {
            inner: map(self.inner),
            span: self.span,
        }
    }
}

impl<P> std::ops::Deref for Span<P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<P: PartialEq<P>> PartialEq<P> for Span<P> {
    fn eq(&self, other: &P) -> bool {
        &self.inner == other
    }
}

impl<'a, I, P> Parse<'a, I> for Span<P>
where
    I: ParseOffsetInput<'a>,
    P: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(input: I, context: &mut ParseOffsetContext) -> ParseResult<(Self, I)> {
        with_parse_offset_scope_if_missing(context, input, |context| {
            let start = current_parse_offset(context, input);

            match P::parse_with_context(input, context) {
                Ok((parsed, rest)) => {
                    let consumed = input.input_len().saturating_sub(rest.input_len());
                    let span = SourceSpan::from_start_len(start, consumed);
                    Ok((Self::new(parsed, span), rest))
                }
                Err(err) => Err(err.with_span(start)),
            }
        })
    }
}

/// Extension trait for parsing any pattern with span capture.
///
/// This allows ergonomic parsing without manually writing `Span<P>` in call sites.
pub trait SpanExt: Sized {
    /// The parser output type wrapped with span metadata.
    type Spanned;

    /// Parse `Self` and return the parsed value wrapped with its consumed span.
    fn parse_spanned<'a, I>(input: I) -> ParseResult<(Self::Spanned, I)>
    where
        I: ParseOffsetInput<'a>,
        Self::Spanned: Parse<'a, I>,
    {
        let mut context = ParseOffsetContext::new();
        with_parse_offset_scope(&mut context, input, |context| {
            Self::Spanned::parse_with_context(input, context)
        })
    }
}

impl<T> SpanExt for T {
    type Spanned = Span<T>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combinators::ws::Ws;
    use crate::literals::Null;

    #[test]
    fn span_wraps_successful_parse() {
        let (spanned, rest) = Span::<Ws<i64>>::parse("  -12x").unwrap();
        assert_eq!(spanned.inner.into_inner(), -12);
        // Span covers the full input the parser consumed, including any
        // whitespace trimmed by the wrapped `Ws<_>`.
        assert_eq!(spanned.span, 0..5);
        assert_eq!(rest, "x");
    }

    #[test]
    fn span_attaches_error_location() {
        let err = Span::<i64>::parse("abc").unwrap_err();
        assert_eq!(err.span(), Some(SourceSpan::point(0)));
    }

    #[test]
    fn extension_trait_parses_spanned() {
        let (spanned, rest) = bool::parse_spanned("true false").unwrap();
        assert_eq!(spanned, true);
        assert_eq!(spanned.span, 0..4);
        assert_eq!(rest, " false");
    }

    #[test]
    fn parse_spanned_tracks_nested_offsets_without_complete_parse() {
        type Parser = crate::and!(Ws<i64>, Span<Ws<Null>>);
        let (spanned, rest) = Parser::parse_spanned("42 null!").unwrap();
        assert_eq!(spanned.inner.right.span, 2..7);
        assert_eq!(rest, "!");
    }

    #[test]
    fn parse_spanned_tracks_byte_offsets_without_complete_parse() {
        type Parser = crate::and!(Ws<i64>, Span<Ws<Null>>);
        let (spanned, rest) = Parser::parse_spanned(b"42 null!".as_slice()).unwrap();

        assert_eq!(spanned.span, 0..7);
        assert_eq!(spanned.inner.right.span, 2..7);
        assert_eq!(rest, b"!");
    }
}
