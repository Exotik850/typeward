pub type Many0<T> = Vec<T>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Many1<T> {
    items: Vec<T>,
}

impl<T> Many1<T> {
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

    #[must_use]
    pub fn first(&self) -> &T {
        // Invariant: `Many1::parse_with_context` only constructs the value
        // after pushing the first successful item, so `items` is never empty.
        self.items
            .first()
            .expect("Many1 invariant: at least one item")
    }

    #[must_use]
    pub fn last(&self) -> &T {
        self.items
            .last()
            .expect("Many1 invariant: at least one item")
    }

    /// Convert to the underlying `Vec<T>`.
    ///
    /// Guaranteed to be non-empty; prefer [`Self::into_items`] when semantic
    /// emphasis on the non-empty invariant is desirable.
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.items
    }
}

impl<'a, I, T> crate::parse::Parse<'a, I> for Many1<T>
where
    I: crate::input::Input<'a>,
    T: crate::parse::Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> crate::error::ParseResult<(Self, I)> {
        let (first, mut rest) = T::parse_with_context(input, context)?;
        crate::collections::ensure_progress(input, rest, "Many1")?;

        let mut items = Vec::new();
        items.push(first);

        loop {
            match T::parse_with_context(rest, context) {
                Ok((item, new_rest)) => {
                    crate::collections::ensure_progress(rest, new_rest, "Many1")?;
                    items.push(item);
                    rest = new_rest;
                }
                Err(err) if err.is_fatal() => return Err(err),
                Err(_) => break,
            }
        }

        Ok((Many1 { items }, rest))
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::Parse;

    use super::*;

    #[test]
    fn many0_collects_until_failure() {
        let (result, rest) = Many0::<char>::parse("abc123").unwrap();
        assert_eq!(result, ['a', 'b', 'c', '1', '2', '3']);
        assert_eq!(rest, "");
    }

    #[test]
    fn many1_requires_at_least_one_match() {
        let result = Many1::<i64>::parse("abc");
        assert!(result.is_err());
    }

    #[test]
    fn many0_rejects_non_consuming_parser() {
        let result = Many0::<()>::parse("input");
        assert!(result.is_err());
    }

    #[test]
    fn many1_rejects_non_consuming_parser() {
        let result = Many1::<()>::parse("input");
        assert!(result.is_err());
    }
}
