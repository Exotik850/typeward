use crate::{error::ParseResult, parse::Parse};

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
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        if let Ok((a, rest)) = A::parse_with_context(input, context) {
            Ok((Or::Left(a), rest))
        } else {
            let (b, rest) = B::parse_with_context(input, context)?;
            Ok((Or::Right(b), rest))
        }
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
    use crate::primitives::prelude::*;

    use super::*;

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
}
