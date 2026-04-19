use std::borrow::Cow;

use super::{BorrowInput, Input, shared};
use crate::error::ParseResult;
use stable_pattern::Pattern;

impl<'a> Input<'a> for &'a str {
    fn input_len(self) -> usize {
        self.len()
    }

    fn trim_start(self) -> Self {
        self.trim_start()
    }

    fn is_empty(self) -> bool {
        self.is_empty()
    }

    fn display(self) -> Cow<'a, str> {
        self.into()
    }

    fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>> {
        Ok(self.strip_prefix(prefix))
    }

    fn find(self, needle: &str) -> ParseResult<Option<Self>> {
        Ok(self.find(needle).map(|idx| &self[idx..]))
    }

    fn slice_to(self, end: Self) -> ParseResult<Self> {
        let consumed = shared::checked_consumed_len(self.len(), end.len(), "string")?;
        Ok(&self[..consumed])
    }

    fn take_char(self) -> ParseResult<Option<(char, Self)>> {
        let mut chars = self.chars();
        Ok(chars.next().map(|ch| (ch, chars.as_str())))
    }

    fn take_while<P>(self, predicate: P) -> ParseResult<(Cow<'a, str>, Self)>
    where
        P: for<'b> Pattern<'b> + Copy,
    {
        let (matched, rest) = self.take_while_borrowed(predicate)?;
        Ok((Cow::Borrowed(matched), rest))
    }

    fn take_till<P>(self, predicate: P) -> ParseResult<(Cow<'a, str>, Self)>
    where
        P: for<'b> Pattern<'b> + Copy,
    {
        let (matched, rest) = self.take_till_borrowed(predicate)?;
        Ok((Cow::Borrowed(matched), rest))
    }
}

impl<'a> BorrowInput<'a> for &'a str {
    fn take_while_borrowed<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: for<'b> Pattern<'b> + Copy,
    {
        let idx = shared::take_while_prefix_len(self, predicate);
        Ok((&self[..idx], &self[idx..]))
    }

    fn take_till_borrowed<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: for<'b> Pattern<'b> + Copy,
    {
        Ok(shared::split_take_till(self, predicate))
    }
}
