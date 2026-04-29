use crate::{error::ParseResult, parse::Parse};

/// A parser that matches `P` without consuming any input.
/// 
/// Useful for lookahead or peeking at upcoming input without advancing the parser state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Peek<P> {
    value: P,
}

impl<'a, I, P> Parse<'a, I> for Peek<P>
where
    I: crate::input::Input<'a>,
    P: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let (value, _) = P::parse_with_context(input, context)?;
        Ok((Peek { value }, input))
    }
}

#[cfg(test)]
mod tests {
    use crate::{combinators::and::And, primitives::filtered::AlphaString};

    use super::*;

    #[test]
    fn test_peek() {
        let input = "42";
        let (result, rest) = Peek::<i64>::parse(input).unwrap();
        assert_eq!(result.value, 42);
        assert_eq!(rest, "42");
    }

    #[test]
    fn test_peek_and() {
        let input = "42abc";
        let (result, rest) = And::<i64, Peek<AlphaString>>::parse(input).unwrap();
        assert_eq!(result.left, 42);
        assert_eq!(result.right.value, "abc");
        assert_eq!(rest, "abc");
    }
}
