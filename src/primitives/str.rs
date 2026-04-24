use std::borrow::Cow;

use crate::token::Token;
use crate::{
    error::{ParseResult, SourceSpan},
    input::{BorrowInput, FromInputStr, Input},
    parse::{Parse, ParseOffsetInput, current_parse_offset},
};

impl<'a, I> Parse<'a, I> for &'a str
where
    I: BorrowInput<'a> + ParseOffsetInput<'a>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        if input.is_empty() {
            let start = current_parse_offset(context, input);
            return Err(
                crate::error::ParseError::custom("expected string, found end of input")
                    .with_span(SourceSpan::point(start)),
            );
        }

        let (token, rest) = input.take_while_borrowed(|c: char| !c.is_whitespace())?;
        if token.is_empty() {
            let start = current_parse_offset(context, input);
            let preview = crate::error::preview_input(
                input.display().as_ref(),
                crate::error::DEFAULT_INPUT_PREVIEW,
            );
            return Err(crate::error::ParseError::custom(format!(
                "expected string, found '{preview}'"
            ))
            .with_span(SourceSpan::point(start)));
        }

        Ok((token, rest))
    }
}

impl<'a, I> Parse<'a, I> for String
where
    I: ParseOffsetInput<'a>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        if input.is_empty() {
            let start = current_parse_offset(context, input);
            return Err(
                crate::error::ParseError::custom("expected string, found end of input")
                    .with_span(SourceSpan::point(start)),
            );
        }

        let (token, rest) = input.take_while(|c: char| !c.is_whitespace())?;
        if token.is_empty() {
            let start = current_parse_offset(context, input);
            let preview = crate::error::preview_input(
                input.display().as_ref(),
                crate::error::DEFAULT_INPUT_PREVIEW,
            );
            return Err(crate::error::ParseError::custom(format!(
                "expected string, found '{preview}'"
            ))
            .with_span(SourceSpan::point(start)));
        }

        Ok((token.into_owned(), rest))
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
    S: AsRef<str> + FromInputStr<'a, I>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        _context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let (value, rest) = input.take_till(T::VALUE)?;
        Ok((
            Self {
                value: S::from_input_str(value)?,
                _marker: std::marker::PhantomData,
            },
            rest,
        ))
    }
}

pub type TakeTillTokenStr<'a, T> = TakeTillToken<T, &'a str>;
pub type TakeTillTokenCow<'a, T> = TakeTillToken<T, Cow<'a, str>>;
pub type TakeTillTokenString<T> = TakeTillToken<T, String>;

#[cfg(test)]
mod tests {
    use stable_pattern::Searcher;

    use crate::literals::Comma;
    use crate::primitives::prelude::AlphaString;

    use super::*;

    #[test]
    fn test_cow_parse() {
        let input = "hello world";
        let (cow, rest) = Cow::parse(input).unwrap();
        assert_eq!(cow, "hello");
        assert_eq!(rest, " world");
    }

    #[test]
    fn test_alpha_parse_bytes() {
        let (word, rest) = AlphaString::parse(b"hello world".as_slice()).unwrap();
        assert_eq!(word.value, "hello");
        assert_eq!(rest, b" world");
    }

    #[test]
    fn test_take_till_token() {
        let (value, rest) = TakeTillToken::<Comma>::parse("hello,world").unwrap();
        assert_eq!(value, "hello");
        assert_eq!(rest, ",world");
    }

    #[test]
    fn test_borrowed_str_parse_requires_borrow_input() {
        #[derive(Clone, Copy)]
        struct StreamingStub<'a>(&'a str);

        impl<'a> Input<'a> for StreamingStub<'a> {
            fn input_len(self) -> usize {
                self.0.len()
            }

            fn trim_start(self) -> Self {
                Self(self.0.trim_start())
            }

            fn is_empty(self) -> bool {
                self.0.is_empty()
            }

            fn display(self) -> Cow<'a, str> {
                self.0.into()
            }

            fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>> {
                Ok(self.0.strip_prefix(prefix).map(Self))
            }

            fn advance(self, bytes: usize) -> ParseResult<Self> {
                if bytes > self.0.len() {
                    return Err(crate::error::ParseError::custom("invalid input bounds"));
                }
                if !self.0.is_char_boundary(bytes) {
                    return Err(crate::error::ParseError::custom("invalid UTF-8 boundary"));
                }

                Ok(Self(&self.0[bytes..]))
            }

            fn find(self, needle: &str) -> ParseResult<Option<Self>> {
                Ok(self.0.find(needle).map(|idx| Self(&self.0[idx..])))
            }

            fn slice_to(self, end: Self) -> ParseResult<Self> {
                if end.0.len() > self.0.len() {
                    return Err(crate::error::ParseError::custom("invalid input bounds"));
                }
                let consumed = self.0.len() - end.0.len();
                Ok(Self(&self.0[..consumed]))
            }

            fn take_char(self) -> ParseResult<Option<(char, Self)>> {
                let mut chars = self.0.chars();
                Ok(chars.next().map(|ch| (ch, Self(chars.as_str()))))
            }

            fn take_while<P>(self, predicate: P) -> ParseResult<(Cow<'a, str>, Self)>
            where
                P: for<'b> stable_pattern::Pattern<'b> + Copy,
            {
                let mut idx = 0;
                while let Some(rest) = predicate.strip_prefix_of(&self.0[idx..]) {
                    idx += self.0[idx..].len() - rest.len();
                }
                let matched = self.0[..idx].to_owned();
                Ok((Cow::Owned(matched), Self(&self.0[idx..])))
            }

            fn take_till<P>(self, predicate: P) -> ParseResult<(Cow<'a, str>, Self)>
            where
                P: for<'b> stable_pattern::Pattern<'b> + Copy,
            {
                let idx = predicate
                    .into_searcher(self.0)
                    .next_match()
                    .map_or(self.0.len(), |(start, _)| start);
                let matched = self.0[..idx].to_owned();
                Ok((Cow::Owned(matched), Self(&self.0[idx..])))
            }
        }

        impl<'a> crate::parse::ParseOffsetInput<'a> for StreamingStub<'a> {
            fn parse_offset_anchor(self) -> crate::parse::ParseOffsetAnchor {
                crate::parse::ParseOffsetAnchor::new(self.0.as_ptr() as usize, self.0.len())
            }

            fn parse_offset_from(self, root: crate::parse::ParseOffsetAnchor) -> Option<usize> {
                let start = self.0.as_ptr() as usize;
                let end = start.saturating_add(self.0.len());

                if start < root.start() || end > root.end() {
                    return None;
                }

                Some(start.saturating_sub(root.start()))
            }
        }

        let (parsed, rest) = String::parse(StreamingStub("hello world")).unwrap();
        assert_eq!(parsed, "hello");
        assert_eq!(rest.0, " world");
    }
}
