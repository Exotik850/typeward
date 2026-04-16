use crate::{error::ParseResult, input::Input, parse::Parse};

macro_rules! parse_float {
    ($($ty:ty),*) => {
        $(
            impl<'a, I> Parse<'a, I> for $ty
            where
                I: Input<'a>,
            {
                fn parse(input: I) -> ParseResult<(Self, I)> {
                    let input = input.trim_start()?;
                    // TODO: Make this more robust by parsing manually
                    // instead of using this filter, which can produce false negatives.
                    // e.g. it would fail to parse "1.2e-3e" because of the trailing 'e'.
                    let (result, rest) = input.take_while(|c: char| {
                        matches!(c, '+' | '-' | '.' | 'e' | 'E' | '0'..='9')
                    })?;
                    if result.is_empty() {
                        return Err(crate::error::ParseError::custom(format!(
                            "expected {}",
                            stringify!($ty)
                        )));
                    }
                    match result.parse::<$ty>() {
                        Ok(num) => Ok((num, rest)),
                        Err(_) => Err(crate::error::ParseError::custom(format!(
                            "expected {}",
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
        let input = "3.14 rest";
        let (num, rest) = f64::parse(input).unwrap();
        assert!((num - 3.14).abs() < f64::EPSILON);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_f64_parse_bytes() {
        let input: &[u8] = b"2.718 rest";
        let (num, rest) = f64::parse(input).unwrap();
        assert!((num - 2.718).abs() < f64::EPSILON);
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
        assert_eq!(rest, " rest");
    }
}
