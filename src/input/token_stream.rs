use std::borrow::Cow;

use super::{Input, shared};
use crate::error::{ParseError, ParseResult};
use stable_pattern::Pattern;

/// Borrowed token-stream input wrapper.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TokenStream<'a, T> {
    tokens: &'a [T],
}

impl<T> Clone for TokenStream<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for TokenStream<'_, T> {}

impl<'a, T> TokenStream<'a, T> {
    #[must_use]
    pub fn new(tokens: &'a [T]) -> Self {
        Self { tokens }
    }

    #[must_use]
    pub fn as_slice(self) -> &'a [T] {
        self.tokens
    }
}

impl<'a, T> From<&'a [T]> for TokenStream<'a, T> {
    fn from(value: &'a [T]) -> Self {
        Self::new(value)
    }
}

impl<'a, T> TokenStream<'a, T>
where
    T: AsRef<str>,
{
    fn split_first(self) -> Option<(&'a str, Self)> {
        self.tokens
            .split_first()
            .map(|(first, rest)| (first.as_ref(), Self::new(rest)))
    }

    fn partial_token_error() -> ParseError {
        ParseError::custom("cannot consume partial token from token stream input")
    }
}

impl<'a, T> Input<'a> for TokenStream<'a, T>
where
    T: AsRef<str>,
{
    fn input_len(self) -> usize {
        self.tokens.len()
    }

    fn trim_start(self) -> Self {
        let start = self
            .tokens
            .iter()
            .position(|token| !token.as_ref().trim_start().is_empty())
            .unwrap_or(self.tokens.len());
        Self::new(&self.tokens[start..])
    }

    fn is_empty(self) -> bool {
        self.tokens.is_empty()
    }

    fn display(self) -> Cow<'a, str> {
        self.tokens
            .iter()
            .take(8)
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(" ")
            .into()
    }

    fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>> {
        Ok(self
            .tokens
            .split_first()
            .and_then(|(first, rest)| (first.as_ref() == prefix).then(|| Self::new(rest))))
    }

    fn find(self, needle: &str) -> ParseResult<Option<Self>> {
        Ok(self
            .tokens
            .iter()
            .position(|token| token.as_ref() == needle)
            .map(|idx| Self::new(&self.tokens[idx..])))
    }

    fn slice_to(self, end: Self) -> ParseResult<Self> {
        let consumed = shared::checked_consumed_len(self.tokens.len(), end.tokens.len(), "token")?;
        Ok(Self::new(&self.tokens[..consumed]))
    }

    fn take_char(self) -> ParseResult<Option<(char, Self)>> {
        let Some((token, rest)) = self.split_first() else {
            return Ok(None);
        };

        let mut chars = token.chars();
        let Some(ch) = chars.next() else {
            return Err(ParseError::custom(
                "encountered empty token in token stream",
            ));
        };
        if chars.next().is_some() {
            return Err(ParseError::custom(
                "cannot parse char from multi-character token stream element",
            ));
        }

        Ok(Some((ch, rest)))
    }

    fn take_while<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy,
    {
        let Some((token, rest)) = self.split_first() else {
            return Ok(("", self));
        };

        if shared::take_while_prefix_len(token, predicate) != token.len() {
            return Err(Self::partial_token_error());
        }

        Ok((token, rest))
    }

    fn take_till<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy,
    {
        let Some((token, rest)) = self.split_first() else {
            return Ok(("", self));
        };

        let (_, remainder) = shared::split_take_till(token, predicate);
        if !remainder.is_empty() {
            return Err(Self::partial_token_error());
        }

        Ok((token, rest))
    }

    fn empty() -> Self {
        Self::new(&[])
    }
}
