use crate::{
    error::{ParseError, ParseResult},
    parse::Parse,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Or<A, B> {
    Left(A),
    Right(B),
}

impl<A, B> Or<A, B> {
    pub fn is_left(&self) -> bool {
        matches!(self, Or::Left(_))
    }

    pub fn is_right(&self) -> bool {
        matches!(self, Or::Right(_))
    }

    pub fn left(&self) -> Option<&A> {
        match self {
            Or::Left(value) => Some(value),
            Or::Right(_) => None,
        }
    }

    pub fn right(&self) -> Option<&B> {
        match self {
            Or::Left(_) => None,
            Or::Right(value) => Some(value),
        }
    }

    pub fn left_mut(&mut self) -> Option<&mut A> {
        match self {
            Or::Left(value) => Some(value),
            Or::Right(_) => None,
        }
    }

    pub fn right_mut(&mut self) -> Option<&mut B> {
        match self {
            Or::Left(_) => None,
            Or::Right(value) => Some(value),
        }
    }

    pub fn into_left(self) -> Option<A> {
        match self {
            Or::Left(value) => Some(value),
            Or::Right(_) => None,
        }
    }

    pub fn into_right(self) -> Option<B> {
        match self {
            Or::Left(_) => None,
            Or::Right(value) => Some(value),
        }
    }

    pub fn as_ref(&self) -> Or<&A, &B> {
        match self {
            Or::Left(value) => Or::Left(value),
            Or::Right(value) => Or::Right(value),
        }
    }

    pub fn as_mut(&mut self) -> Or<&mut A, &mut B> {
        match self {
            Or::Left(value) => Or::Left(value),
            Or::Right(value) => Or::Right(value),
        }
    }

    pub fn map_left<C, F>(self, map: F) -> Or<C, B>
    where
        F: FnOnce(A) -> C,
    {
        match self {
            Or::Left(value) => Or::Left(map(value)),
            Or::Right(value) => Or::Right(value),
        }
    }

    pub fn map_right<C, F>(self, map: F) -> Or<A, C>
    where
        F: FnOnce(B) -> C,
    {
        match self {
            Or::Left(value) => Or::Left(value),
            Or::Right(value) => Or::Right(map(value)),
        }
    }

    pub fn either<T, FL, FR>(self, left: FL, right: FR) -> T
    where
        FL: FnOnce(A) -> T,
        FR: FnOnce(B) -> T,
    {
        match self {
            Or::Left(value) => left(value),
            Or::Right(value) => right(value),
        }
    }
}

impl<'a, I, A, B> Parse<'a, I> for Or<A, B>
where
    I: crate::input::Input<'a>,
    A: Parse<'a, I>,
    B: Parse<'a, I>,
{
    /// Tries `A` then `B`, preferring the error that advanced farthest into
    /// the input. Fatal errors short-circuit the alternative and propagate.
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        match A::parse_with_context(input, context) {
            Ok((a, rest)) => Ok((Or::Left(a), rest)),
            Err(left_err) if left_err.is_fatal() => Err(left_err),
            Err(left_err) => match B::parse_with_context(input, context) {
                Ok((b, rest)) => Ok((Or::Right(b), rest)),
                Err(right_err) if right_err.is_fatal() => Err(right_err),
                Err(right_err) => Err(combine_alternative_errors(left_err, right_err)),
            },
        }
    }
}

fn combine_alternative_errors(left_err: ParseError, right_err: ParseError) -> ParseError {
    if let Some(merged) = merge_expected_token_errors(&left_err, &right_err) {
        return merged;
    }

    left_err.farthest(right_err)
}

fn merge_expected_token_errors(
    left_err: &ParseError,
    right_err: &ParseError,
) -> Option<ParseError> {
    if left_err.span() != right_err.span() {
        return None;
    }

    let (left_expected, left_found) = expected_tokens(left_err.root_cause())?;
    let (right_expected, right_found) = expected_tokens(right_err.root_cause())?;

    if left_found != right_found {
        return None;
    }

    let expected = left_expected
        .into_iter()
        .chain(right_expected)
        .collect::<Vec<_>>();
    let mut error = ParseError::expected_one_of(expected, left_found);
    if let Some(span) = left_err.span() {
        error = error.with_span(span);
    }

    Some(error)
}

fn expected_tokens(error: &ParseError) -> Option<(Vec<&'static str>, &str)> {
    match error {
        ParseError::UnexpectedToken { expected, found } => Some((vec![*expected], found.as_str())),
        ParseError::ExpectedOneOf { expected, found } => Some((expected.clone(), found.as_str())),
        _ => None,
    }
}

#[macro_export]
macro_rules! or {
    ($first:ty, $second:ty $(,)?) => {
        $crate::combinators::or::Or<$first, $second>
    };
    ($first:ty, $($rest:ty),+ $(,)?) => {
        $crate::combinators::or::Or<$first, $crate::or!($($rest),+)>
    };
}

