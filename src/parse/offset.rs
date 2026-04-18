use crate::input::{Input, TokenStream};
use std::{cmp::Reverse, mem};

/// Domain for offset tracking units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseOffsetDomain {
    /// Byte offsets (`&str`, `&[u8]`).
    Bytes,
    /// Element offsets (`TokenStream`).
    Tokens,
}

/// Anchor metadata used to compute absolute parser offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseOffsetAnchor {
    domain: ParseOffsetDomain,
    start: usize,
    len: usize,
    unit_size: usize,
}

impl ParseOffsetAnchor {
    #[must_use]
    pub const fn new(
        domain: ParseOffsetDomain,
        start: usize,
        len: usize,
        unit_size: usize,
    ) -> Self {
        Self {
            domain,
            start,
            len,
            unit_size,
        }
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

impl<'a> ParseOffsetInput<'a> for &'a str {
    fn parse_offset_anchor(self) -> ParseOffsetAnchor {
        ParseOffsetAnchor::new(
            ParseOffsetDomain::Bytes,
            self.as_ptr() as usize,
            self.len(),
            1,
        )
    }

    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize> {
        if root.domain != ParseOffsetDomain::Bytes || root.unit_size != 1 {
            return None;
        }

        let current_start = self.as_ptr() as usize;
        let current_end = current_start.saturating_add(self.len());
        let root_end = root.start.saturating_add(root.len);

        if current_start < root.start || current_end > root_end {
            return None;
        }

        Some(current_start.saturating_sub(root.start))
    }
}

impl<'a> ParseOffsetInput<'a> for &'a [u8] {
    fn parse_offset_anchor(self) -> ParseOffsetAnchor {
        ParseOffsetAnchor::new(
            ParseOffsetDomain::Bytes,
            self.as_ptr() as usize,
            self.len(),
            1,
        )
    }

    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize> {
        if root.domain != ParseOffsetDomain::Bytes || root.unit_size != 1 {
            return None;
        }

        let current_start = self.as_ptr() as usize;
        let current_end = current_start.saturating_add(self.len());
        let root_end = root.start.saturating_add(root.len);

        if current_start < root.start || current_end > root_end {
            return None;
        }

        Some(current_start.saturating_sub(root.start))
    }
}

impl<'a, T> ParseOffsetInput<'a> for TokenStream<'a, T>
where
    T: AsRef<str>,
{
    fn parse_offset_anchor(self) -> ParseOffsetAnchor {
        ParseOffsetAnchor::new(
            ParseOffsetDomain::Tokens,
            self.as_slice().as_ptr() as usize,
            self.as_slice().len(),
            mem::size_of::<T>(),
        )
    }

    fn parse_offset_from(self, root: ParseOffsetAnchor) -> Option<usize> {
        if root.domain != ParseOffsetDomain::Tokens || root.unit_size != mem::size_of::<T>() {
            return None;
        }

        let current_tokens = self.as_slice();
        let current_start = current_tokens.as_ptr() as usize;
        let current_len = current_tokens.len();

        if root.unit_size == 0 {
            if current_start != root.start || current_len > root.len {
                return None;
            }
            return Some(root.len.saturating_sub(current_len));
        }

        let byte_diff = current_start.checked_sub(root.start)?;
        if byte_diff % root.unit_size != 0 {
            return None;
        }

        let consumed = byte_diff / root.unit_size;
        if consumed.saturating_add(current_len) > root.len {
            return None;
        }

        Some(consumed)
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
