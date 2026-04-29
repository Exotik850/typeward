use std::marker::PhantomData;

use crate::{error::ParseResult, parse::Parse};

#[derive(Debug)]
pub struct PIter<'a, I, T> {
    input: I,
    context: &'a mut crate::parse::ParseOffsetContext,
    name: &'static str,
    _marker: PhantomData<T>,
}

pub fn iter<'a, I, T>(
    input: I,
    context: &'a mut crate::parse::ParseOffsetContext,
    name: &'static str,
) -> PIter<'a, I, T> {
    PIter {
        input,
        context,
        name,
        _marker: PhantomData,
    }
}

impl<'input, I, T> Iterator for PIter<'_, I, T>
where
    I: crate::input::Input<'input>,
    T: Parse<'input, I>,
{
    type Item = ParseResult<(T, I)>;

    fn next(&mut self) -> Option<Self::Item> {
        match T::parse_with_context(self.input, self.context) {
            Ok((item, rest)) => {
                if let Err(e) = crate::collections::ensure_progress(self.input, rest, self.name) {
                    return Some(Err(e.into_fatal()));
                }
                self.input = rest;
                Some(Ok((item, rest)))
            }
            Err(err) if err.is_fatal() => Some(Err(err)),
            Err(_) => None,
        }
    }
}
