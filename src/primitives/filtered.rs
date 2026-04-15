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
            S: AsRef<str> + From<&'a str>,
        {
            fn parse(input: I) -> $crate::error::ParseResult<(Self, I)> {
                let (s, rest) = input.take_while($filter)?;
                if s.is_empty() {
                    return Err($crate::error::ParseError::custom(format!(
                        "expected {}, found '{}'",
                        stringify!($name),
                        input.display()
                    )));
                }
                Ok(($name { value: S::from(s) }, rest))
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
                fn parse(input: I) -> ParseResult<(Self, I)> {
                    let (result, rest) = <$ty>::parse(input)?;
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
