use crate::error::ParseResult;
use crate::literals::{LParen, RParen, LBracket, RBracket, LBrace, RBrace};
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
pub struct Delimited<S, E, I> {
    /// The parsed start delimiter token.
    pub start: S,
    /// The parsed end delimiter token.
    pub end: E,
    /// The parsed inner content.
    pub inner: I,
}

pub type Parenthesized<I> = Delimited<LParen, RParen, I>;
pub type Bracketed<I> = Delimited<LBracket, RBracket, I>;
pub type Braced<I> = Delimited<LBrace, RBrace, I>;

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
    fn parse(input: In) -> ParseResult<(Self, In)> {
        let (start, remaining) = S::parse(input)?;
        let remaining = remaining.trim_start()?;
        let (inner, remaining) = I::parse(remaining)?;
        let remaining = remaining.trim_start()?;
        let (end, remaining) = E::parse(remaining)?;

        Ok((Delimited { start, end, inner }, remaining))
    }
}

#[cfg(test)]
mod tests {
    use crate::primitives::*;

    use super::*;

    #[test]
    fn test_delimited_parentheses() {
        let input = "(42)";
        let (result, rest) = Parenthesized::<i64>::parse(input).unwrap();
        assert_eq!(result.inner, 42);
        assert_eq!(rest, "");
    }

    #[test]
    fn test_delimited_with_trailing() {
        let input = "(42) rest";
        let (result, rest) = Parenthesized::<i64>::parse(input).unwrap();
        assert_eq!(result.inner, 42);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_delimited_brackets() {
        let input = "[hello]";
        let (result, rest) = Bracketed::<AlphaString>::parse(input).unwrap();
        assert_eq!(&(*result.inner), "hello");
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
}
