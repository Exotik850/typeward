use crate::error::ParseResult;
use crate::parse::Parse;

// ============================================================================
// Delimited Parser
// ============================================================================

/// A parser that matches content wrapped between start and end tokens.
///
/// This is a generic combinator that parses a start delimiter, then inner content,
/// then an end delimiter. It's useful for parsing parenthesized expressions,
/// bracketed lists, quoted strings, and similar delimited structures.
///
/// # Type Parameters
/// * `S` - The start delimiter token type
/// * `E` - The end delimiter token type
/// * `I` - The inner content parser type
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Delimited<S, E, I> {
    /// The parsed start delimiter token.
    pub start: S,
    /// The parsed end delimiter token.
    pub end: E,
    /// The parsed inner content.
    pub inner: I,
}

pub type Padded<E, I> = Delimited<E, E, I>;

/// A parser that matches content wrapped between start and end tokens without
/// automatic whitespace trimming.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DelimitedExact<S, E, I> {
    pub start: S,
    pub end: E,
    pub inner: I,
}

pub type PaddedExact<E, I> = DelimitedExact<E, E, I>;

impl<S, E, I> DelimitedExact<S, E, I> {
    pub fn map_inner<J>(self, f: impl FnOnce(I) -> J) -> DelimitedExact<S, E, J> {
        DelimitedExact {
            start: self.start,
            end: self.end,
            inner: f(self.inner),
        }
    }

    pub fn into_inner(self) -> I {
        self.inner
    }

    pub fn inner(&self) -> &I {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }
}

impl<S, E, I> Delimited<S, E, I> {
    /// Map the inner content of the delimited parser
    pub fn map_inner<J>(self, f: impl FnOnce(I) -> J) -> Delimited<S, E, J> {
        Delimited {
            start: self.start,
            end: self.end,
            inner: f(self.inner),
        }
    }

    pub fn into_inner(self) -> I {
        self.inner
    }

    pub fn inner(&self) -> &I {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }
}

impl<'a, In, S, E, I> Parse<'a, In> for Delimited<S, E, I>
where
    In: crate::input::Input<'a>,
    S: Parse<'a, In>,
    E: Parse<'a, In>,
    I: Parse<'a, In>,
{
    #[inline]
    fn parse_with_context(
        input: In,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, In)> {
        let (start, remaining) = S::parse_with_context(input.trim_start(), context)?;
        let remaining = remaining.trim_start();
        let (inner, remaining) = I::parse_with_context(remaining, context)?;
        let remaining = remaining.trim_start();
        let (end, remaining) = E::parse_with_context(remaining, context)?;

        Ok((Delimited { start, end, inner }, remaining))
    }
}

impl<'a, In, S, E, I> Parse<'a, In> for DelimitedExact<S, E, I>
where
    In: crate::parse::ParseOffsetInput<'a>,
    S: Parse<'a, In>,
    E: crate::token::Token + Parse<'a, In>,
    I: Parse<'a, In>,
{
    #[inline]
    fn parse_with_context(
        input: In,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, In)> {
        let (start, remaining) = S::parse_with_context(input, context)?;

        let end_match = remaining.find(E::VALUE)?;
        let (inner_input, close_start) = if let Some(match_input) = end_match {
            let inner_val = remaining.slice_to(match_input)?;
            (inner_val, match_input)
        } else {
            let start_offset = crate::parse::current_parse_offset(context, remaining);
            return Err(crate::error::ParseError::custom(format!(
                "delimited exact: missing closing token `{}`",
                E::VALUE
            ))
            .with_span(start_offset));
        };

        let (inner, inner_rest) = I::parse_with_context(inner_input, context)?;
        if !inner_rest.is_empty() {
            let start = crate::parse::current_parse_offset(context, inner_rest);
            let span =
                crate::error::SourceSpan::new(start, start.saturating_add(inner_rest.input_len()));
            let preview = crate::error::preview_input(
                inner_rest.display().as_ref(),
                crate::error::DEFAULT_INPUT_PREVIEW,
            );
            return Err(crate::error::ParseError::custom(format!(
                "unexpected trailing input inside delimited exact: '{preview}'"
            ))
            .with_span(span));
        }

        let (end, remaining) = E::parse_with_context(close_start, context)?;

        Ok((DelimitedExact { start, end, inner }, remaining))
    }
}

#[cfg(test)]
mod tests {
    use crate::literals::*;
    use crate::primitives::prelude::AlphaString;

    use super::*;

    #[test]
    fn test_delimited_parentheses() {
        let input = "(42)";
        let (result, rest) = Delimited::<LParen, RParen, i64>::parse(input).unwrap();
        assert_eq!(result.inner, 42);
        assert_eq!(rest, "");
    }

    #[test]
    fn test_delimited_with_trailing() {
        let input = "(42) rest";
        let (result, rest) = Delimited::<LParen, RParen, i64>::parse(input).unwrap();
        assert_eq!(result.inner, 42);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_delimited_brackets() {
        let input = "[hello]";
        let (result, rest) = Delimited::<LBracket, RBracket, AlphaString>::parse(input).unwrap();
        assert_eq!(result.inner, "hello");
        assert_eq!(rest, "");
    }

    #[test]
    fn test_delimited_nested() {
        let input = "((42))";
        let (outer, rest) =
            Delimited::<LParen, RParen, Delimited<LParen, RParen, i64>>::parse(input).unwrap();
        assert_eq!(outer.inner.inner, 42);
        assert_eq!(rest, "");
    }

    #[test]
    fn test_delimited_missing_start() {
        let input = "42)";
        let result = Delimited::<LParen, RParen, i64>::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_delimited_missing_end() {
        let input = "(42";
        let result = Delimited::<LParen, RParen, i64>::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_delimited_exact_preserves_inner_whitespace() {
        let input = "(  42)";
        let (result, rest) =
            DelimitedExact::<LParen, RParen, crate::primitives::str::TakeTillToken<RParen>>::parse(
                input,
            )
            .unwrap();
        assert_eq!(result.inner, "  42");
        assert_eq!(rest, "");
    }
}
