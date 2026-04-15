use crate::{error::ParseResult, parse::Parse};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Peek<P> {
    parser: P,
}

impl<'a, P> Parse<'a> for Peek<P>
where
    P: Parse<'a>,
{
    fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
        let (parser, _) = P::parse(input)?;
        Ok((Peek { parser }, input))
    }
}

#[cfg(test)]
mod tests {
    use crate::{combinators::and::And, primitives::AlphaString};

    use super::*;

    #[test]
    fn test_peek() {
        let input = "42";
        let (result, rest) = Peek::<i64>::parse(input).unwrap();
        assert_eq!(result.parser, 42);
        assert_eq!(rest, "42");
    }

    #[test]
    fn test_peek_and() {
        let input = "42abc";
        let (result, rest) = And::<i64, Peek<AlphaString>>::parse(input).unwrap();
        assert_eq!(result.left, 42);
        assert_eq!(&(*result.right.parser), "abc");
        assert_eq!(rest, "abc");
    }
}
