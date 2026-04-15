use std::borrow::Cow;

use crate::{error::ParseResult, input::Input, parse::Parse};

impl<'a, I> Parse<'a, I> for &'a str
where
    I: Input<'a>,
{
    fn parse(input: I) -> ParseResult<(Self, I)> {
        if input.is_empty() {
            return Err(crate::error::ParseError::custom(
                "expected string, found end of input",
            ));
        }

        let (token, rest) = input.take_while(|c| !c.is_whitespace())?;
        if token.is_empty() {
            return Err(crate::error::ParseError::custom(format!(
                "expected string, found '{}'",
                input.display()
            )));
        }

        Ok((token, rest))
    }
}

impl<'a, I> Parse<'a, I> for String
where
    I: Input<'a>,
{
    fn parse(input: I) -> ParseResult<(Self, I)> {
        let (s, rest) = <&str>::parse(input)?;
        Ok((s.to_string(), rest))
    }
}

impl<'a, I> Parse<'a, I> for Cow<'a, str>
where
    I: Input<'a>,
{
    fn parse(input: I) -> ParseResult<(Self, I)> {
        let (result, rest) = <&str>::parse(input)?;
        Ok((Cow::Borrowed(result), rest))
    }
}

#[cfg(test)]
mod tests {
    use crate::{input::TokenStream, primitives::prelude::AlphaString};

    use super::*;

    #[test]
    fn test_cow_parse() {
        let input = "hello world";
        let (cow, rest) = Cow::parse(input).unwrap();
        assert_eq!(cow, "hello");
        assert_eq!(rest, " world");
    }

    #[test]
    fn test_alpha_parse_token_stream() {
        let tokens = ["hello", "world"];
        let (word, rest) = AlphaString::parse(TokenStream::new(&tokens)).unwrap();
        assert_eq!(word.value, "hello");
        assert_eq!(rest.as_slice(), &["world"]);
    }
}
