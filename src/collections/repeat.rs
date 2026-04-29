use crate::parse::Parse;

/// A Parser that matches `T` between `MIN` and `MAX` times, inclusive. `MIN` must be less than or equal to `MAX`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Repeat<T, const MIN: usize, const MAX: usize> {
    items: Vec<T>,
}

impl<T, const MIN: usize, const MAX: usize> Repeat<T, MIN, MAX> {
    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.items
    }

    #[must_use]
    pub fn into_items(self) -> Vec<T> {
        self.items
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<'a, I, T, const MIN: usize, const MAX: usize> crate::parse::Parse<'a, I>
    for Repeat<T, MIN, MAX>
where
    I: crate::input::Input<'a>,
    T: crate::parse::Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> crate::error::ParseResult<(Self, I)> {
        const {
            assert!(MIN <= MAX, "Repeat<T, MIN, MAX> requires MIN <= MAX");
        }

        let mut items = Vec::new();
        let mut rest = input;

        for res in crate::combinators::iter::iter::<I, T>(rest, context, "Repeat").take(MAX) {
            let (item, new_rest) = match res {
                Ok((item, new_rest)) => (item, new_rest),
                Err(err) if err.is_fatal() => return Err(err),
                Err(_) => break,
            };
            items.push(item);
            rest = new_rest;
        }

        if items.len() < MIN {
            return Err(crate::error::ParseError::custom(format!(
                "expected at least {} items, found {}",
                MIN,
                items.len()
            )));
        }

        Ok((Repeat { items }, rest))
    }
}

pub type MaxN<T, const N: usize> = Repeat<T, 0, N>;
pub type MinN<T, const N: usize> = Repeat<T, N, { usize::MAX }>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RepeatUntil<T, U> {
    items: Vec<T>,
    _marker: std::marker::PhantomData<U>,
}

impl<T, U> RepeatUntil<T, U> {
    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.items
    }

    #[must_use]
    pub fn into_items(self) -> Vec<T> {
        self.items
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<'a, T, U, I> Parse<'a, I> for RepeatUntil<T, U>
where
    I: crate::input::Input<'a>,
    T: Parse<'a, I>,
    U: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> crate::prelude::ParseResult<(Self, I)> {
        // Repeatedly parse T until U succeeds, without consuming input for U.
        let mut items = Vec::new();
        let mut rest = input;

        loop {
            match U::parse_with_context(rest, context) {
                Ok(_) => break,
                Err(err) if err.is_fatal() => return Err(err),
                Err(_) => {}
            }

            match T::parse_with_context(rest, context) {
                Ok((item, new_rest)) => {
                    crate::collections::ensure_progress(rest, new_rest, "RepeatUntil")?;
                    items.push(item);
                    rest = new_rest;
                }
                // Propagate T's error verbatim so callers keep span / root cause.
                Err(err) => return Err(err),
            }
        }
        Ok((
            RepeatUntil {
                items,
                _marker: std::marker::PhantomData,
            },
            rest,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parse::Parse,
        primitives::prelude::{AlphaString, DigitStr},
    };

    #[test]
    fn test_repeat_max_limit() {
        let input = "abc123def456ghi";
        let (result, rest) = Repeat::<char, 3, 5>::parse(input).unwrap();
        assert_eq!(result.into_items(), "abc12".chars().collect::<Vec<_>>());
        assert_eq!(rest, "3def456ghi");
    }

    #[test]
    fn test_repeat_min_not_met() {
        let input = "ab";
        let result = Repeat::<char, 3, 5>::parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_repeat_rejects_non_consuming_parser() {
        let result = Repeat::<(), 0, 5>::parse("abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_repeat_until() {
        let input = "abc123def456ghi";
        let (result, rest) = RepeatUntil::<AlphaString, DigitStr<'_>>::parse(input).unwrap();
        assert_eq!(result.into_items(), vec!["abc"]);
        assert_eq!(rest, "123def456ghi");
    }
}
