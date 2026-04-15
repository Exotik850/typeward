use crate::{error::ParseResult, input::Input, parse::Parse};

pub mod filtered;
pub mod float;
pub mod int;
pub mod str;
pub mod basic;

pub mod prelude {
    pub use super::filtered::*;
    pub use super::basic::*;
}

impl<'a, I> Parse<'a, I> for bool
where
    I: Input<'a>,
{
    fn parse(input: I) -> ParseResult<(Self, I)> {
        if let Some(rest) = input.strip_prefix("true")? {
            Ok((true, rest))
        } else if let Some(rest) = input.strip_prefix("false")? {
            Ok((false, rest))
        } else {
            Err(crate::error::ParseError::custom(format!(
                "expected 'true' or 'false', found '{}'",
                input.display()
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
