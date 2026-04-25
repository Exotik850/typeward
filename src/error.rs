use std::{borrow::Cow, collections::HashSet, fmt, ops::Range};

/// Default maximum character count for rendering input snippets in errors.
pub const DEFAULT_INPUT_PREVIEW: usize = 80;

/// Truncate an input snippet to at most `max_chars` Unicode scalar values.
#[must_use]
pub fn preview_input(input: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }

    let mut cut_at = input.len();

    for (char_count, (idx, _)) in input.char_indices().enumerate() {
        if char_count == max_chars {
            cut_at = idx;
            break;
        }
    }

    if cut_at == input.len() {
        input.to_owned()
    } else {
        let mut preview = String::with_capacity(cut_at + 3);
        preview.push_str(&input[..cut_at]);
        preview.push_str("...");
        preview
    }
}

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

impl From<usize> for SourceSpan {
    fn from(offset: usize) -> Self {
        Self::point(offset)
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
///
/// Parse errors split into two conceptual categories:
///
/// - **Recoverable** errors describe normal "this parser didn't match"
///   failures and can be caught by backtracking combinators such as
///   [`Option<T>`], [`Result<T, ParseError>`], [`crate::combinators::or::Or`],
///   and `Many0`.
/// - **Fatal** errors are produced when the parser cannot make progress for
///   structural reasons (I/O failures, invalid UTF-8 in a streaming source,
///   exceeded replay windows, etc.). They short-circuit all backtracking
///   combinators. Use [`Self::fatal`] / [`Self::into_fatal`] to produce them
///   and [`Self::is_fatal`] to inspect them.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// The parser expected a specific token but found something else.
    ///
    /// The `found` field is already truncated to [`DEFAULT_INPUT_PREVIEW`]
    /// characters at construction time, so this variant never holds the full
    /// remaining input.
    UnexpectedToken {
        expected: &'static str,
        found: String,
    },
    /// The parser expected one of several specific tokens.
    ///
    /// The `found` field is truncated to [`DEFAULT_INPUT_PREVIEW`] at
    /// construction time.
    ExpectedOneOf {
        expected: Vec<&'static str>,
        found: String,
    },
    /// The input ended unexpectedly while parsing.
    UnexpectedEOF,
    /// A custom error message.
    Custom(Cow<'static, str>),
    /// A structural error that must not be recovered from by backtracking
    /// combinators. Produced by I/O failures, invalid UTF-8 in streaming
    /// sources, exceeded replay windows, etc.
    Fatal(Box<ParseError>),
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
                // `found` is stored pre-truncated, but re-preview defensively in
                // case callers constructed the variant manually.
                let found = preview_input(found, DEFAULT_INPUT_PREVIEW);
                write!(
                    f,
                    "unexpected token: expected '{expected}', found '{found}'"
                )
            }
            ParseError::ExpectedOneOf { expected, found } => {
                let found = preview_input(found, DEFAULT_INPUT_PREVIEW);
                let expected = expected
                    .iter()
                    .map(|token| format!("'{token}'"))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(
                    f,
                    "unexpected token: expected one of [{expected}], found '{found}'"
                )
            }
            ParseError::UnexpectedEOF => write!(f, "unexpected end of input"),
            ParseError::Custom(msg) => write!(f, "{msg}"),
            ParseError::Fatal(source) => write!(f, "fatal: {source}"),
            ParseError::WithSpan { span, source } => write!(f, "{source} at {span}"),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::WithSpan { source, .. } | ParseError::Fatal(source) => Some(source),
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

    /// Build an [`Self::UnexpectedToken`] error with the `found` snippet eagerly
    /// truncated to [`DEFAULT_INPUT_PREVIEW`] characters.
    ///
    /// Prefer this constructor over literal struct construction so that parser
    /// errors never retain a full copy of remaining input.
    #[must_use]
    pub fn unexpected_token(expected: &'static str, found: &str) -> Self {
        ParseError::UnexpectedToken {
            expected,
            found: preview_input(found, DEFAULT_INPUT_PREVIEW),
        }
    }

    /// Build an [`Self::ExpectedOneOf`] error with de-duplicated expected
    /// tokens and a pre-truncated `found` snippet.
    #[must_use]
    pub fn expected_one_of(expected: impl IntoIterator<Item = &'static str>, found: &str) -> Self {
        let mut deduped = HashSet::new();
        for token in expected {
            deduped.insert(token);
        }

        ParseError::ExpectedOneOf {
            expected: deduped.into_iter().collect(),
            found: preview_input(found, DEFAULT_INPUT_PREVIEW),
        }
    }

    /// Construct a fatal wrapper around a custom message.
    ///
    /// Fatal errors propagate through all backtracking combinators. See the
    /// type-level documentation on [`ParseError`] for details.
    #[must_use]
    pub fn fatal<S: Into<Cow<'static, str>>>(msg: S) -> Self {
        ParseError::Fatal(Box::new(ParseError::custom(msg)))
    }

