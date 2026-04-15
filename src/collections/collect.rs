use crate::{error::ParseResult, parse::Parse};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Collect<T, C> {
    items: C,
    _marker: std::marker::PhantomData<T>,
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
                rest = new_rest;
                Some(item)
            }
            Err(_) => None,
        });

        Ok((
            Collect {
                items: iter.collect(),
                _marker: std::marker::PhantomData,
            },
            rest,
        ))
    }
}
