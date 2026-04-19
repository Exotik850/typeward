use crate::{error::ParseResult, input::Input, parse::Parse};

fn scan_float_lexeme<'a, I>(input: I) -> ParseResult<Option<(String, I)>>
where
    I: Input<'a>,
{
    let mut rest = input;
    let mut lexeme = String::new();

    if let Some((ch, next)) = rest.take_char()?
        && matches!(ch, '+' | '-')
    {
        lexeme.push(ch);
        rest = next;
    }

    let mut seen_mantissa_digit = false;
    let mut seen_dot = false;
    let mut seen_exp = false;
    let mut seen_exp_digit = false;
    let mut exp_sign_allowed = false;
    let mut best_len = None;
    let mut best_rest = None;

    while let Some((ch, next)) = rest.take_char()? {
        let did_consume = if ch.is_ascii_digit() {
            if seen_exp {
                seen_exp_digit = true;
            } else {
                seen_mantissa_digit = true;
            }
            exp_sign_allowed = false;
            true
        } else if ch == '.' && !seen_dot && !seen_exp {
            seen_dot = true;
            true
        } else if matches!(ch, 'e' | 'E') && !seen_exp && seen_mantissa_digit {
            seen_exp = true;
            exp_sign_allowed = true;
            true
        } else if matches!(ch, '+' | '-') && seen_exp && exp_sign_allowed {
            exp_sign_allowed = false;
            true
        } else {
            false
        };

        if !did_consume {
            break;
        }

        lexeme.push(ch);
        rest = next;
        if seen_mantissa_digit && (!seen_exp || seen_exp_digit) {
            best_len = Some(lexeme.len());
            best_rest = Some(rest);
        }
    }

    Ok(best_len.zip(best_rest).map(|(len, best_rest)| {
        let matched = lexeme[..len].to_owned();
        (matched, best_rest)
    }))
}

macro_rules! parse_float {
    ($($ty:ty),*) => {
        $(
            impl<'a, I> Parse<'a, I> for $ty
            where
                I: Input<'a>,
            {
                #[inline]
                fn parse_with_context(
                    input: I,
                    _context: &mut crate::parse::ParseOffsetContext,
                ) -> ParseResult<(Self, I)> {
                    let Some((lexeme, rest)) = scan_float_lexeme(input)? else {
                        return Err(crate::error::ParseError::custom(concat!(
                            "expected ",
                            stringify!($ty)
                        )));
                    };

                    match lexeme.parse::<$ty>() {
                        Ok(num) => Ok((num, rest)),
                        Err(_) => Err(crate::error::ParseError::custom(concat!(
                            "expected ",
                            stringify!($ty)
                        ))),
                    }
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
}