    /// Promote any error into a fatal error if it isn't already one.
    #[must_use]
    pub fn into_fatal(self) -> Self {
        match self {
            ParseError::Fatal(_) => self,
            inner => ParseError::Fatal(Box::new(inner)),
        }
    }

    /// Returns `true` when this error (or any inner wrapper) is [`Self::Fatal`].
    #[must_use]
    pub fn is_fatal(&self) -> bool {
        match self {
            ParseError::Fatal(_) => true,
            ParseError::WithSpan { source, .. } => source.is_fatal(),
            _ => false,
        }
    }

    /// Attach a span to this error if it does not already carry one.
    #[must_use]
    pub fn with_span(self, span: impl Into<SourceSpan>) -> Self {
        let span = span.into();
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
    ///
    /// Looks through [`Self::Fatal`] wrappers so a span attached before or
    /// after fatal-promotion is reported identically.
    #[must_use]
    pub fn span(&self) -> Option<SourceSpan> {
        match self {
            ParseError::WithSpan { span, .. } => Some(*span),
            ParseError::Fatal(source) => source.span(),
            _ => None,
        }
    }

    /// Chooses whichever error occurred farther in the input.
    ///
    /// Preference order:
    /// - Larger span start offset
    /// - If starts tie, larger span end offset
    /// - If spans are identical or both missing, `other` (last-alternative semantics)
    /// - If only one error has a span, that error is preferred
    #[must_use]
    pub fn farthest(self, other: ParseError) -> ParseError {
        match (self.span(), other.span()) {
            (Some(left), Some(right)) => {
                if right.start > left.start {
                    other
                } else if right.start < left.start {
                    self
                } else if right.end > left.end {
                    other
                } else if right.end < left.end {
                    self
                } else {
                    other
                }
            }
            (Some(_), None) => self,
            (None, _) => other,
        }
    }

    /// Returns the innermost non-wrapper error.
    #[must_use]
    pub fn root_cause(&self) -> &ParseError {
        match self {
            ParseError::WithSpan { source, .. } | ParseError::Fatal(source) => source.root_cause(),
            _ => self,
        }
    }

    /// Render this error with line/column/source context when a span is present.
    #[must_use]
    pub fn render_with_source(&self, source: &str) -> String {
        let Some(span) = self.span() else {
            return self.to_string();
        };

        let (line, col) = span.line_col(source);
        let (line_text, marker_len) = source_line_context(source, span);
        let marker_len = marker_len.max(1);
        let marker = format!(
            "{}{}",
            " ".repeat(col.saturating_sub(1)),
            "^".repeat(marker_len)
        );

        format!(
            "{self}\n --> line {line}, column {col}\n  |\n{line:>3} | {line_text}\n  | {marker}"
        )
    }
}

fn source_line_context(source: &str, span: SourceSpan) -> (String, usize) {
    let safe_start = clamp_to_char_boundary(source, span.start.min(source.len()));
    let safe_end = clamp_to_char_boundary(source, span.end.min(source.len()));

    let line_start = source[..safe_start].rfind('\n').map_or(0, |idx| idx + 1);
    let line_end = source[safe_start..]
        .find('\n')
        .map_or(source.len(), |idx| safe_start + idx);

    let line_text = source[line_start..line_end].to_string();
    let marker_end = safe_end.min(line_end);
    let marker_len = if marker_end <= safe_start {
        1
    } else {
        source[safe_start..marker_end].chars().count().max(1)
    };

    (line_text, marker_len)
}

fn clamp_to_char_boundary(source: &str, mut offset: usize) -> usize {
    while offset > 0 && !source.is_char_boundary(offset) {
        offset -= 1;
    }

    offset
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
        let err = ParseError::custom("something went wrong").with_span(3..6);
        assert_eq!(format!("{}", err), "something went wrong at offsets 3..6");
    }

