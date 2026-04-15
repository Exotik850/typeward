#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Not<S, P> {
    value: S,
    _marker: std::marker::PhantomData<P>,
}

impl<'a, I, S, P> crate::parse::Parse<'a, I> for Not<S, P>
where
    I: crate::input::Input<'a>,
    S: crate::parse::Parse<'a, I>,
    P: crate::parse::Parse<'a, I>,
{
    fn parse(input: I) -> crate::error::ParseResult<(Self, I)> {
        let (value, rest) = S::parse(input)?;
        if P::parse(input).is_ok() {
            Err(crate::error::custom(format!(
                "Expected Not<{}, {}> to fail, but it succeeded",
                std::any::type_name::<S>(),
                std::any::type_name::<P>()
            )))
        } else {
            Ok((
                Not {
                    value,
                    _marker: std::marker::PhantomData,
                },
                rest,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse::Parse, primitives::prelude::*};

    #[test]
    fn test_not() {
        let input = "42";
        let (result, rest) = Not::<i64, AlphaString>::parse(input).unwrap();
        assert_eq!(result.value, 42);
        assert_eq!(rest, "");
    }

    #[test]
    fn test_not_fail() {
        let input = "123";
        let err = Not::<AlphaNumString, i64>::parse(input);
        assert!(err.is_err());
    }
}
