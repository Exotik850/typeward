use crate::{
    error::ParseResult,
    input::Input,
    parse::{Parse, ParseOffsetInput, current_parse_offset},
};

#[inline]
fn is_float_lexeme_char(ch: char) -> bool {
    ch.is_ascii_digit() || matches!(ch, '+' | '-' | '.' | 'e' | 'E')
}

fn parse_float_partial_prefix<'a, I, F>(input: I) -> ParseResult<Option<(F, I)>>
where
    I: Input<'a>,
    F: fast_float2::FastFloat,
{
    let (candidate, scanned_rest) = input.take_while(is_float_lexeme_char)?;

    let Ok((parsed, consumed)) = fast_float2::parse_partial::<F, _>(candidate.as_bytes()) else {
        return Ok(None);
    };

    if consumed == 0 {
        return Ok(None);
    }

    let rest = if consumed == candidate.len() {
        scanned_rest
    } else {
        input.advance(consumed)?
    };

    Ok(Some((parsed, rest)))
}

macro_rules! parse_float {
    ($($ty:ty),*) => {
        $(
            impl<'a, I> Parse<'a, I> for $ty
            where
                I: ParseOffsetInput<'a>,
            {
                #[inline]
                fn parse_with_context(
                    input: I,
                    context: &mut crate::parse::ParseOffsetContext,
                ) -> ParseResult<(Self, I)> {
                    let Some((num, rest)) = parse_float_partial_prefix::<_, $ty>(input)? else {
                        let start = current_parse_offset(context, input);
                        return Err(crate::error::ParseError::custom(concat!(
                            "expected ",
                            stringify!($ty)
                        ))
                        .with_span(start));
                    };

                    Ok((num, rest))
                }
            }
        )*
    };
}

parse_float!(f32, f64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_parse() {
        let input = "3.125 rest";
        let (num, rest) = f64::parse(input).unwrap();
        assert_eq!(num, 3.125);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_f64_parse_bytes() {
        let input: &[u8] = b"2.75 rest";
        let (num, rest) = f64::parse(input).unwrap();
        assert_eq!(num, 2.75);
        assert_eq!(rest, b" rest");
    }

    #[test]
    fn test_f64_parse_until_delimiter() {
        let input = "-12.5e2, next";
        let (num, rest) = f64::parse(input).unwrap();
        assert!((num - (-1250.0)).abs() < f64::EPSILON);
        assert_eq!(rest, ", next");
    }

    #[test]
    fn test_f64_parse_trailing_e() {
        let input = "-2.5e2e rest";
        let (result, rest) = f64::parse(input).unwrap();
        assert!((result - (-250.0)).abs() < f64::EPSILON);
        assert_eq!(rest, "e rest");
    }

    #[test]
    fn test_f64_parse_rejects_consecutive_signs_after_mantissa() {
        let input = "1.5+-99";
        let (result, rest) = f64::parse(input).unwrap();
        assert!((result - 1.5).abs() < f64::EPSILON);
        assert_eq!(rest, "+-99");
    }

    #[test]
    fn test_f64_parse_invalid_exponent_suffix_keeps_marker() {
        let input = "12.5e+ rest";
        let (result, rest) = f64::parse(input).unwrap();
        assert!((result - 12.5).abs() < f64::EPSILON);
        assert_eq!(rest, "e+ rest");
    }

    #[test]
    fn test_f64_parse_invalid_has_span() {
        let err = f64::parse("nope").unwrap_err();
        assert_eq!(err.span(), Some(crate::error::SourceSpan::point(0)));
    }
}