    #[test]
    fn test_preview_input_truncates_long_values() {
        let input = "abcdefghijklmnopqrstuvwxyz";
        let preview = preview_input(input, 5);
        assert_eq!(preview, "abcde...");
    }

    #[test]
    fn test_preview_input_keeps_short_values() {
        let input = "short";
        let preview = preview_input(input, 10);
        assert_eq!(preview, "short");
    }

    #[test]
    fn test_error_custom() {
        let err = ParseError::custom("test error");
        assert_eq!(err, ParseError::Custom(Cow::Borrowed("test error")));
    }

    #[test]
    fn test_error_span_accessors() {
        let err = ParseError::UnexpectedEOF.with_span(10);
        assert_eq!(err.span(), Some(SourceSpan::point(10)));
        assert_eq!(err.root_cause(), &ParseError::UnexpectedEOF);
    }

    #[test]
    fn test_error_farthest_prefers_later_span() {
        let left = ParseError::custom("left").with_span(3);
        let right = ParseError::custom("right").with_span(7);

        assert_eq!(left.farthest(right.clone()), right);
    }

    #[test]
    fn test_error_farthest_prefers_spanned_error() {
        let left = ParseError::custom("left").with_span(3);
        let right = ParseError::custom("right");

        assert_eq!(left.clone().farthest(right), left);
    }

    #[test]
    fn test_error_farthest_prefers_other_on_tie() {
        let left = ParseError::custom("left").with_span(3..5);
        let right = ParseError::custom("right").with_span(3..5);

        assert_eq!(left.farthest(right.clone()), right);
    }

    #[test]
    fn test_source_span_line_col() {
        let source = "a\nxyz\nrest";
        let span = SourceSpan::point(4);
        assert_eq!(span.line_col(source), (2, 3));
    }

    #[test]
    fn test_fatal_constructor_marks_is_fatal() {
        let err = ParseError::fatal("boom");
        assert!(err.is_fatal());
        assert_eq!(err.root_cause(), &ParseError::Custom(Cow::Borrowed("boom")));
    }

    #[test]
    fn test_into_fatal_is_idempotent() {
        let once = ParseError::custom("boom").into_fatal();
        let twice = once.clone().into_fatal();
        assert_eq!(once, twice);
        assert!(twice.is_fatal());
    }

    #[test]
    fn test_fatal_span_is_transparent_either_way() {
        let span = SourceSpan::point(7);
        let a = ParseError::custom("x").with_span(span).into_fatal();
        let b = ParseError::fatal("x").with_span(span);
        assert_eq!(a.span(), Some(span));
        assert_eq!(b.span(), Some(span));
        assert!(a.is_fatal());
        assert!(b.is_fatal());
    }

    #[test]
    fn test_unexpected_token_constructor_truncates_found() {
        let long = "x".repeat(DEFAULT_INPUT_PREVIEW + 50);
        let err = ParseError::unexpected_token("literal", &long);
        match err {
            ParseError::UnexpectedToken { found, .. } => {
                // Truncation marker "..." appended when input exceeds the limit.
                assert!(found.ends_with("..."));
                assert!(found.len() < long.len());
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    #[test]
    fn test_expected_one_of_constructor_dedupes_expected_tokens() {
        let err = ParseError::expected_one_of(["let", "if", "let"], "value");
        match err {
            ParseError::ExpectedOneOf { mut expected, .. } => {
                expected.sort();
                assert_eq!(expected, vec!["if", "let"]);
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    #[test]
    fn test_render_with_source_includes_line_context() {
        let source = "let alpha = 1;\nlet beta = ;\n";
        let err = ParseError::custom("expected expression").with_span(25);
        let rendered = err.render_with_source(source);

        assert!(rendered.contains("line 2, column 11"));
        assert!(rendered.contains("2 | let beta = ;"));
        assert!(rendered.contains("^"));
    }
}
