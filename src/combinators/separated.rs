use crate::{error::ParseResult, literals::Comma, parse::Parse, prelude::Or};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeparatedIterState {
    First,
    Item,
    Separator,
    Done,
}

#[derive(Debug)]
pub struct SeparatedIter<'a, I, T, S> {
    input: I,
    context: &'a mut crate::parse::ParseOffsetContext,
    state: SeparatedIterState,
    _marker: std::marker::PhantomData<(T, S)>,
}

impl<'a, I, T, S> SeparatedIter<'a, I, T, S> {
    pub fn new(input: I, context: &'a mut crate::parse::ParseOffsetContext) -> Self {
        Self {
            input,
            context,
            state: SeparatedIterState::First,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'input, I, T, S> Iterator for SeparatedIter<'_, I, T, S>
where
    I: crate::input::Input<'input>,
    T: Parse<'input, I>,
    S: Parse<'input, I>,
{
    type Item = ParseResult<Or<Or<T, S>, I>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            SeparatedIterState::First | SeparatedIterState::Item => {
                match T::parse_with_context(self.input, self.context) {
                    Ok((item, rest)) => {
                        if let Err(e) =
                            crate::collections::ensure_progress(self.input, rest, "SeparatedIter")
                        {
                            return Some(Err(e));
                        }
                        self.input = rest;
                        self.state = SeparatedIterState::Separator;
                        Some(Ok(Or::Left(Or::Left(item))))
                    }
                    Err(err) if err.is_fatal() => Some(Err(err)),
                    Err(_) => None,
                }
            }
            SeparatedIterState::Separator => {
                match S::parse_with_context(self.input, self.context) {
                    Ok((sep, rest)) => {
                        if let Err(e) =
                            crate::collections::ensure_progress(self.input, rest, "SeparatedIter")
                        {
                            return Some(Err(e));
                        }
                        self.input = rest;
                        self.state = SeparatedIterState::Item;
                        Some(Ok(Or::Left(Or::Right(sep))))
                    }
                    Err(err) if err.is_fatal() => Some(Err(err)),
                    Err(_) => {
                        self.state = SeparatedIterState::Done;
                        Some(Ok(Or::Right(self.input)))
                    }
                }
            }
            SeparatedIterState::Done => None,
        }
    }
}

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

impl<'a, I, T, S> Parse<'a, I> for Separated<T, S>
where
    I: crate::input::Input<'a>,
    T: Parse<'a, I>,
    S: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let mut items = Vec::new();
        let mut separators = Vec::new();
        let mut input = input;
        for result in SeparatedIter::new(input, context) {
            match result {
                Ok(Or::Left(Or::Left(item))) => items.push(item),
                Ok(Or::Left(Or::Right(sep))) => separators.push(sep),
                Ok(Or::Right(rest)) => {
                    input = rest;
                    break;
                }
                Err(err) => return Err(err),
            }
        }
        if separators.len() >= items.len() {
            let preview = crate::error::preview_input(
                input.display().as_ref(),
                crate::error::DEFAULT_INPUT_PREVIEW,
            );
            return Err(crate::error::ParseError::custom(format!(
                "expected an item but found a separator at '{preview}'",
            )));
        }

        Ok((Separated { items, separators }, input))
    }
}

pub type CommaSeparated<T> = Separated<T, Comma>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Separated0<T, S> {
    pub items: Vec<T>,
    pub separators: Vec<S>,
}

impl<T, S> Separated0<T, S> {
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

impl<'a, I, T, S> Parse<'a, I> for Separated0<T, S>
where
    I: crate::input::Input<'a>,
    T: Parse<'a, I>,
    S: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let mut items = Vec::new();
        let mut separators = Vec::new();

        for result in SeparatedIter::new(input, context) {
            match result {
                Ok(Or::Left(Or::Left(item))) => items.push(item),
                Ok(Or::Left(Or::Right(sep))) => separators.push(sep),
                Ok(Or::Right(rest)) => {
                    return Ok((Separated0 { items, separators }, rest));
                }
                Err(err) => return Err(err),
            }
        }
        Ok((Separated0 { items, separators }, input))
    }
}

pub type CommaSeparated0<T> = Separated0<T, Comma>;

#[cfg(test)]
mod tests {
    use crate::{parse::Parse, primitives::filtered::AlphaString};

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

    #[test]
    fn separated0_parses_empty_sequence() {
        let (result, rest) = Separated0::<AlphaString, Comma>::parse(";").unwrap();
        assert!(result.items().is_empty());
        assert!(result.separators().is_empty());
        assert_eq!(rest, ";");
    }

    #[test]
    fn separated0_parses_non_empty_sequence() {
        let (result, rest) = Separated0::<AlphaString, Comma>::parse("a,b,c;").unwrap();
        assert_eq!(result.items().len(), 3);
        assert_eq!(result.separators().len(), 2);
        assert_eq!(rest, ";");
    }
}
