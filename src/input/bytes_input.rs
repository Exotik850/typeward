use std::borrow::Cow;

use super::{Input, shared};
use crate::error::ParseResult;
use stable_pattern::Pattern;

impl<'a> Input<'a> for &'a [u8] {
    fn input_len(self) -> usize {
        self.len()
    }

    fn trim_start(self) -> ParseResult<Self> {
        let start = self
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .unwrap_or(self.len());
        Ok(&self[start..])
    }

    fn is_empty(self) -> bool {
        self.is_empty()
    }

    fn display(self) -> Cow<'a, str> {
        String::from_utf8_lossy(self)
    }

    fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>> {
        let s = shared::utf8(self)?;
        Ok(s.strip_prefix(prefix)
            .map(|rest| &self[s.len() - rest.len()..]))
    }

    fn find(self, needle: &str) -> ParseResult<Option<Self>> {
        let s = shared::utf8(self)?;
        Ok(s.find(needle).map(|idx| &self[idx..]))
    }

    fn slice_to(self, end: Self) -> ParseResult<Self> {
        let consumed = shared::checked_consumed_len(self.len(), end.len(), "byte")?;
        Ok(&self[..consumed])
    }

    fn take_char(self) -> ParseResult<Option<(char, Self)>> {
        let s = shared::utf8(self)?;
        let mut chars = s.chars();
        Ok(chars.next().map(|ch| (ch, &self[ch.len_utf8()..])))
    }

    fn take_while<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy,
    {
        let s = shared::utf8(self)?;
        let idx = shared::take_while_prefix_len(s, predicate);
        Ok((&s[..idx], &self[idx..]))
    }

    fn take_till<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy,
    {
        let s = shared::utf8(self)?;
        let (matched, rest) = shared::split_take_till(s, predicate);
        Ok((matched, rest.as_bytes()))
    }

    fn empty() -> Self {
        &[]
    }
}
