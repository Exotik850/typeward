use crate::error::{ParseError, ParseResult};

/// A trait for types that can be parsed from a string.
///
/// This is the main trait that structs should implement to become parseable.
/// The lifetime parameter `'a` represents the lifetime of the input string,
/// allowing implementations to borrow from the input when appropriate.
pub trait Parse<'a>: Sized {
    /// Parse a value from the input string.
    ///
    /// Returns the parsed value and the remaining unconsumed input.
    fn parse(input: &'a str) -> ParseResult<(Self, &'a str)>;
}

impl Parse<'_> for () {
    fn parse(input: &str) -> ParseResult<(Self, &str)> {
        Ok(((), input))
    }
}

/// Convenience function to parse complete input, ensuring everything is consumed.
///
/// This function parses the input and verifies that all meaningful content
/// has been consumed (trailing whitespace is allowed).
pub fn parse_complete<'a, T: Parse<'a>>(input: &'a str) -> ParseResult<T> {
    let (result, remaining) = T::parse(input)?;
    let trimmed = remaining.trim();
    if trimmed.is_empty() {
        Ok(result)
    } else {
        Err(ParseError::custom(format!(
            "unexpected trailing input: '{}'",
            trimmed
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A simple test parser that consumes "hello"
    struct HelloParser;
    impl<'a> Parse<'a> for HelloParser {
        fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
            if input.starts_with("hello") {
                Ok((HelloParser, &input[5..]))
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
}
