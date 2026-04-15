#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Many0<T> {
    items: Vec<T>,
}

impl<'a, T> crate::parse::Parse<'a> for Many0<T>
where
    T: crate::parse::Parse<'a>,
{
    fn parse(input: &'a str) -> crate::error::ParseResult<(Self, &'a str)> {
        let mut items = Vec::new();
        let mut rest = input;

        while let Ok((item, new_rest)) = T::parse(rest) {
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

impl<'a, T> crate::parse::Parse<'a> for Many1<T>
where
    T: crate::parse::Parse<'a>,
{
    fn parse(input: &'a str) -> crate::error::ParseResult<(Self, &'a str)> {
        let (first, mut rest) = T::parse(input)?;
        let mut items = Vec::new();
        items.push(first);

        while let Ok((item, new_rest)) = T::parse(rest) {
            items.push(item);
            rest = new_rest;
        }

        Ok((Many1 { items }, rest))
    }
}
