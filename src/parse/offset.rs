use crate::input::{Input, ReadInput, ReadInputStreamInput};
use std::cmp::Reverse;

/// Anchor metadata used to compute absolute parser offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseOffsetAnchor {
    start: usize,
    len: usize,
}

impl ParseOffsetAnchor {
    #[must_use]
    pub const fn new(start: usize, len: usize) -> Self {
        Self { start, len }
    }

    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    #[must_use]
    pub const fn len(self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub const fn end(self) -> usize {
        self.start.saturating_add(self.len)
    }
}

/// Input support required for offset tracking through parse context.
///
/// This trait lives outside of [`Input`] so parser position tracking remains an
/// opt-in concern managed by parse orchestration.
pub trait ParseOffsetInput<'a>: Input<'a> {
    /// Returns an anchor describing the current input view.
    fn parse_offset_anchor(self) -> ParseOffsetAnchor;

    /// Returns the absolute offset from a root anchor when this input is inside it.
    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize>;
}

/// Parse-scoped offset context passed through parse calls.
///
/// This context is explicitly owned by the parse entry point and passed to
/// nested parsers, avoiding any global state while remaining thread-safe.
#[derive(Debug, Default, Clone)]
pub struct ParseOffsetContext {
    roots: Vec<ParseOffsetAnchor>,
}

impl ParseOffsetContext {
    #[must_use]
    pub fn new() -> Self {
        Self { roots: Vec::new() }
    }

    fn has_scope_for<'a, I>(&self, input: I) -> bool
    where
        I: ParseOffsetInput<'a>,
    {
        self.roots
            .iter()
            .copied()
            .any(|root| input.parse_offset_from(root).is_some())
    }
}

pub(crate) fn with_parse_offset_scope<'a, I, R>(
    context: &mut ParseOffsetContext,
    input: I,
    parse: impl FnOnce(&mut ParseOffsetContext) -> R,
) -> R
where
    I: ParseOffsetInput<'a>,
{
    context.roots.push(input.parse_offset_anchor());
    let result = parse(context);
    context.roots.pop();
    result
}

pub(crate) fn with_parse_offset_scope_if_missing<'a, I, R>(
    context: &mut ParseOffsetContext,
    input: I,
    parse: impl FnOnce(&mut ParseOffsetContext) -> R,
) -> R
where
    I: ParseOffsetInput<'a>,
{
    if context.has_scope_for(input) {
        parse(context)
    } else {
        with_parse_offset_scope(context, input, parse)
    }
}

pub(crate) fn current_parse_offset<'a, I>(context: &ParseOffsetContext, input: I) -> usize
where
    I: ParseOffsetInput<'a>,
{
    context
        .roots
        .iter()
        .copied()
        .filter_map(|root| input.parse_offset_from(root).map(|offset| (offset, root)))
        .min_by_key(|(_, root)| (root.len, Reverse(root.start)))
        .map_or(0, |(offset, _)| offset)
}

fn byte_offset_from(start: usize, len: usize, root: ParseOffsetAnchor) -> Option<usize> {
    let current_end = start.saturating_add(len);
    let root_end = root.start.saturating_add(root.len);

    if start < root.start || current_end > root_end {
        return None;
    }

    Some(start.saturating_sub(root.start))
}

impl<'a> ParseOffsetInput<'a> for &'a str {
    fn parse_offset_anchor(self) -> ParseOffsetAnchor {
        ParseOffsetAnchor::new(self.as_ptr() as usize, self.len())
    }

    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize> {
        byte_offset_from(self.as_ptr() as usize, self.len(), root)
    }
}

impl<'a> ParseOffsetInput<'a> for &'a [u8] {
    fn parse_offset_anchor(self) -> ParseOffsetAnchor {
        ParseOffsetAnchor::new(self.as_ptr() as usize, self.len())
    }

    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize> {
        byte_offset_from(self.as_ptr() as usize, self.len(), root)
    }
}

impl<'a> ParseOffsetInput<'a> for ReadInput<'a> {
    fn parse_offset_anchor(self) -> ParseOffsetAnchor {
        ParseOffsetAnchor::new(self.as_bytes().as_ptr() as usize, self.as_bytes().len())
    }

    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize> {
        byte_offset_from(
            self.as_bytes().as_ptr() as usize,
            self.as_bytes().len(),
            root,
        )
    }
}

impl<'a, R, const N: usize> ParseOffsetInput<'a> for ReadInputStreamInput<'a, R, N>
where
    R: std::io::Read,
{
    fn parse_offset_anchor(self) -> ParseOffsetAnchor {
        ParseOffsetAnchor::new(self.absolute_start(), self.input_len())
    }

    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize> {
        byte_offset_from(self.absolute_start(), self.input_len(), root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_parse_offset_scope_if_missing_reuses_existing_scope() {
        let input = "abcdef";
        let sub = &input[2..];
        let mut context = ParseOffsetContext::new();

        with_parse_offset_scope(&mut context, input, |context| {
            let roots_before = context.roots.len();
            with_parse_offset_scope_if_missing(context, sub, |context| {
                assert_eq!(context.roots.len(), roots_before);
                assert_eq!(current_parse_offset(context, sub), 2);
            });
            assert_eq!(context.roots.len(), roots_before);
        });

        assert!(context.roots.is_empty());
    }

    #[test]
    fn test_with_parse_offset_scope_if_missing_pushes_when_unscoped() {
        let input = "abcdef";
        let sub = &input[2..];
        let mut context = ParseOffsetContext::new();

        with_parse_offset_scope_if_missing(&mut context, sub, |context| {
            assert_eq!(context.roots.len(), 1);
            assert_eq!(current_parse_offset(context, sub), 0);
        });

        assert!(context.roots.is_empty());
    }

    #[test]
    fn test_current_parse_offset_prefers_innermost_scope() {
        let input = "abcdef";
        let inner = &input[2..5];
        let inner_tail = &inner[1..];
        let mut context = ParseOffsetContext::new();

        with_parse_offset_scope(&mut context, input, |context| {
            assert_eq!(current_parse_offset(context, inner), 2);

            with_parse_offset_scope(context, inner, |context| {
                assert_eq!(current_parse_offset(context, inner), 0);
                assert_eq!(current_parse_offset(context, inner_tail), 1);
            });

            assert_eq!(current_parse_offset(context, inner_tail), 3);
        });
    }
}
