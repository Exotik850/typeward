use crate::{error::ParseResult, input::Input, parse::Parse};

pub mod basic;
pub mod filtered;
pub mod float;
pub mod int;
pub mod str;

pub mod prelude {
    pub use super::basic::*;
    pub use super::filtered::*;
    pub use super::str::{TakeTillToken, TakeTillTokenCow, TakeTillTokenStr};
}

impl<'a, I> Parse<'a, I> for bool
where
    I: Input<'a>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        _context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        if let Some(rest) = input.strip_prefix("true")? {
            Ok((true, rest))
        } else if let Some(rest) = input.strip_prefix("false")? {
            Ok((false, rest))
        } else {
            let preview = crate::error::preview_input(
                input.display().as_ref(),
                crate::error::DEFAULT_INPUT_PREVIEW,
            );
            Err(crate::error::ParseError::custom(format!(
                "expected 'true' or 'false', found '{preview}'"
            )))
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
        let result = bool::parse(input);
        assert!(result.is_err());
    }
}
