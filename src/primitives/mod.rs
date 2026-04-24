use crate::{
    error::{ParseResult, SourceSpan},
    parse::{Parse, ParseOffsetInput, current_parse_offset},
};

pub mod basic;
pub mod filtered;
pub mod float;
pub mod int;
pub mod str;

pub mod prelude {
    pub use super::basic::*;
    pub use super::filtered::*;
    pub use super::str::{TakeTillToken, TakeTillTokenCow, TakeTillTokenStr, TakeTillTokenString};
}

impl<'a, I> Parse<'a, I> for bool
where
    I: ParseOffsetInput<'a>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        if let Some(rest) = input.strip_prefix("true")? {
            Ok((true, rest))
        } else if let Some(rest) = input.strip_prefix("false")? {
            Ok((false, rest))
        } else {
            let start = current_parse_offset(context, input);
            let preview = crate::error::preview_input(
                input.display().as_ref(),
                crate::error::DEFAULT_INPUT_PREVIEW,
            );
            Err(crate::error::ParseError::custom(format!(
                "expected 'true' or 'false', found '{preview}'"
            ))
            .with_span(SourceSpan::point(start)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_parse_true() {
        let input = "true rest";
        let (val, rest) = bool::parse(input).unwrap();
        assert!(val);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_bool_parse_false() {
        let input = "false rest";
        let (val, rest) = bool::parse(input).unwrap();
        assert!(!val);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_bool_parse_invalid() {
        let input = "yes rest";
        let err = bool::parse(input).unwrap_err();
        assert_eq!(err.span(), Some(SourceSpan::point(0)));
    }
}
