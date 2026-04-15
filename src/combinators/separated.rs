use crate::{error::ParseResult, literals::Comma, parse::Parse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Separated<T, S> {
    pub items: Vec<T>,
    pub separators: Vec<S>,
}

impl<T, S> Separated<T, S> {
    pub fn new(items: Vec<T>, separators: Vec<S>) -> Self {
        Self { items, separators }
    }
    pub fn items(&self) -> &[T] {
        &self.items
    }
    pub fn separators(&self) -> &[S] {
        &self.separators
    }
    pub fn push(&mut self, item: T, separator: Option<S>) {
        self.items.push(item);
        if let Some(sep) = separator {
            self.separators.push(sep);
        }
    }
    pub fn into_items(self) -> Vec<T> {
        self.items
    }
    pub fn into_separators(self) -> Vec<S> {
        self.separators
    }
    pub fn items_mut(&mut self) -> &mut Vec<T> {
        &mut self.items
    }
    pub fn separators_mut(&mut self) -> &mut Vec<S> {
        &mut self.separators
    }
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
        items.push(first_item);
        input = remaining;

        loop {
            // Try to parse a separator
            match S::parse(input) {
                Ok((sep, remaining)) => {
                    separators.push(sep);
                    input = remaining;
                }
                Err(_) => break, // No more separators, we're done
            }

            // Parse the next item
            let (item, remaining) = T::parse(input)?;
            items.push(item);
            input = remaining;
        }

        Ok((Separated { items, separators }, input))
    }
}

pub type CommaSeparated<T> = Separated<T, Comma>;
