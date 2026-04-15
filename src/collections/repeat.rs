#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Repeat<T, const MIN: usize, const MAX: usize> {
    items: Vec<T>,
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
    fn test_repeat() {
        let input = "abc123def456ghi";
        let (result, rest) = Repeat::<char, 3, 5>::parse(input).unwrap();
        assert_eq!(result.items, vec!['a', 'b', 'c', '1', '2']);
        assert_eq!(rest, "3def456ghi");
    }
}
