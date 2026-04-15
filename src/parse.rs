use crate::error::{ParseError, ParseResult};
use crate::input::Input;

/// A trait for types that can be parsed from an abstract input.
///
/// This is the main trait that structs should implement to become parseable.
/// The lifetime parameter `'a` represents the lifetime of the borrowed input.
///
/// The second generic parameter defaults to `&str`, which keeps string parsing
/// ergonomic while allowing additional input forms such as `&[u8]` and token
/// slices.
pub trait Parse<'a, I: Input<'a> = &'a str>: Sized {
    /// Parse a value from the input.
    ///
    /// Returns the parsed value and the remaining unconsumed input.
    fn parse(input: I) -> ParseResult<(Self, I)>;
}

impl<'a, I> Parse<'a, I> for ()
where
    I: Input<'a>,
{
    fn parse(input: I) -> ParseResult<(Self, I)> {
        Ok(((), input))
    }
}

impl<'a, I, T> Parse<'a, I> for Option<T>
where
    I: Input<'a>,
    T: Parse<'a, I>,
{
    fn parse(input: I) -> ParseResult<(Self, I)> {
        match T::parse(input) {
            Ok((value, remaining)) => Ok((Some(value), remaining)),
            Err(_) => Ok((None, input)),
        }
    }
}

/// Convenience function to parse complete input, ensuring everything is consumed.
///
/// This function parses the input and verifies that all meaningful content
/// has been consumed (trailing whitespace is allowed).
pub fn parse_complete<'a, T: Parse<'a>>(input: &'a str) -> ParseResult<T> {
    parse_complete_input::<_, T>(input)
}

/// Convenience function to parse and fully consume an abstract input.
///
/// Leading and trailing whitespace handling is delegated to the input type.
pub fn parse_complete_input<'a, I, T>(input: I) -> ParseResult<T>
where
    I: Input<'a>,
    T: Parse<'a, I>,
{
    let (result, remaining) = T::parse(input)?;
    let remaining = remaining.trim_start()?;
    if remaining.is_empty() {
        Ok(result)
    } else {
        Err(ParseError::custom(format!(
            "unexpected trailing input: '{}'",
            remaining.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::TokenStream;

    // A simple test parser that consumes "hello"
    struct HelloParser;
    impl<'a, I> Parse<'a, I> for HelloParser
    where
        I: Input<'a>,
    {
        fn parse(input: I) -> ParseResult<(Self, I)> {
            if let Some(remaining) = input.strip_prefix("hello")? {
                Ok((HelloParser, remaining))
            } else {
                Err(ParseError::custom("expected 'hello'"))
            }
        }
    }

    #[test]
    fn test_parse_complete_success() {
        let result = parse_complete::<HelloParser>("hello").unwrap();
        let _ = result; // HelloParser has no fields to check
    }

    #[test]
    fn test_parse_complete_with_whitespace() {
        let result = parse_complete::<HelloParser>("hello   ").unwrap();
        let _ = result;
    }

    #[test]
    fn test_parse_complete_trailing_input() {
        let result = parse_complete::<HelloParser>("hello world");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_complete_input_bytes() {
        let result = parse_complete_input::<_, HelloParser>(b"hello".as_slice()).unwrap();
        let _ = result;
    }

    #[test]
    fn test_parse_complete_input_tokens() {
        let tokens = ["hello"];
        let result = parse_complete_input::<_, HelloParser>(TokenStream::new(&tokens)).unwrap();
        let _ = result;
    }
}
