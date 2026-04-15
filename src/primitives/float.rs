use crate::{error::ParseResult, input::Input, parse::Parse};

macro_rules! parse_float {
    ($($ty:ty),*) => {
        $(
            impl<'a, I> Parse<'a, I> for $ty
            where
                I: Input<'a>,
            {
                fn parse(input: I) -> ParseResult<(Self, I)> {
                    let (result, rest) = <&str>::parse(input)?;
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
                            result
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
}
