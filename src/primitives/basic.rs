use std::borrow::Cow;

use crate::{input::Input, parse::Parse};

pub type Empty = ();
pub type Success = ();

/// A parser that always succeeds and consumes the rest of the input.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Rest<I> {
    value: I,
}

impl<'a, I> Parse<'a, I> for Rest<I>
where
    I: Input<'a>,
{
    #[inline]
    fn parse_with_context(
        value: I,
        _context: &mut crate::parse::ParseOffsetContext,
    ) -> crate::error::ParseResult<(Self, I)> {
        let empty = value.slice_to(value)?;
        Ok((Rest { value }, empty))
    }
}

impl<I> std::ops::Deref for Rest<I> {
    type Target = I;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<I: PartialEq<I>> PartialEq<I> for Rest<I> {
    fn eq(&self, other: &I) -> bool {
        &self.value == other
    }
}

pub type RestStr<'a> = Rest<&'a str>;
pub type RestString = Rest<String>;
pub type RestCowStr<'a> = Rest<Cow<'a, str>>;

/// A parser that checks that the input is empty, and fails if it is not.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Eof;

impl<'a, I> Parse<'a, I> for Eof
where
    I: Input<'a>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        _context: &mut crate::parse::ParseOffsetContext,
    ) -> crate::error::ParseResult<(Self, I)> {
        if input.is_empty() {
            Ok((Eof, input))
        } else {
            let preview = crate::error::preview_input(
                input.display().as_ref(),
                crate::error::DEFAULT_INPUT_PREVIEW,
            );
            Err(crate::error::ParseError::custom(format!(
                "expected end of input, but found '{preview}'"
            )))
        }
    }
}

/// A parser that always fails.
///
/// Useful as a sentinel in generic combinators (e.g. as the `B` branch of
/// `Or<A, B>` to forbid a second alternative) or to seed a [`crate::combinators::or::Or`]
/// chain that is later extended.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fail;

impl<'a, I> Parse<'a, I> for Fail
where
    I: Input<'a>,
{
    #[inline]
    fn parse_with_context(
        _: I,
        _context: &mut crate::parse::ParseOffsetContext,
    ) -> crate::error::ParseResult<(Self, I)> {
        Err(crate::error::ParseError::custom("`Fail` always fails"))
    }
}

#[cfg(test)]
mod tests {
    use crate::{and, parse::Parse, primitives::prelude::AlphaString};

    use super::*;

    #[test]
    fn test_rest() {
        let input = "some input";
        type AlphaAndRest<'a> = and!(AlphaString, RestStr<'a>);
        let (res, remain) = AlphaAndRest::parse(input).unwrap();
        assert_eq!(res.left, "some");
        assert_eq!(res.right.value, " input");
        assert_eq!(remain, "");
    }
}
