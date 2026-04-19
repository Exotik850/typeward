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
                #[inline]
                fn parse_with_context(
                    input: I,
                    context: &mut crate::parse::ParseOffsetContext,
                ) -> ParseResult<(Self, I)> {
                    let (result, rest) = Digit::<&str>::parse_with_context(input, context)?;
                    if result.is_empty() {
                        let preview = crate::error::preview_input(
                            input.display().as_ref(),
                            crate::error::DEFAULT_INPUT_PREVIEW,
                        );
                        return Err(crate::error::ParseError::custom(format!(
                            "expected {}, found '{}'",
                            stringify!($ty),
                            preview
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
                #[inline]
                fn parse_with_context(
                    input: I,
                    context: &mut crate::parse::ParseOffsetContext,
                ) -> ParseResult<(Self, I)> {
                    let input = input.trim_start();
                    let sign_trimmed = if let Some(rest) = input.strip_prefix("-")? {
                        rest
                    } else if let Some(rest) = input.strip_prefix("+")? {
                        rest
                    } else {
                        input
                    };

                    let (result, rest) = Digit::<&str>::parse_with_context(sign_trimmed, context)?;
                    if result.is_empty() {
                        let preview = crate::error::preview_input(
                            input.display().as_ref(),
                            crate::error::DEFAULT_INPUT_PREVIEW,
                        );
                        return Err(crate::error::ParseError::custom(format!(
                            "expected {}, found '{}'",
                            stringify!($ty),
                            preview
                        )));
                    }

                    let (signed_lexeme, _) = input.slice_to(rest)?.take_while(|_: char| true)?;
                    match signed_lexeme.parse::<$ty>() {
                        Ok(num) => Ok((num, rest)),
                        Err(_) => Err(crate::error::ParseError::custom(format!(
                            "expected {}, found '{}'",
                            stringify!($ty),
                            signed_lexeme.deref()
                        ))),
                    }
                }
            }
        )*
    };
}

parse_signed!(i8, i16, i32, i64, i128, isize);

macro_rules! parse_nonzero {
    ($($wrapper:ident => $inner:ty),* $(,)?) => {
        $(
            impl<'a, I> Parse<'a, I> for std::num::$wrapper
            where
                I: Input<'a>,
            {
                #[inline]
                fn parse_with_context(
                    input: I,
                    context: &mut crate::parse::ParseOffsetContext,
                ) -> ParseResult<(Self, I)> {
                    let (value, rest) = <$inner>::parse_with_context(input, context)?;
                    match std::num::$wrapper::new(value) {
                        Some(value) => Ok((value, rest)),
                        None => Err(crate::error::ParseError::custom(format!(
                            "expected non-zero {}, found 0",
                            stringify!($wrapper)
                        ))),
                    }
                }
            }
        )*
    };
}

parse_nonzero!(
    NonZeroU8 => u8,
    NonZeroU16 => u16,
    NonZeroU32 => u32,
    NonZeroU64 => u64,
    NonZeroU128 => u128,
    NonZeroUsize => usize,
    NonZeroI8 => i8,
    NonZeroI16 => i16,
    NonZeroI32 => i32,
    NonZeroI64 => i64,
    NonZeroI128 => i128,
    NonZeroIsize => isize,
);

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
    fn test_i64_parse_min_value() {
        let input = "-9223372036854775808 rest";
        let (num, rest) = i64::parse(input).unwrap();
        assert_eq!(num, i64::MIN);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_i8_parse_min_value() {
        let input = "-128 rest";
        let (num, rest) = i8::parse(input).unwrap();
        assert_eq!(num, i8::MIN);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_i64_parse_bytes() {
        let input: &[u8] = b"123 rest";
        let (num, rest) = i64::parse(input).unwrap();
        assert_eq!(num, 123);
        assert_eq!(rest, b" rest");
    }

    #[test]
    fn test_nonzero_u32_parse() {
        let input = "42 rest";
        let (num, rest) = std::num::NonZeroU32::parse(input).unwrap();
        assert_eq!(num.get(), 42);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_nonzero_i64_parse() {
        let input = "-7 rest";
        let (num, rest) = std::num::NonZeroI64::parse(input).unwrap();
        assert_eq!(num.get(), -7);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_nonzero_u32_parse_rejects_zero() {
        let input = "0 rest";
        let err = std::num::NonZeroU32::parse(input).unwrap_err();
        assert_eq!(
            err,
            crate::error::ParseError::custom("expected non-zero NonZeroU32, found 0")
        );
    }
}
