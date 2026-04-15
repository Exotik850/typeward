use std::borrow::Cow;
use std::ops::Deref;

use crate::error::ParseResult;
use crate::parse::Parse;

// ============================================================================
// String-like types
// ============================================================================

impl<'a> Parse<'a> for &'a str {
    fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
        if input.is_empty() {
            return Err(crate::error::ParseError::custom(
                "expected string, found end of input",
            ));
        }
        let split_idx = input.find(char::is_whitespace).unwrap_or(input.len());
        if split_idx == 0 {
            return Err(crate::error::ParseError::custom(format!(
                "expected string, found '{}'",
                input
            )));
        }
        Ok(input.split_at(split_idx))
    }
}

impl<'a> Parse<'a> for String {
    fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
        let (s, rest) = <&str>::parse(input)?;
        Ok((s.to_string(), rest))
    }
}

impl<'a> Parse<'a> for Cow<'a, str> {
    fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
        let (result, rest) = <&str>::parse(input)?;
        Ok((Cow::Borrowed(result), rest))
    }
}

#[macro_export]
macro_rules! filter_str {
    ($name:ident, $filter:expr) => {
        #[derive(Debug, PartialEq, Eq, Clone, Hash, Default, PartialOrd, Ord)]
        pub struct $name<S>
        where
            S: AsRef<str>,
        {
            pub value: S,
        }

        impl<S: AsRef<str>> std::ops::Deref for $name<S> {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                self.value.as_ref()
            }
        }

        impl<'a, S> Parse<'a> for $name<S>
        where
            S: AsRef<str> + From<&'a str>,
        {
            fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
                let end_idx = input.find(|c| !$filter(c)).unwrap_or(input.len());
                let s = &input[..end_idx];
                if s.is_empty() {
                    return Err(crate::error::ParseError::custom(format!(
                        "expected {}, found '{}'",
                        stringify!($name),
                        input
                    )));
                }
                Ok(($name { value: S::from(s) }, &input[end_idx..]))
            }
        }

        pastey::paste! {
            pub type [<$name Str>]<'a> = $name<&'a str>;
            pub type [<$name String>] = $name<String>;
            pub type [<$name Cow>]<'a> = $name<Cow<'a, str>>;
        }
    };
}

filter_str!(Alpha, char::is_alphabetic);
filter_str!(Digit, |c: char| c.is_ascii_digit());
filter_str!(AlphaNum, char::is_alphanumeric);
filter_str!(Identifier, |c: char| c.is_alphanumeric() || c == '_');
filter_str!(Whitespace, char::is_whitespace);
filter_str!(NonWhitespace, |c: char| !c.is_whitespace());
filter_str!(HexDigit, |c: char| c.is_ascii_hexdigit());
filter_str!(Octal, |c: char| c.is_digit(8));
filter_str!(Binary, |c: char| c == '0' || c == '1');
filter_str!(Base36, |c: char| c.is_digit(36));
filter_str!(Base64, |c: char| c.is_ascii_alphanumeric()
    || c == '+'
    || c == '/');
filter_str!(Base64Url, |c: char| c.is_ascii_alphanumeric()
    || c == '-'
    || c == '_');
filter_str!(Control, char::is_control);
filter_str!(Punctuation, |c: char| c.is_ascii_punctuation());
filter_str!(Graph, |c: char| c.is_ascii_graphic());
filter_str!(Upper, char::is_uppercase);
filter_str!(Lower, char::is_lowercase);
filter_str!(Ascii, |c: char| c.is_ascii());
filter_str!(NonAscii, |c: char| !c.is_ascii());

// ============================================================================
// Integer types
// ============================================================================

macro_rules! parse_unsigned {
    ($($ty:ty),*) => {
        $(
            impl<'a> Parse<'a> for $ty {
                fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
                    let (result, rest) = Digit::<&str>::parse(input)?;
                    if result.is_empty() {
                        return Err(crate::error::ParseError::custom(format!(
                            "expected {}, found '{}'",
                            stringify!($ty),
                            input
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
            impl<'a> Parse<'a> for $ty {
                fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
                    let input = input.trim_start();
                    let (sign, rest) = if input.starts_with('-') {
                        (-1, &input[1..])
                    } else if input.starts_with('+') {
                        (1, &input[1..])
                    } else {
                        (1, input)
                    };
                    let (result, rest) = Digit::<&str>::parse(rest)?;
                    if result.is_empty() {
                        return Err(crate::error::ParseError::custom(format!(
                            "expected {}, found '{}'",
                            stringify!($ty),
                            input
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

macro_rules! parse_float {
    ($($ty:ty),*) => {
        $(
            impl<'a> Parse<'a> for $ty {
                fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
                    let (result, rest) = <&str>::parse(input)?;
                    if result.is_empty() {
                        return Err(crate::error::ParseError::custom(format!(
                            "expected {}, found '{}'",
                            stringify!($ty),
                            input
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

macro_rules! parse_filtered {
    ($name:ident, $filter:expr, $($ty:ty),+) => {

        #[derive(Debug, PartialEq, Eq, Clone, Hash, Default, PartialOrd, Ord)]
        pub struct $name<T>(pub T);
        $(
            impl<'a> Parse<'a> for $name<$ty> {
                fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
                    let (result, rest) = <$ty>::parse(input)?;
                    if !$filter(result) {
                        return Err(crate::error::ParseError::custom(format!(
                            "expected {}, found '{}'",
                            stringify!($name),
                            result
                        )));
                    }
                    Ok(($name(result), rest))
                }
            }
        )*
    };
}

parse_filtered!(Positive, |n| n > 0, i8, i16, i32, i64, i128, isize);
parse_filtered!(Negative, |n| n < 0, i8, i16, i32, i64, i128, isize);
parse_filtered!(
    NonZero,
    |n| n != 0,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize
);
parse_filtered!(NonNegative, |n| n >= 0, i8, i16, i32, i64, i128, isize);
parse_filtered!(NonPositive, |n| n <= 0, i8, i16, i32, i64, i128, isize);
parse_filtered!(NonZeroFloat, |n| n != 0.0, f32, f64);

// ============================================================================
// Boolean
// ============================================================================

impl<'a> Parse<'a> for bool {
    fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
        if input.starts_with("true") {
            Ok((true, &input[4..]))
        } else if input.starts_with("false") {
            Ok((false, &input[5..]))
        } else {
            Err(crate::error::ParseError::custom(format!(
                "expected 'true' or 'false', found '{}'",
                input
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i64_parse() {
        let input = "123 rest";
        let (num, rest) = i64::parse(input).unwrap();
        assert_eq!(num, 123);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_i64_parse_invalid() {
        let input = "abc rest";
        let result = i64::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_f64_parse() {
        let input = "3.14 rest";
        let (num, rest) = f64::parse(input).unwrap();
        assert!((num - 3.14).abs() < f64::EPSILON);
        assert_eq!(rest, " rest");
    }

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

    #[test]
    fn test_cow_parse() {
        let input = "hello world";
        let (cow, rest) = Cow::parse(input).unwrap();
        assert_eq!(cow, "hello");
        assert_eq!(rest, " world");
    }

    #[test]
    fn test_u32_parse() {
        let input = "42 rest";
        let (num, rest) = u32::parse(input).unwrap();
        assert_eq!(num, 42);
        assert_eq!(rest, " rest");
    }

    #[test]
    fn test_float_parse() {
        let input = "2.718 rest";
        let (num, rest) = f64::parse(input).unwrap();
        assert!((num - 2.718).abs() < f64::EPSILON);
        assert_eq!(rest, " rest");
    }
}
