pub mod collect;
pub mod many;
pub mod repeat;

pub(crate) fn ensure_progress(before: &str, after: &str, combinator: &str) -> crate::error::ParseResult<()> {
    if before.len() == after.len() {
        return Err(crate::error::ParseError::custom(format!(
            "{combinator} parser matched without consuming input"
        )));
    }

    Ok(())
}

pub mod prelude {
    pub use super::collect::Collect;
    pub use super::many::{Many0, Many1};
    pub use super::repeat::Repeat;
}
