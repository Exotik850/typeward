use crate::{error::ParseResult, parse::Parse};

/// Commit parser failures from `T` by promoting recoverable errors to fatal.
///
/// This is useful when a parser branch has consumed enough structure that
/// falling back to other alternatives would be misleading or expensive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Cut<T>(pub T);

/// Alias for [`Cut`], emphasizing grammar commit semantics.
pub type Commit<T> = Cut<T>;

impl<T> Cut<T> {
    #[must_use]
    pub fn new(value: T) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }

    #[must_use]
    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> std::ops::Deref for Cut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: PartialEq<T>> PartialEq<T> for Cut<T> {
    fn eq(&self, other: &T) -> bool {
        &self.0 == other
    }
}

impl<'a, I, T> Parse<'a, I> for Cut<T>
where
    I: crate::input::Input<'a>,
    T: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        match T::parse_with_context(input, context) {
            Ok((value, rest)) => Ok((Self(value), rest)),
            Err(err) if err.is_fatal() => Err(err),
            Err(err) => Err(err.into_fatal()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combinators::ws::Ws;
    use crate::error::SourceSpan;
    use crate::literals::{KwNull, KwTrue};
    use crate::parse::Parse;

    #[test]
    fn cut_promotes_recoverable_errors_to_fatal() {
        type Parser = Cut<Ws<KwNull>>;

        let err = Parser::parse("nope").unwrap_err();
        assert!(err.is_fatal());
    }

    #[test]
    fn cut_commits_alternative_branch() {
        type Parser = crate::or!(crate::and!(KwTrue, Cut<Ws<KwNull>>), KwTrue);

        let err = Parser::parse("true nope").unwrap_err();
        assert!(err.is_fatal());
    }

    #[test]
    fn cut_commits_repetition_items() {
        type Item = Cut<crate::and!(Ws<i64>, Ws<KwNull>)>;

        let err = Vec::<Item>::parse("1 null 2 nope").unwrap_err();
        assert!(err.is_fatal());
        assert_eq!(err.span(), Some(SourceSpan::point(9)));
    }
}
