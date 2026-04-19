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
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> crate::error::ParseResult<(Self, I)> {
        // Lookahead first: do not execute `S` if `P` accepts the input.
        // This avoids running both parsers' side effects (span scopes, owned
        // buffers from streaming inputs) in the success path.
        if P::parse_with_context(input, context).is_ok() {
            let preview = crate::error::preview_input(
                input.display().as_ref(),
                crate::error::DEFAULT_INPUT_PREVIEW,
            );
            return Err(crate::error::ParseError::custom(format!(
                "expected `{}` but matched the forbidden pattern `{}` at '{}'",
                std::any::type_name::<S>(),
                std::any::type_name::<P>(),
                preview,
            )));
        }

        let (value, rest) = S::parse_with_context(input, context)?;
        Ok((
            Not {
                value,
                _marker: std::marker::PhantomData,
            },
            rest,
        ))
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
