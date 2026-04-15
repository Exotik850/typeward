use crate::{error::ParseResult, parse::Parse};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Collect<T, C> {
    items: C,
    _marker: std::marker::PhantomData<T>,
}

impl<T, C> Collect<T, C> {
    pub fn items(&self) -> &C {
        &self.items
    }

    pub fn into_items(self) -> C {
        self.items
    }
}

impl<'a, T, C> Parse<'a> for Collect<T, C>
where
    T: Parse<'a>,
    C: FromIterator<T>,
{
    fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
        let mut rest = input;

        let iter = std::iter::from_fn(|| match T::parse(rest) {
            Ok((item, new_rest)) => {
                if let Err(err) = crate::collections::ensure_progress(rest, new_rest, "Collect") {
                    return Some(Err(err));
                }
                rest = new_rest;
                Some(Ok(item))
            }
            Err(_) => None,
        });

        let items: C = iter.collect::<ParseResult<C>>()?;

        Ok((
            Collect {
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
    use crate::parse::Parse;
    use crate::primitives::*;

    #[test]
    fn test_collect() {
        let input = "hello";
        let (result, rest) = Collect::<AlphaString, Vec<AlphaString>>::parse(input).unwrap();
        assert_eq!(
            result.items(),
            &vec![AlphaString {
                value: "hello".to_string()
            }]
        );
        assert_eq!(rest, "");
    }

    #[test]
    fn test_collect_rejects_non_consuming_parser() {
        let result = Collect::<(), Vec<()>>::parse("hello");
        assert!(result.is_err());
    }
}
