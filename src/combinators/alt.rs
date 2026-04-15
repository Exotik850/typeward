use crate::{error::ParseResult, parse::Parse};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Alt<A, B> {
    Left(A),
    Right(B),
}

impl<A, B> Alt<A, B> {
    pub fn is_left(&self) -> bool {
        matches!(self, Alt::Left(_))
    }

    pub fn is_right(&self) -> bool {
        matches!(self, Alt::Right(_))
    }

    pub fn left(&self) -> Option<&A> {
        match self {
            Alt::Left(value) => Some(value),
            Alt::Right(_) => None,
        }
    }

    pub fn right(&self) -> Option<&B> {
        match self {
            Alt::Left(_) => None,
            Alt::Right(value) => Some(value),
        }
    }

    pub fn left_mut(&mut self) -> Option<&mut A> {
        match self {
            Alt::Left(value) => Some(value),
            Alt::Right(_) => None,
        }
    }

    pub fn right_mut(&mut self) -> Option<&mut B> {
        match self {
            Alt::Left(_) => None,
            Alt::Right(value) => Some(value),
        }
    }

    pub fn into_left(self) -> Option<A> {
        match self {
            Alt::Left(value) => Some(value),
            Alt::Right(_) => None,
        }
    }

    pub fn into_right(self) -> Option<B> {
        match self {
            Alt::Left(_) => None,
            Alt::Right(value) => Some(value),
        }
    }

    pub fn as_ref(&self) -> Alt<&A, &B> {
        match self {
            Alt::Left(value) => Alt::Left(value),
            Alt::Right(value) => Alt::Right(value),
        }
    }

    pub fn as_mut(&mut self) -> Alt<&mut A, &mut B> {
        match self {
            Alt::Left(value) => Alt::Left(value),
            Alt::Right(value) => Alt::Right(value),
        }
    }

    pub fn map_left<C, F>(self, map: F) -> Alt<C, B>
    where
        F: FnOnce(A) -> C,
    {
        match self {
            Alt::Left(value) => Alt::Left(map(value)),
            Alt::Right(value) => Alt::Right(value),
        }
    }

    pub fn map_right<C, F>(self, map: F) -> Alt<A, C>
    where
        F: FnOnce(B) -> C,
    {
        match self {
            Alt::Left(value) => Alt::Left(value),
            Alt::Right(value) => Alt::Right(map(value)),
        }
    }

    pub fn either<T, FL, FR>(self, left: FL, right: FR) -> T
    where
        FL: FnOnce(A) -> T,
        FR: FnOnce(B) -> T,
    {
        match self {
            Alt::Left(value) => left(value),
            Alt::Right(value) => right(value),
        }
    }
}

impl<'a, A, B> Parse<'a> for Alt<A, B>
where
    A: Parse<'a>,
    B: Parse<'a>,
{
    fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
        match A::parse(input) {
            Ok((a, rest)) => Ok((Alt::Left(a), rest)),
            Err(_) => {
                let (b, rest) = B::parse(input)?;
                Ok((Alt::Right(b), rest))
            }
        }
    }
}

#[macro_export]
macro_rules! alt {
    ($first:ty, $second:ty $(,)?) => {
        $crate::combinators::alt::Alt<$first, $second>
    };
    ($first:ty, $($rest:ty),+ $(,)?) => {
        $crate::combinators::alt::Alt<$first, $crate::alt!($($rest),+)>
    };
}

#[macro_export]
macro_rules! alt_match {
    ($value:expr, $pattern:pat => $expr:expr $(,)?) => {
        {
            let $pattern = $value;
            $expr
        }
    };
    ($value:expr, $pattern:pat => $expr:expr, $($rest_pattern:pat => $rest_expr:expr),+ $(,)?) => {
        match $value {
            $crate::combinators::alt::Alt::Left($pattern) => $expr,
            $crate::combinators::alt::Alt::Right(rest) => {
                $crate::alt_match!(rest, $($rest_pattern => $rest_expr),+)
            }
        }
    };
}

pub type Either<A, B> = Alt<A, B>;

#[cfg(test)]
mod tests {
    use crate::primitives::*;

    use super::*;

    #[test]
    fn test_alt() {
        let input = "42";
        type AltType = alt!(i64, AlphaString);
        let (result, rest) = AltType::parse(input).unwrap();
        assert_eq!(result, Alt::Left(42));
        assert_eq!(rest, "");
    }

    #[test]
    fn test_alt_second() {
        let input = "hello";
        type AltType = alt!(i64, AlphaString);
        let (result, rest) = AltType::parse(input).unwrap();
        assert_eq!(
            result,
            Alt::Right(AlphaString {
                value: "hello".to_string()
            })
        );
        assert_eq!(rest, "");
    }

    #[test]
    fn test_alt_nested() {
        let input = "42";
        type AltType = alt!(i64, AlphaString, IdentifierString);
        let (result, rest) = AltType::parse(input).unwrap();
        assert_eq!(result, Alt::Left(42));
        assert_eq!(rest, "");
    }

    #[test]
    fn test_alt_helpers() {
        let value: Alt<i64, AlphaString> = Alt::Left(42);
        assert!(value.is_left());
        assert!(!value.is_right());
        assert_eq!(value.left().copied(), Some(42));
        assert_eq!(value.right(), None);

        let mapped = value.map_left(|n| n.to_string());
        assert_eq!(mapped, Alt::Left("42".to_string()));

        let rendered = mapped.either(|n| format!("left-{n}"), |s| format!("right-{}", s.value));
        assert_eq!(rendered, "left-42");
    }

    #[test]
    fn test_alt_match_nested_right_branch() {
        type AltType = alt!(i64, AlphaString, IdentifierString);
        let (result, _) = AltType::parse("_name").unwrap();

        let rendered = alt_match!(
            result,
            n => format!("num:{n}"),
            alpha => format!("alpha:{}", alpha.value),
            ident => format!("ident:{}", ident.value),
        );
        assert_eq!(rendered, "ident:_name");
    }

    #[test]
    fn test_alt_match_nested_middle_branch() {
        type AltType = alt!(i64, AlphaString, IdentifierString);
        let (result, _) = AltType::parse("hello").unwrap();

        let rendered = alt_match!(
            result,
            n => format!("num:{n}"),
            alpha => format!("alpha:{}", alpha.value),
            ident => format!("ident:{}", ident.value),
        );

        assert_eq!(rendered, "alpha:hello");
    }
}
