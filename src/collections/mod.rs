pub mod collect;
pub mod many;
pub mod repeat;

pub use collect::Collect;
pub use many::{Many0, Many1};
pub use repeat::{Repeat, RepeatUntil};

use crate::input::Input;

pub(crate) fn ensure_progress<'a, I>(
    before: I,
    after: I,
    combinator: &str,
) -> crate::error::ParseResult<()>
where
    I: Input<'a>,
{
    if before.input_len() == after.input_len() {
        return Err(crate::error::ParseError::custom(format!(
            "{combinator} parser matched without consuming input"
        )));
    }

    Ok(())
}

