use super::{
    Parse,
    ParseOffsetContext,
    ParseOffsetInput,
    current_parse_offset,
    with_parse_offset_scope,
};
use crate::error::{ParseError, ParseResult};
use crate::prelude::Span;

/// Convenience function to parse complete input, ensuring everything is consumed.
///
/// This function parses the input and verifies that all meaningful content
/// has been consumed (trailing whitespace is allowed).
pub fn parse_complete<'a, T: Parse<'a>>(input: &'a str) -> ParseResult<T> {
    parse_complete_input::<_, T>(input)
}

/// Convenience function to parse complete string input into a spanned result.
///
/// This is equivalent to `parse_complete_input::<_, Span<T>>(input)`.
pub fn parse_complete_spanned<'a, T: Parse<'a>>(input: &'a str) -> ParseResult<Span<T>> {
    parse_complete_input::<_, Span<T>>(input)
}

/// Convenience function to parse and fully consume an abstract input.
///
/// Leading and trailing whitespace handling is delegated to the input type.
pub fn parse_complete_input<'a, I, T>(input: I) -> ParseResult<T>
where
    I: ParseOffsetInput<'a>,
    T: Parse<'a, I>,
{
    let mut context = ParseOffsetContext::new();
    with_parse_offset_scope(&mut context, input, |context| {
        let (result, remaining) = T::parse_with_context(input, context)?;
        let remaining = remaining.trim_start();
        if remaining.is_empty() {
            Ok(result)
        } else {
            let start = current_parse_offset(context, remaining);
            let span = crate::error::SourceSpan::new(start, start + remaining.input_len());
            Err(ParseError::custom(format!(
                "unexpected trailing input: '{}'",
                remaining.display()
            ))
            .with_span(span))
        }
    })
}

/// Convenience function to parse and fully consume input into a spanned result.
pub fn parse_complete_input_spanned<'a, I, T>(input: I) -> ParseResult<Span<T>>
where
    I: ParseOffsetInput<'a>,
    T: Parse<'a, I>,
{
    parse_complete_input::<_, Span<T>>(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combinators::{span::Span, ws::Ws};
    use crate::error::SourceSpan;
    use crate::input::TokenStream;
    use crate::lit_token;
    use crate::literals::KwNull;

    lit_token!(HelloParser, "hello");

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
    fn test_parse_complete_spanned() {
        let result = parse_complete_spanned::<HelloParser>("hello").unwrap();
        assert_eq!(result.span.start, 0);
        assert_eq!(result.span.end, 5);
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

    #[test]
    fn test_parse_complete_input_spanned_tokens() {
        let tokens = ["hello"];
        let result =
            parse_complete_input_spanned::<_, HelloParser>(TokenStream::new(&tokens)).unwrap();
        assert_eq!(result.span.start, 0);
        assert_eq!(result.span.end, 1);
    }

    #[test]
    fn test_parse_complete_input_spanned_bytes_uses_byte_offsets() {
        let result = parse_complete_input_spanned::<_, char>("\u{00E9}".as_bytes()).unwrap();
        assert_eq!(result.inner, '\u{00E9}');
        assert_eq!(result.span, SourceSpan::new(0, 2));
    }

    #[test]
    fn test_parse_complete_input_trailing_bytes_reports_byte_span() {
        let err = parse_complete_input::<_, HelloParser>("hello\u{00E9}".as_bytes())
            .err()
            .expect("expected trailing byte input error");
        assert_eq!(err.span(), Some(SourceSpan::new(5, 7)));
    }

    #[test]
    fn test_parse_complete_input_trailing_tokens_reports_token_span() {
        let tokens = ["hello", "world"];
        let err = parse_complete_input::<_, HelloParser>(TokenStream::new(&tokens))
            .err()
            .expect("expected trailing token input error");
        assert_eq!(err.span(), Some(SourceSpan::new(1, 2)));
    }

    #[test]
    fn test_parse_complete_tracks_nested_span_offset() {
        type Parser = crate::and!(Ws<i64>, Span<Ws<KwNull>>);
        let result = parse_complete::<Parser>("42 null").unwrap();
        let null = result.right;
        assert_eq!(null.span, SourceSpan::new(2, 7));
    }

    #[test]
    fn test_parse_complete_tracks_error_offset() {
        type Parser = crate::and!(Ws<i64>, Ws<KwNull>);
        let err = parse_complete::<Parser>("42 nope").unwrap_err();
        assert_eq!(err.span(), Some(SourceSpan::point(3)));
    }

    #[test]
    fn test_parse_complete_tracks_eof_after_partial_consumption() {
        type Parser = crate::and!(char, char);
        let err = parse_complete::<Parser>("a").unwrap_err();
        assert_eq!(err.span(), Some(SourceSpan::point(1)));
    }

    #[test]
    fn test_parse_offset_tracking_is_thread_safe() {
        type Parser = crate::and!(Ws<i64>, Span<Ws<KwNull>>);
        let strs = ["1 null", "42 null", "123 null"];

        std::thread::scope(|s| {
            let handles = [
                s.spawn(|| parse_complete::<Parser>(&strs[0]).unwrap().right.span),
                s.spawn(|| parse_complete::<Parser>(&strs[1]).unwrap().right.span),
                s.spawn(|| parse_complete::<Parser>(&strs[2]).unwrap().right.span),
            ];

            let spans: Vec<_> = handles
                .into_iter()
                .map(|handle| handle.join().expect("threaded parse should succeed"))
                .collect();

            assert_eq!(spans[0], SourceSpan::new(1, 6));
            assert_eq!(spans[1], SourceSpan::new(2, 7));
            assert_eq!(spans[2], SourceSpan::new(3, 8));
        });
    }
}
