#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Many0<T> {
    items: Vec<T>,
}

impl<T> Many0<T> {
    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn into_items(self) -> Vec<T> {
        self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<'a, T> crate::parse::Parse<'a> for Many0<T>
where
    T: crate::parse::Parse<'a>,
{
    fn parse(input: &'a str) -> crate::error::ParseResult<(Self, &'a str)> {
        let mut items = Vec::new();
        let mut rest = input;

        while let Ok((item, new_rest)) = T::parse(rest) {
            crate::collections::ensure_progress(rest, new_rest, "Many0")?;
            items.push(item);
            rest = new_rest;
        }

        Ok((Many0 { items }, rest))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Many1<T> {
    items: Vec<T>,
}

impl<T> Many1<T> {
    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn into_items(self) -> Vec<T> {
        self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn first(&self) -> &T {
        // Safe: `Many1` guarantees at least one item
        &self.items[0]
    }
}

impl<'a, T> crate::parse::Parse<'a> for Many1<T>
where
    T: crate::parse::Parse<'a>,
{
    fn parse(input: &'a str) -> crate::error::ParseResult<(Self, &'a str)> {
        let (first, mut rest) = T::parse(input)?;
        crate::collections::ensure_progress(input, rest, "Many1")?;

        let mut items = Vec::new();
        items.push(first);

        while let Ok((item, new_rest)) = T::parse(rest) {
            crate::collections::ensure_progress(rest, new_rest, "Many1")?;
            items.push(item);
            rest = new_rest;
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
        assert_eq!(result.items(), &['a', 'b', 'c', '1', '2', '3']);
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
