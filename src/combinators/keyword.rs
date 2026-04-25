use std::marker::PhantomData;

use crate::{
    error::{ParseError, ParseResult, SourceSpan},
    parse::{Parse, ParseOffsetInput, current_parse_offset},
    token::Token,
};

/// Boundary policy used by [`Keyword`].
///
/// The parser succeeds only when the next character after a matched token
/// satisfies `is_boundary`.
pub trait KeywordBoundary {
    fn is_boundary(ch: char) -> bool;
}

/// Default boundary policy for language keywords.
///
/// Alphanumeric and underscore characters are considered identifier
/// continuations and therefore not valid keyword boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct IdentBoundary;

impl KeywordBoundary for IdentBoundary {
    fn is_boundary(ch: char) -> bool {
        !(ch.is_alphanumeric() || ch == '_')
    }
}

/// Parse a [`Token`] as a keyword, requiring a boundary after the token text.
///
/// This avoids prefix-footguns like parsing `null` from `nullish`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Keyword<T, B = IdentBoundary> {
    token: T,
    _boundary: PhantomData<B>,
}

impl<T, B> Keyword<T, B> {
    #[must_use]
    pub fn into_inner(self) -> T {
        self.token
    }

    #[must_use]
    pub fn inner(&self) -> &T {
        &self.token
    }
}

impl<T, B> std::ops::Deref for Keyword<T, B> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.token
    }
}

impl<T: PartialEq<T>, B> PartialEq<T> for Keyword<T, B> {
    fn eq(&self, other: &T) -> bool {
        &self.token == other
    }
}

/// Convenience alias for [`Keyword`] with the default identifier boundary.
pub type Kw<T> = Keyword<T, IdentBoundary>;

impl<'a, I, T, B> Parse<'a, I> for Keyword<T, B>
where
    I: ParseOffsetInput<'a>,
    T: Token + Copy,
    B: KeywordBoundary,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let start = current_parse_offset(context, input);

        let Some(rest) = input.strip_prefix(T::VALUE)? else {
            let found = input.display();
            return Err(ParseError::unexpected_token(T::VALUE, found.as_ref())
                .with_span(SourceSpan::point(start)));
        };

        if let Some((next, _)) = rest.take_char()?
            && !B::is_boundary(next)
        {
            return Err(ParseError::custom(format!(
                "keyword '{}' must be followed by a boundary",
                T::VALUE
            ))
            .with_span(SourceSpan::from_start_len(
                start,
                T::VALUE.len() + next.len_utf8(),
            )));
        }

        Ok((
            Keyword {
                token: T::default(),
                _boundary: PhantomData,
            },
            rest,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        literals::{Comma, KwNull},
        parse::Parse,
    };

    use super::*;

    #[test]
    fn keyword_accepts_boundary_end_of_input() {
        let (kw, rest) = Kw::<KwNull>::parse("null").unwrap();
        assert_eq!(kw, KwNull);
        assert_eq!(rest, "");
    }

    #[test]
    fn keyword_accepts_boundary_punctuation() {
        let (kw, rest) = Kw::<KwNull>::parse("null,").unwrap();
        assert_eq!(kw, KwNull);
        let (_comma, rest) = Comma::parse(rest).unwrap();
        assert_eq!(rest, "");
    }

    #[test]
    fn keyword_rejects_identifier_continuation() {
        let err = Kw::<KwNull>::parse("nullish").unwrap_err();
        assert_eq!(err.span(), Some(SourceSpan::new(0, 5)));
        assert!(
            err.to_string()
                .contains("keyword 'null' must be followed by a boundary")
        );
    }
}
