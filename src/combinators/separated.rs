use crate::{error::ParseResult, literals::Comma, parse::Parse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Separated<T, S> {
    pub items: Vec<T>,
    pub separators: Vec<S>,
}

impl<T, S> Separated<T, S> {
    #[must_use]
    pub fn new(items: Vec<T>, separators: Vec<S>) -> Self {
        Self { items, separators }
    }
    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.items
    }
    #[must_use]
    pub fn separators(&self) -> &[S] {
        &self.separators
    }
    #[must_use]
    pub fn into_items(self) -> Vec<T> {
        self.items
    }
    #[must_use]
    pub fn into_separators(self) -> Vec<S> {
        self.separators
    }
    #[must_use]
    pub fn into_parts(self) -> (Vec<T>, Vec<S>) {
        (self.items, self.separators)
    }
}

impl<'a, T, S> Parse<'a> for Separated<T, S>
where
    T: Parse<'a>,
    S: Parse<'a>,
{
    fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
        let mut items = Vec::new();
        let mut separators = Vec::new();
        let mut input = input;

        // Parse the first item
        let (first_item, remaining) = T::parse(input)?;
        if input.len() == remaining.len() {
            return Err(crate::error::ParseError::custom(
                "Separated item parser matched without consuming input",
            ));
        }
        items.push(first_item);
        input = remaining;

        loop {
            // Try to parse a separator
            match S::parse(input) {
                Ok((sep, remaining)) => {
                    if input.len() == remaining.len() {
                        return Err(crate::error::ParseError::custom(
                            "Separated separator parser matched without consuming input",
                        ));
                    }
                    separators.push(sep);
                    input = remaining;
                }
                Err(_) => break, // No more separators, we're done
            }

            // Parse the next item
            let (item, remaining) = T::parse(input)?;
            if input.len() == remaining.len() {
                return Err(crate::error::ParseError::custom(
                    "Separated item parser matched without consuming input",
                ));
            }
            items.push(item);
            input = remaining;
        }

        Ok((Separated { items, separators }, input))
    }
}

pub type CommaSeparated<T> = Separated<T, Comma>;

#[cfg(test)]
mod tests {
    use crate::{parse::Parse, primitives::AlphaString};

    use super::*;

    #[test]
    fn separated_parses_items_and_separators() {
        let (result, rest) = CommaSeparated::<AlphaString>::parse("a,b,c;").unwrap();
        assert_eq!(result.items().len(), 3);
        assert_eq!(result.separators().len(), 2);
        assert_eq!(rest, ";");
    }

    #[test]
    fn separated_rejects_non_consuming_item() {
        let result = Separated::<(), char>::parse("abc");
        assert!(result.is_err());
    }

    #[test]
    fn separated_rejects_non_consuming_separator() {
        let result = Separated::<char, ()>::parse("abc");
        assert!(result.is_err());
    }
}
