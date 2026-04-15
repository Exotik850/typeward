use crate::{
    error::ParseError,
    input::Input,
    parse::Parse,
};

/// A trait for types that represent a specific token string value.
///
/// This is the foundational trait for defining tokens in the type system.
/// Each token type has a constant string value that it matches against input.
pub trait Token {
    /// The literal string value this token represents.
    const VALUE: &'static str;
}

#[macro_export]
macro_rules! lit_token {
    ($(#[$attr:meta])* $name:ident, $value:literal) => {
        #[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
        $(#[$attr])*
        pub struct $name;
        impl $crate::token::Token for $name {
            const VALUE: &'static str = $value;
        }
    };
}

/// Defines many tokens at once using a concise syntax.
#[macro_export]
macro_rules! define_tokens {
    ($(#[$attr:meta])* $name:ident, $value:literal; $($rest:tt)*) => {
        $crate::lit_token!($(#[$attr])* $name, $value);
        $crate::define_tokens!($($rest)*);
    };
    () => {};
}

// Blanket implementation for types that implement Token + Default
impl<'a, I, T> Parse<'a, I> for T
where
    I: Input<'a>,
    T: Token + Default,
{
    fn parse(input: I) -> Result<(Self, I), ParseError> {
        if let Some(remaining) = input.strip_prefix(Self::VALUE)? {
            Ok((Self::default(), remaining))
        } else {
            Err(ParseError::UnexpectedToken {
                expected: Self::VALUE,
                found: input.display(),
            })
        }
    }
}

// Explicit implementation for char (avoids conflict with blanket impl
// since char does not implement Token)
impl<'a, I> Parse<'a, I> for char
where
    I: Input<'a>,
{
    fn parse(input: I) -> Result<(Self, I), ParseError> {
        match input.take_char()? {
            Some((c, rest)) => Ok((c, rest)),
            None => Err(ParseError::UnexpectedEOF),
        }
    }
}

#[cfg(feature = "arrays")]
impl<'a, I, T, const N: usize> Parse<'a, I> for [T; N]
where
    I: Input<'a>,
    T: Parse<'a, I>,
{
    fn parse(mut input: I) -> crate::error::ParseResult<(Self, I)> {
        let arr = array_util::try_from_fn(|_| {
            let (value, rest) = T::parse(input)?;
            input = rest;
            Ok::<T, ParseError>(value)
        })?;
        Ok((arr, input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    lit_token!(TestKw, "test");

    #[test]
    fn test_parse_token_success() {
        let input = "test rest";
        let (_token, remaining) = TestKw::parse(input).unwrap();
        assert_eq!(remaining, " rest");
    }

    #[test]
    fn test_parse_token_failure() {
        let input = "foo bar";
        let result = TestKw::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_char_parse_token() {
        let input = "abc";
        let (c, remaining) = char::parse(input).unwrap();
        assert_eq!(c, 'a');
        assert_eq!(remaining, "bc");
    }

    #[test]
    fn test_char_parse_token_empty() {
        let input = "";
        let result = char::parse(input);
        assert_eq!(result, Err(ParseError::UnexpectedEOF));
    }

    #[test]
    #[cfg(feature = "arrays")]
    fn test_array_parse() {
        lit_token!(NToken, "n");

        let input = "nn!";
        let (_tokens, remaining) = <[NToken; 2]>::parse(input).unwrap();
        assert_eq!(remaining, "!");
    }
}
