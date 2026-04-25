use crate::{error::ParseResult, input::Input, parse::Parse};
use std::borrow::Cow;

#[macro_export]
macro_rules! filter_str {
    ($name:ident, $filter:expr) => {
        #[derive(Debug, Eq, Clone, Hash, Default, PartialOrd, Ord)]
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

        impl<S: AsRef<str>> AsRef<str> for $name<S> {
            fn as_ref(&self) -> &str {
                self.value.as_ref()
            }
        }

        // partial eq impl to compare with &str
        impl<S: AsRef<str>, O: AsRef<str>> PartialEq<O> for $name<S> {
            fn eq(&self, other: &O) -> bool {
                self.value.as_ref() == other.as_ref()
            }
        }

        impl<'a, I, S> $crate::parse::Parse<'a, I> for $name<S>
        where
            I: $crate::input::Input<'a>,
            S: AsRef<str> + $crate::input::FromInputStr<'a, I>,
        {
            #[inline]
            fn parse_with_context(
                input: I,
                _context: &mut $crate::parse::ParseOffsetContext,
            ) -> $crate::error::ParseResult<(Self, I)> {
                let (s, rest) = input.take_while($filter)?;
                if s.is_empty() {
                    let preview = $crate::error::preview_input(
                        input.display().as_ref(),
                        $crate::error::DEFAULT_INPUT_PREVIEW,
                    );
                    return Err($crate::error::ParseError::custom(format!(
                        "expected {}, found '{}'",
                        stringify!($name),
                        preview
                    )));
                }
                Ok((
                    $name {
                        value: S::from_input_str(s)?,
                    },
                    rest,
                ))
            }
        }

        pastey::paste! {
            pub type [<$name Str>]<'a> = $name<&'a str>;
            pub type [<$name String>] = $name<String>;
            pub type [<$name Cow>]<'a> = $name<Cow<'a, str>>;
        }
    };
}

filter_str!(Alpha, |c: char| c.is_ascii_alphabetic());
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

#[macro_export]
macro_rules! parse_filtered {
    ($name:ident, $filter:expr, $($ty:ty),+) => {

        #[derive(Debug, PartialEq, Eq, Clone, Hash, Default, PartialOrd, Ord)]
        pub struct $name<T>(pub T);
        $(
            impl<'a, I> Parse<'a, I> for $name<$ty>
            where
                I: Input<'a>,
                $ty: Parse<'a, I>,
            {
                #[inline]
                fn parse_with_context(
                    input: I,
                    context: &mut $crate::parse::ParseOffsetContext,
                ) -> ParseResult<(Self, I)> {
                    let (result, rest) = <$ty>::parse_with_context(input, context)?;
                    if !$filter(result) {
                        return Err($crate::error::ParseError::custom(format!(
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
parse_filtered!(NonNegative, |n| n >= 0, i8, i16, i32, i64, i128, isize);
parse_filtered!(NonPositive, |n| n <= 0, i8, i16, i32, i64, i128, isize);
parse_filtered!(NonZeroFloat, |n| n != 0.0, f32, f64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positive() {
        let result = Positive::<i32>::parse("5");
        assert!(result.is_ok());
    }

    #[test]
    fn test_negative() {
        let result = Negative::<i32>::parse("-5");
        assert!(result.is_ok());
    }

    #[test]
    fn test_non_negative() {
        let result = NonNegative::<i32>::parse("5");
        assert!(result.is_ok());
    }

    #[test]
    fn test_non_positive() {
        let result = NonPositive::<i32>::parse("-5");
        assert!(result.is_ok());
    }

    #[test]
    fn test_non_zero_float() {
        let result = NonZeroFloat::<f32>::parse("5.0");
        assert!(result.is_ok());
    }

    // is error tests

    #[test]
    fn test_positive_error() {
        let result = Positive::<i32>::parse("-5");
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_error() {
        let result = Negative::<i32>::parse("5");
        assert!(result.is_err());
    }

    // string tests

    #[test]
    fn test_alpha() {
        let result = AlphaStr::parse("abc");
        assert!(result.is_ok());

        let (parsed, remaining) = AlphaStr::parse("abc123").unwrap();
        assert_eq!(parsed, "abc");
        assert_eq!(remaining, "123");
    }
}
