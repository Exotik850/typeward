use super::filtered::Digit;
use crate::{error::ParseResult, input::Input, parse::Parse};
use std::ops::Deref;

macro_rules! parse_unsigned {
    ($($ty:ty),*) => {
        $(
            impl<'a, I> Parse<'a, I> for $ty
            where
                I: Input<'a>,
            {
                fn parse(input: I) -> ParseResult<(Self, I)> {
                    let (result, rest) = Digit::<&str>::parse(input)?;
                    if result.is_empty() {
                        return Err(crate::error::ParseError::custom(format!(
                            "expected {}, found '{}'",
                            stringify!($ty),
                            input.display()
                        )));
                    }
                    match result.parse::<$ty>() {
                        Ok(num) => Ok((num, rest)),
                        Err(_) => Err(crate::error::ParseError::custom(format!(
                            "expected {}, found '{}'",
                            stringify!($ty),
                            result.deref()
                        ))),
                    }
                }
            }
        )*
    };
}

parse_unsigned!(u8, u16, u32, u64, u128, usize);

macro_rules! parse_signed {
    ($($ty:ty),*) => {
        $(
            impl<'a, I> Parse<'a, I> for $ty
            where
                I: Input<'a>,
            {
                fn parse(input: I) -> ParseResult<(Self, I)> {
                    let input = input.trim_start()?;
                    let (sign, rest) = if let Some(rest) = input.strip_prefix("-")? {
                        (-1, rest)
                    } else if let Some(rest) = input.strip_prefix("+")? {
                        (1, rest)
                    } else {
                        (1, input)
                    };
                    let (result, rest) = Digit::<&str>::parse(rest)?;
                    if result.is_empty() {
                        return Err(crate::error::ParseError::custom(format!(
                            "expected {}, found '{}'",
                            stringify!($ty),
                            input.display()
                        )));
                    }
                    match result.parse::<$ty>() {
                        Ok(num) => Ok((sign * num, rest)),
                        Err(_) => Err(crate::error::ParseError::custom(format!(
                            "expected {}, found '{}'",
                            stringify!($ty),
                            result.deref()
                        ))),
                    }
                }
            }
        )*
    };
}

parse_signed!(i8, i16, i32, i64, i128, isize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u32_parse() {
        let input = "42 rest";
        let (num, rest) = u32::parse(input).unwrap();
        assert_eq!(num, 42);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_i64_parse() {
        let input = "-123 rest";
        let (num, rest) = i64::parse(input).unwrap();
        assert_eq!(num, -123);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_i64_parse_invalid() {
        let input = "abc rest";
        let result = i64::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_i64_parse_bytes() {
        let input: &[u8] = b"123 rest";
        let (num, rest) = i64::parse(input).unwrap();
        assert_eq!(num, 123);
        assert_eq!(rest, b" rest");
    }
}
