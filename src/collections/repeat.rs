#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Repeat<T, const MIN: usize, const MAX: usize> {
    items: Vec<T>,
}

impl<T, const MIN: usize, const MAX: usize> Repeat<T, MIN, MAX> {
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

impl<'a, T, const MIN: usize, const MAX: usize> crate::parse::Parse<'a> for Repeat<T, MIN, MAX>
where
    T: crate::parse::Parse<'a>,
{
    fn parse(input: &'a str) -> crate::error::ParseResult<(Self, &'a str)> {
        let mut items = Vec::new();
        let mut rest = input;

        while items.len() < MAX {
            match T::parse(rest) {
                Ok((item, new_rest)) => {
                    crate::collections::ensure_progress(rest, new_rest, "Repeat")?;
                    items.push(item);
                    rest = new_rest;
                }
                Err(_) => break,
            }
        }

        if items.len() < MIN {
            return Err(crate::error::ParseError::custom(format!(
                "Expected at least {} items, found {}",
                MIN,
                items.len()
            )));
        }

        Ok((Repeat { items }, rest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::Parse;

    #[test]
    fn test_repeat_max_limit() {
        let input = "abc123def456ghi";
        let (result, rest) = Repeat::<char, 3, 5>::parse(input).unwrap();
        assert_eq!(result.into_items(), "abc123".chars().collect::<Vec<_>>());
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
}
