use crate::{
    error::{ParseError, ParseResult, SourceSpan},
    literals::{LBrace, LBracket, LParen, RBrace, RBracket, RParen},
    parse::{Parse, ParseOffsetInput, current_parse_offset},
    token::Token,
};

/// A combinator for parsing nested delimiters, like parentheses, brackets, or braces.
///
/// This combinator handles balanced delimiters and allows the inner content to contain
/// nested occurrences of the same delimiters. For example, it can parse `((42))` (with [`Parenthesized`]) as a parenthesized integer,
/// or `[a[b]c]` as a bracketed string (with [`Bracketed`]), correctly handling the nesting structure.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DelimNested<Left, Right, Inner> {
    pub left: Left,
    pub inner: Inner,
    pub right: Right,
}

impl<Left, Right, Inner> DelimNested<Left, Right, Inner> {
    pub fn map_inner<NewInner>(
        self,
        f: impl FnOnce(Inner) -> NewInner,
    ) -> DelimNested<Left, Right, NewInner> {
        DelimNested {
            left: self.left,
            inner: f(self.inner),
            right: self.right,
        }
    }

    pub fn into_inner(self) -> Inner {
        self.inner
    }

    pub fn inner(&self) -> &Inner {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut Inner {
        &mut self.inner
    }
}

/// Parenthesized content, e.g. `(content)`
pub type Parenthesized<I> = DelimNested<LParen, RParen, I>;
/// Bracketed content, e.g. `[content]`
pub type Bracketed<I> = DelimNested<LBracket, RBracket, I>;
/// Braced content, e.g. `{content}`
pub type Braced<I> = DelimNested<LBrace, RBrace, I>;

impl<'a, I, Left, Right, Inner> Parse<'a, I> for DelimNested<Left, Right, Inner>
where
    I: ParseOffsetInput<'a>,
    Left: Token + Parse<'a, I>,
    Right: Token + Parse<'a, I>,
    Inner: Parse<'a, I>,
{
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        // These checks are effectively const with respect to `Left`/`Right`
        // but cannot be enforced at compile-time on stable Rust because
        // `Token::VALUE` is a trait-level associated const of a generic param.
        // The branches are monomorphized per instantiation, so the checks
        // effectively have zero cost in the steady state.
        if Left::VALUE.is_empty() || Right::VALUE.is_empty() {
            return Err(ParseError::fatal(
                "nested delimiters must be non-empty tokens",
            ));
        }
        if Left::VALUE == Right::VALUE {
            return Err(ParseError::fatal(
                "nested delimiters must not be identical tokens",
            ));
        }
        let (left, mut cursor) = Left::parse_with_context(input, context)?;
        let inner_start = cursor;
        let mut depth = 1usize;
        let close_start = loop {
            let next_left = cursor.find(Left::VALUE)?;
            let next_right = cursor.find(Right::VALUE)?;

            if let (Some(left_at), Some(right_at)) = (next_left, next_right) {
                let left_dist = cursor.input_len() - left_at.input_len();
                let right_dist = cursor.input_len() - right_at.input_len();

                if left_dist < right_dist
                    || (left_dist == right_dist && Left::VALUE.len() > Right::VALUE.len())
                {
                    let (_, rest) = Left::parse_with_context(left_at, context)?;
                    depth += 1;
                    cursor = rest;
                    continue;
                }

                if depth == 1 {
                    break right_at;
                }

                let (_, rest) = Right::parse_with_context(right_at, context)?;
                depth -= 1;
                cursor = rest;
                continue;
            }

            if let Some(left_at) = next_left {
                let (_, rest) = Left::parse_with_context(left_at, context)?;
                depth += 1;
                cursor = rest;
                continue;
            }

            if let Some(right_at) = next_right {
                if depth == 1 {
                    break right_at;
                }

                let (_, rest) = Right::parse_with_context(right_at, context)?;
                depth -= 1;
                cursor = rest;
                continue;
            }

            // Neither delimiter remains; the opening left is unmatched.
            let start = current_parse_offset(context, input);
            return Err(ParseError::custom(format!(
                "unbalanced nested delimiter: missing `{}` to close `{}`",
                Right::VALUE,
                Left::VALUE,
            ))
            .with_span(start));
        };

        let inner_input = inner_start.slice_to(close_start)?;
        let (inner, inner_rest) = Inner::parse_with_context(inner_input, context)?;
        let inner_rest = inner_rest.trim_start();

        if !inner_rest.is_empty() {
            let start = current_parse_offset(context, inner_rest);
            let span = SourceSpan::new(start, start.saturating_add(inner_rest.input_len()));
            let preview = crate::error::preview_input(
                inner_rest.display().as_ref(),
                crate::error::DEFAULT_INPUT_PREVIEW,
            );
            return Err(ParseError::custom(format!(
                "unexpected trailing input inside nested delimiter: '{preview}'"
            ))
            .with_span(span));
        }

        let (right, remaining) = Right::parse_with_context(close_start, context)?;
        Ok((DelimNested { left, inner, right }, remaining))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        literals::*,
        parse::{Parse, parse_complete},
    };

    use super::DelimNested;

    #[test]
    fn nested_parses_balanced_parentheses() {
        let (parsed, rest) = DelimNested::<LParen, RParen, i64>::parse("(42)tail").unwrap();
        assert_eq!(parsed.inner, 42);
        assert_eq!(rest, "tail");
    }

    #[test]
    fn nested_parses_inner_with_nested_delimiters() {
        let (parsed, rest) = DelimNested::<LParen, RParen, String>::parse("(a(b)c)tail").unwrap();
        assert_eq!(parsed.inner, "a(b)c");
        assert_eq!(rest, "tail");
    }

    #[test]
    fn nested_rejects_unbalanced_input() {
        let result = DelimNested::<LParen, RParen, String>::parse("(a(b)c");
        assert!(result.is_err());
    }

    #[test]
    fn nested_rejects_partial_inner_match() {
        let err = parse_complete::<DelimNested<LParen, RParen, i64>>("(42abc)").unwrap_err();
        assert_eq!(err.span(), Some(crate::error::SourceSpan::new(3, 6)));
        assert!(
            err.to_string()
                .contains("unexpected trailing input inside nested delimiter")
        );
    }
}
