pub type Empty = ();

/// A parser that always succeeds and consumes the rest of the input.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rest<I> {
    value: I,
}

impl<'a, I> crate::parse::Parse<'a, I> for Rest<I>
where
    I: crate::input::Input<'a>,
{
    fn parse(value: I) -> crate::error::ParseResult<(Self, I)> {
        Ok((Rest { value }, I::empty()))
    }
}

/// A parser that checks that the input is empty, and fails if it is not.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Eof;

impl<'a, I> crate::parse::Parse<'a, I> for Eof
where
    I: crate::input::Input<'a>,
{
    fn parse(input: I) -> crate::error::ParseResult<(Self, I)> {
        if input.is_empty() {
            Ok((Eof, input))
        } else {
            Err(crate::error::custom(format!(
                "Expected end of input, but found '{}'",
                input.display()
            )))
        }
    }
}

/// A parser that always fails.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fail;

impl<'a, I> crate::parse::Parse<'a, I> for Fail
where
    I: crate::input::Input<'a>,
{
    fn parse(_: I) -> crate::error::ParseResult<(Self, I)> {
        Err(crate::error::custom("Expected Fail to always fail"))
    }
}
