use crate::{
    error::{ParseError, ParseResult, SourceSpan},
    parse::{Parse, ParseOffsetContext, ParseOffsetInput, current_parse_offset},
};

macro_rules! parse_unsigned {
    ($($ty:ty),*) => {
        $(
            impl<'a, I> Parse<'a, I> for $ty
            where
                I: ParseOffsetInput<'a>,
            {
                #[inline]
                fn parse_with_context(
                    input: I,
                    context: &mut ParseOffsetContext,
                ) -> ParseResult<(Self, I)> {
                    let (result, rest) = input.take_while(|c: char| c.is_ascii_digit())?;
                    if result.is_empty() {
                        let start = current_parse_offset(context, input);
                        return Err(ParseError::custom(concat!(
                            "expected ",
                            stringify!($ty)
                        ))
                        .with_span(SourceSpan::point(start)));
                    }

                    match result.parse::<$ty>() {
                        Ok(num) => Ok((num, rest)),
                        Err(_) => {
                            let start = current_parse_offset(context, input);
                            let consumed = input.input_len().saturating_sub(rest.input_len());
                            Err(ParseError::custom(concat!(
                                "value out of range for ",
                                stringify!($ty)
                            ))
                            .with_span(SourceSpan::from_start_len(start, consumed)))
                        }
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
                I: ParseOffsetInput<'a>,
            {
                #[inline]
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap, clippy::cast_sign_loss)]
                fn parse_with_context(
                    input: I,
                    context: &mut ParseOffsetContext,
                ) -> ParseResult<(Self, I)> {
                    let (negative, sign_trimmed) = if let Some(rest) = input.strip_prefix("-")? {
                        (true, rest)
                    } else if let Some(rest) = input.strip_prefix("+")? {
                        (false, rest)
                    } else {
                        (false, input)
                    };

                    let (result, rest) = sign_trimmed.take_while(|c: char| c.is_ascii_digit())?;
                    if result.is_empty() {
                        let start = current_parse_offset(context, sign_trimmed);
                        return Err(ParseError::custom(concat!(
                            "expected ",
                            stringify!($ty)
                        ))
                        .with_span(SourceSpan::point(start)));
                    }

                    let start = current_parse_offset(context, input);
                    let consumed = input.input_len().saturating_sub(rest.input_len());
                    let span = SourceSpan::from_start_len(start, consumed);

                    let Ok(magnitude) = result.parse::<u128>() else {
                        return Err(ParseError::custom(concat!(
                            "value out of range for ",
                            stringify!($ty)
                        ))
                        .with_span(span));
                    };

                    let max = <$ty>::MAX as u128;
                    let value = if negative {
                        if magnitude == max + 1 {
                            <$ty>::MIN
                        } else if magnitude <= max {
                            -(magnitude as $ty)
                        } else {
                            return Err(ParseError::custom(concat!(
                                "value out of range for ",
                                stringify!($ty)
                            ))
                            .with_span(span));
                        }
                    } else if magnitude <= max {
                        magnitude as $ty
                    } else {
                        return Err(ParseError::custom(concat!(
                            "value out of range for ",
                            stringify!($ty)
                        ))
                        .with_span(span));
                    };

                    Ok((value, rest))
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
                I: ParseOffsetInput<'a>,
            {
                #[inline]
                fn parse_with_context(
                    input: I,
                    context: &mut crate::parse::ParseOffsetContext,
                ) -> ParseResult<(Self, I)> {
                    let (value, rest) = <$inner>::parse_with_context(input, context)?;
                    match std::num::$wrapper::new(value) {
                        Some(value) => Ok((value, rest)),
                        None => {
                            let start = current_parse_offset(context, input);
                            let consumed = input.input_len().saturating_sub(rest.input_len());
                            Err(crate::error::ParseError::custom(concat!(
                                "expected non-zero ",
                                stringify!($wrapper),
                                ", found 0"
                            ))
                            .with_span(SourceSpan::from_start_len(start, consumed)))
                        }
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
        let err = i64::parse(input).unwrap_err();
        assert_eq!(err.span(), Some(SourceSpan::point(0)));
    }

    #[test]
    fn test_i64_parse_overflow() {
        let input = "9223372036854775808 rest"; // i64::MAX + 1
        let err = i64::parse(input).unwrap_err();
        assert_eq!(err.span(), Some(SourceSpan::new(0, 19)));
    }

    #[test]
    fn test_i64_parse_limits() {
        let input = "-9223372036854775808 rest";
        let (num, rest) = i64::parse(input).unwrap();
        assert_eq!(num, i64::MIN);
        assert_eq!(rest, " rest");

        let input = "9223372036854775807 rest";
        let (num, rest) = i64::parse(input).unwrap();
        assert_eq!(num, i64::MAX);
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
        assert_eq!(err.span(), Some(SourceSpan::new(0, 1)));
        assert_eq!(
            err.root_cause(),
            &crate::error::ParseError::custom("expected non-zero NonZeroU32, found 0")
        );
    }
}