#[macro_export]
macro_rules! or_match {
    ($value:expr, $pattern:pat => $expr:expr $(,)?) => {
        {
            let $pattern = $value;
            $expr
        }
    };
    ($value:expr, $pattern:pat => $expr:expr, $($rest_pattern:pat => $rest_expr:expr),+ $(,)?) => {
        match $value {
            $crate::combinators::or::Or::Left($pattern) => $expr,
            $crate::combinators::or::Or::Right(rest) => {
                $crate::or_match!(rest, $($rest_pattern => $rest_expr),+)
            }
        }
    };
}

pub type Either<A, B> = Or<A, B>;
pub type Alt<A, B> = Or<A, B>;

#[cfg(test)]
mod tests {
    use crate::combinators::ws::Ws;
    use crate::literals::*;
    use crate::parse::parse_complete;
    use crate::primitives::prelude::*;

    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct UnspannedError;

    impl<'a, I> Parse<'a, I> for UnspannedError
    where
        I: crate::input::Input<'a>,
    {
        fn parse_with_context(
            _input: I,
            _context: &mut crate::parse::ParseOffsetContext,
        ) -> crate::error::ParseResult<(Self, I)> {
            Err(crate::error::ParseError::custom("unspanned"))
        }
    }

    #[test]
    fn test_alt() {
        let input = "42";
        type AltType = or!(i64, AlphaString);
        let (result, rest) = AltType::parse(input).unwrap();
        assert_eq!(result, Or::Left(42));
        assert_eq!(rest, "");
    }

    #[test]
    fn test_alt_second() {
        let input = "hello";
        type AltType = or!(i64, AlphaString);
        let (result, rest) = AltType::parse(input).unwrap();
        assert_eq!(
            result,
            Or::Right(AlphaString {
                value: "hello".to_string()
            })
        );
        assert_eq!(rest, "");
    }

    #[test]
    fn test_alt_nested() {
        let input = "42";
        type AltType = or!(i64, AlphaString, IdentifierString);
        let (result, rest) = AltType::parse(input).unwrap();
        assert_eq!(result, Or::Left(42));
        assert_eq!(rest, "");
    }

    #[test]
    fn test_alt_helpers() {
        let value: Or<i64, AlphaString> = Or::Left(42);
        assert!(value.is_left());
        assert!(!value.is_right());
        assert_eq!(value.left().copied(), Some(42));
        assert_eq!(value.right(), None);

        let mapped = value.map_left(|n| n.to_string());
        assert_eq!(mapped, Or::Left("42".to_string()));

        let rendered = mapped.either(|n| format!("left-{n}"), |s| format!("right-{}", s.value));
        assert_eq!(rendered, "left-42");
    }

    #[test]
    fn test_alt_match_nested_right_branch() {
        type AltType = or!(i64, AlphaString, IdentifierString);
        let (result, _) = AltType::parse("_name").unwrap();

        let rendered = or_match!(
            result,
            n => format!("num:{n}"),
            alpha => format!("alpha:{}", alpha.value),
            ident => format!("ident:{}", ident.value),
        );
        assert_eq!(rendered, "ident:_name");
    }

    #[test]
    fn test_alt_match_nested_middle_branch() {
        type AltType = or!(i64, AlphaString, IdentifierString);
        let (result, _) = AltType::parse("hello").unwrap();

        let rendered = or_match!(
            result,
            n => format!("num:{n}"),
            alpha => format!("alpha:{}", alpha.value),
            ident => format!("ident:{}", ident.value),
        );

        assert_eq!(rendered, "alpha:hello");
    }

    #[test]
    fn test_alt_error_prefers_farthest_span() {
        type AltType = or!(
            crate::and!(i64, Ws<KwNull>),
            crate::and!(i64, Ws<KwTrue>, Ws<KwNull>)
        );
        let err = parse_complete::<AltType>("42 true nope").unwrap_err();

        assert_eq!(err.span(), Some(crate::error::SourceSpan::point(8)));
    }

    #[test]
    fn test_alt_error_tie_merges_expected_tokens() {
        type AltType = or!(True, False);
        let err = parse_complete::<AltType>("maybe").unwrap_err();

        assert!(matches!(
            err.root_cause(),
            crate::error::ParseError::ExpectedOneOf {
                expected,
                ..
            } if expected.contains(&"true") && expected.contains(&"false")
        ));
    }

    #[test]
    fn test_alt_error_prefers_spanned_over_unspanned() {
        type AltType = or!(i64, Null);
        let err = parse_complete::<AltType>("abc").unwrap_err();

        assert_eq!(err.span(), Some(crate::error::SourceSpan::point(0)));
        assert!(matches!(
            err.root_cause(),
            crate::error::ParseError::UnexpectedToken {
                expected: "null",
                ..
            }
        ));
    }

    #[test]
    fn test_alt_error_keeps_first_when_second_is_unspanned() {
        type AltType = or!(Null, UnspannedError);
        let err = parse_complete::<AltType>("abc").unwrap_err();

        assert_eq!(err.span(), Some(crate::error::SourceSpan::point(0)));
        assert!(matches!(
            err.root_cause(),
            crate::error::ParseError::UnexpectedToken {
                expected: "null",
                ..
            }
        ));
    }
}
