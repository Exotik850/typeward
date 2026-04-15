use crate::{error::ParseResult, parse::Parse};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Default)]
pub struct And<A, B> {
    pub left: A,
    pub right: B,
}

impl<A, B> And<A, B> {
    pub fn new(left: A, right: B) -> Self {
        Self { left, right }
    }
    pub fn left(&self) -> &A {
        &self.left
    }
    pub fn right(&self) -> &B {
        &self.right
    }
    pub fn left_mut(&mut self) -> &mut A {
        &mut self.left
    }
    pub fn right_mut(&mut self) -> &mut B {
        &mut self.right
    }
    pub fn into_parts(self) -> (A, B) {
        (self.left, self.right)
    }
}

impl<A, B, C> And<A, And<B, C>> {
    pub fn flatten(self) -> (A, B, C) {
        (self.left, self.right.left, self.right.right)
    }
}

impl<'a, I, A, B> Parse<'a, I> for And<A, B>
where
    I: crate::input::Input<'a>,
    A: Parse<'a, I>,
    B: Parse<'a, I>,
{
    fn parse(input: I) -> ParseResult<(Self, I)> {
        let (left, remaining) = A::parse(input)?;
        let remaining = remaining.trim_start()?;
        let (right, remaining) = B::parse(remaining)?;
        Ok((And { left, right }, remaining))
    }
}

#[macro_export]
macro_rules! and {
    ($a:expr, $b:expr) => {
        $crate::combinators::and::And<$a, $b>
    };
    ($a:expr, $($rest:expr),+) => {
        $crate::combinators::and::And<$a, $crate::and!($($rest),+)>
    };
}

#[macro_export]
macro_rules! new_and {
    ($a:expr, $b:expr) => {
        $crate::combinators::and::And::new($a, $b)
    };
    ($a:expr, $($rest:expr),+) => {
        $crate::combinators::and::And::new($a, $crate::new_and!($($rest),+))
    };
}

#[macro_export]
macro_rules! unpack_and {
    ($val:expr, $single:ty $(,)?) => {
        ($val,)
    };
    ($val:expr, $head:ty, $($tail:ty),+ $(,)?) => {
        $crate::unpack_and!(@collect $val; ; $head, $($tail),+)
    };
    (@collect $val:expr; $($out:expr,)* ; $head:ty, $($tail:ty),+) => {{
        let $crate::combinators::and::And { left, right } = $val;
        $crate::unpack_and!(@collect right; $($out,)* left, ; $($tail),+)
    }};
    (@collect $val:expr; $($out:expr,)* ; $last:ty) => {
        ($($out,)* $val)
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn unpack_and_two_values() {
        let value = crate::new_and!(1_u8, 2_u16);
        let tuple: (u8, u16) = crate::unpack_and!(value, u8, u16);
        assert_eq!(tuple, (1, 2));
    }

    #[test]
    fn unpack_and_three_values() {
        let value = crate::new_and!(1_u8, 2_u16, 3_u32);
        let tuple: (u8, u16, u32) = crate::unpack_and!(value, u8, u16, u32);
        assert_eq!(tuple, (1, 2, 3));
    }

    #[test]
    fn unpack_and_four_values() {
        let value = crate::new_and!(1_u8, 2_u16, 3_u32, 4_u64);
        let tuple: (u8, u16, u32, u64) = crate::unpack_and!(value, u8, u16, u32, u64);
        assert_eq!(tuple, (1, 2, 3, 4));
    }
}
