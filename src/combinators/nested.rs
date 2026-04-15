use crate::{
    error::{ParseError, ParseResult},
    literals::*,
    input::Input,
    parse::{Parse, parse_complete_input},
    token::Token,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Nested<Left, Right, Inner> {
    pub left: Left,
    pub inner: Inner,
    pub right: Right,
}

impl<Left, Right, Inner> Nested<Left, Right, Inner> {
    pub fn map_inner<NewInner>(
        self,
        f: impl FnOnce(Inner) -> NewInner,
    ) -> Nested<Left, Right, NewInner> {
        Nested {
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

pub type Parenthesized<I> = Nested<LParen, RParen, I>;
pub type Bracketed<I> = Nested<LBracket, RBracket, I>;
pub type Braced<I> = Nested<LBrace, RBrace, I>;

impl<'a, I, Left, Right, Inner> Parse<'a, I> for Nested<Left, Right, Inner>
where
    I: Input<'a>,
    Left: Token,
    Right: Token,
    Inner: Parse<'a, I>,
{
    fn parse(input: I) -> ParseResult<(Self, I)> {
        if Left::VALUE.is_empty() || Right::VALUE.is_empty() {
            return Err(ParseError::custom(
                "nested delimiters must be non-empty tokens",
            ));
        }
        if Left::VALUE == Right::VALUE {
            return Err(ParseError::custom(
                "nested delimiters must not be identical tokens",
            ));
        }
        let (left, mut cursor) = Left::parse(input)?;
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
                    let (_, rest) = Left::parse(left_at)?;
                    depth += 1;
                    cursor = rest;
                    continue;
                }

                if depth == 1 {
                    break right_at;
                }

                let (_, rest) = Right::parse(right_at)?;
                depth -= 1;
                cursor = rest;
                continue;
            }

            if let Some(left_at) = next_left {
                let (_, rest) = Left::parse(left_at)?;
                depth += 1;
                cursor = rest;
                continue;
            }

            if let Some(right_at) = next_right {
                if depth == 1 {
                    break right_at;
                }

                let (_, rest) = Right::parse(right_at)?;
                depth -= 1;
                cursor = rest;
                continue;
            }

            return Err(ParseError::UnexpectedEOF);
        };

        let inner_input = inner_start.slice_to(close_start)?;
        let inner = parse_complete_input::<I, Inner>(inner_input)?;

        let (right, remaining) = Right::parse(close_start)?;
        Ok((Nested { left, inner, right }, remaining))
    }
}

#[cfg(test)]
mod tests {
    use crate::{literals::*, parse::Parse};

    use super::Nested;

    #[test]
    fn nested_parses_balanced_parentheses() {
        let (parsed, rest) = Nested::<LParen, RParen, i64>::parse("(42)tail").unwrap();
        assert_eq!(parsed.inner, 42);
        assert_eq!(rest, "tail");
    }

    #[test]
    fn nested_parses_inner_with_nested_delimiters() {
        let (parsed, rest) = Nested::<LParen, RParen, String>::parse("(a(b)c)tail").unwrap();
        assert_eq!(parsed.inner, "a(b)c");
        assert_eq!(rest, "tail");
    }

    #[test]
    fn nested_rejects_unbalanced_input() {
        let result = Nested::<LParen, RParen, String>::parse("(a(b)c");
        assert!(result.is_err());
    }
}
