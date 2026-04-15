use std::{borrow::Cow, fmt};

/// The result of a parsing operation.
pub type ParseResult<T> = Result<T, ParseError>;

/// Errors that can occur during parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// The parser expected a specific token but found something else.
    UnexpectedToken {
        expected: &'static str,
        found: String,
    },
    /// The input ended unexpectedly while parsing.
    UnexpectedEOF,
    /// A custom error message.
    Custom(Cow<'static, str>),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, found } => {
                write!(
                    f,
                    "unexpected token: expected '{expected}', found '{found}'"
                )
            }
            ParseError::UnexpectedEOF => write!(f, "unexpected end of input"),
            ParseError::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    /// Create a custom parse error from any string-like value.
    pub fn custom<S: Into<Cow<'static, str>>>(msg: S) -> Self {
        ParseError::Custom(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_unexpected_token() {
        let err = ParseError::UnexpectedToken {
            expected: "let",
            found: "foo".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "unexpected token: expected 'let', found 'foo'"
        );
    }

    #[test]
    fn test_error_display_eof() {
        let err = ParseError::UnexpectedEOF;
        assert_eq!(format!("{}", err), "unexpected end of input");
    }

    #[test]
    fn test_error_display_custom() {
        let err = ParseError::custom("something went wrong");
        assert_eq!(format!("{}", err), "something went wrong");
    }

    #[test]
    fn test_error_custom() {
        let err = ParseError::custom("test error");
        assert_eq!(err, ParseError::Custom(Cow::Borrowed("test error")));
    }
}

pub fn custom<S: Into<Cow<'static, str>>>(msg: S) -> ParseError {
    ParseError::Custom(msg.into())
}
