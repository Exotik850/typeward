use std::borrow::Cow;

use crate::token::Token;
use crate::{error::ParseResult, input::Input, parse::Parse};

impl<'a, I> Parse<'a, I> for &'a str
where
    I: Input<'a>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        _context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        if input.is_empty() {
            return Err(crate::error::ParseError::custom(
                "expected string, found end of input",
            ));
        }

        let (token, rest) = input.take_while(|c: char| !c.is_whitespace())?;
        if token.is_empty() {
            let preview = crate::error::preview_input(
                input.display().as_ref(),
                crate::error::DEFAULT_INPUT_PREVIEW,
            );
            return Err(crate::error::ParseError::custom(format!(
                "expected string, found '{}'",
                preview
            )));
        }

        Ok((token, rest))
    }
}

impl<'a, I> Parse<'a, I> for String
where
    I: Input<'a>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let (s, rest) = <&str>::parse_with_context(input, context)?;
        Ok((s.to_string(), rest))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TakeTillToken<T, S = String>
where
    S: AsRef<str>,
{
    value: S,
    _marker: std::marker::PhantomData<T>,
}

impl<T, S> TakeTillToken<T, S>
where
    S: AsRef<str>,
{
    #[must_use]
    pub fn into_inner(self) -> S {
        self.value
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_ref()
    }
}

impl<T, S: AsRef<str>> std::ops::Deref for TakeTillToken<T, S> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref()
    }
}

impl<T, S: AsRef<str>, O: AsRef<str>> PartialEq<O> for TakeTillToken<T, S> {
    fn eq(&self, other: &O) -> bool {
        self.value.as_ref() == other.as_ref()
    }
}

impl<'a, I, T, S> Parse<'a, I> for TakeTillToken<T, S>
where
    I: Input<'a>,
    T: Token,
    S: AsRef<str> + From<&'a str>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        _context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let (value, rest) = input.take_till(T::VALUE)?;
        Ok((
            Self {
                value: S::from(value),
                _marker: std::marker::PhantomData,
            },
            rest,
        ))
    }
}

pub type TakeTillTokenStr<'a, T> = TakeTillToken<T, &'a str>;
pub type TakeTillTokenCow<'a, T> = TakeTillToken<T, Cow<'a, str>>;

#[cfg(test)]
mod tests {
    use crate::literals::Comma;
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

    #[test]
    fn test_take_till_token() {
        let (value, rest) = TakeTillToken::<Comma>::parse("hello,world").unwrap();
        assert_eq!(value, "hello");
        assert_eq!(rest, ",world");
    }
}
