use std::{borrow::Cow, fmt, ops::Range};

/// The result of a parsing operation.
pub type ParseResult<T> = Result<T, ParseError>;

/// A span in the input source.
///
/// Spans are represented as half-open byte ranges `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

impl PartialEq<Range<usize>> for SourceSpan {
    fn eq(&self, other: &Range<usize>) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl SourceSpan {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        if end < start {
            Self { start, end: start }
        } else {
            Self { start, end }
        }
    }

    #[must_use]
    pub const fn from_start_len(start: usize, len: usize) -> Self {
        Self::new(start, start.saturating_add(len))
    }

    #[must_use]
    pub const fn range(range: Range<usize>) -> Self {
        Self::new(range.start, range.end)
    }

    #[must_use]
    pub const fn point(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    #[must_use]
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Convert the start offset into 1-based line/column using `source`.
    #[must_use]
    pub fn line_col(self, source: &str) -> (usize, usize) {
        let mut safe_start = self.start.min(source.len());
        while safe_start > 0 && !source.is_char_boundary(safe_start) {
            safe_start -= 1;
        }

        let mut line = 1usize;
        let mut col = 1usize;

        for ch in source[..safe_start].chars() {
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        (line, col)
    }
}

impl From<Range<usize>> for SourceSpan {
    fn from(range: Range<usize>) -> Self {
        Self::range(range)
    }
}

impl fmt::Display for SourceSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            write!(f, "offset {}", self.start)
        } else {
            write!(f, "offsets {}..{}", self.start, self.end)
        }
    }
}

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
    /// Wraps another parse error with a source span.
    WithSpan {
        span: SourceSpan,
        source: Box<ParseError>,
    },
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
            ParseError::WithSpan { span, source } => write!(f, "{source} at {span}"),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::WithSpan { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl ParseError {
    /// Create a custom parse error from any string-like value.
    #[must_use]
    pub fn custom<S: Into<Cow<'static, str>>>(msg: S) -> Self {
        ParseError::Custom(msg.into())
    }

    /// Create a custom parse error at a specific source span.
    #[must_use]
    pub fn custom_at<S: Into<Cow<'static, str>>>(msg: S, span: SourceSpan) -> Self {
        Self::custom(msg).with_span(span)
    }

    /// Attach a span to this error if it does not already carry one.
    #[must_use]
    pub fn with_span(self, span: SourceSpan) -> Self {
        match self {
            ParseError::WithSpan { .. } => self,
            source => ParseError::WithSpan {
                span,
                source: Box::new(source),
            },
        }
    }

    /// Alias for [`with_span`], useful for fluent call chains.
    #[must_use]
    pub fn at(self, span: SourceSpan) -> Self {
        self.with_span(span)
    }

    /// Returns the span associated with this error, if any.
    #[must_use]
    pub fn span(&self) -> Option<SourceSpan> {
        match self {
            ParseError::WithSpan { span, .. } => Some(*span),
            _ => None,
        }
    }

    /// Returns the innermost non-wrapper error.
    #[must_use]
    pub fn root_cause(&self) -> &ParseError {
        match self {
            ParseError::WithSpan { source, .. } => source.root_cause(),
            _ => self,
        }
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
    fn test_error_display_with_span() {
        let err = ParseError::custom("something went wrong").with_span(SourceSpan::new(3, 6));
        assert_eq!(format!("{}", err), "something went wrong at offsets 3..6");
    }

    #[test]
    fn test_error_custom() {
        let err = ParseError::custom("test error");
        assert_eq!(err, ParseError::Custom(Cow::Borrowed("test error")));
    }

    #[test]
    fn test_error_span_accessors() {
        let err = ParseError::UnexpectedEOF.with_span(SourceSpan::point(10));
        assert_eq!(err.span(), Some(SourceSpan::point(10)));
        assert_eq!(err.root_cause(), &ParseError::UnexpectedEOF);
    }

    #[test]
    fn test_source_span_line_col() {
        let source = "a\nxyz\nrest";
        let span = SourceSpan::point(4);
        assert_eq!(span.line_col(source), (2, 3));
    }
}

pub fn custom<S: Into<Cow<'static, str>>>(msg: S) -> ParseError {
    ParseError::Custom(msg.into())
}

pub fn custom_at<S: Into<Cow<'static, str>>>(msg: S, span: SourceSpan) -> ParseError {
    ParseError::custom_at(msg, span)
}
