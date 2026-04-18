use super::ParseOffsetContext;
use crate::error::ParseResult;
use crate::input::Input;
use std::borrow::Cow;
use std::marker::PhantomData;

/// A trait for types that can be parsed from an abstract input.
///
/// This is the main trait that structs should implement to become parseable.
/// The lifetime parameter `'a` represents the lifetime of the borrowed input.
///
/// The second generic parameter defaults to `&str`, which keeps string parsing
/// ergonomic while allowing additional input forms such as `&[u8]` and token
/// slices.
pub trait Parse<'a, I: Input<'a> = &'a str>: Sized {
    /// Parse a value from the input.
    ///
    /// Returns the parsed value and the remaining unconsumed input.
    fn parse(input: I) -> ParseResult<(Self, I)> {
        let mut context = ParseOffsetContext::new();
        Self::parse_with_context(input, &mut context)
    }

    /// Parse a value from the input using an explicit offset context.
    fn parse_with_context(input: I, context: &mut ParseOffsetContext) -> ParseResult<(Self, I)>;
}

impl<'a, I> Parse<'a, I> for ()
where
    I: Input<'a>,
{
    fn parse_with_context(input: I, _context: &mut ParseOffsetContext) -> ParseResult<(Self, I)> {
        Ok(((), input))
    }
}

impl<'a, I, T> Parse<'a, I> for Option<T>
where
    I: Input<'a>,
    T: Parse<'a, I>,
{
    fn parse_with_context(input: I, context: &mut ParseOffsetContext) -> ParseResult<(Self, I)> {
        match T::parse_with_context(input, context) {
            Ok((value, remaining)) => Ok((Some(value), remaining)),
            Err(_) => Ok((None, input)),
        }
    }
}

/// Generate an impl based on a collection type,
/// and the function to push items into it.
macro_rules! parse_collection {
    ($ty:ty, $push_fn:ident $(; $($bound:path),*)?) => {
        impl<'a, I, T> Parse<'a, I> for $ty
        where
            I: Input<'a>,
            T: Parse<'a, I> $(+ $($bound +)*)?,
        {
            fn parse_with_context(input: I, context: &mut ParseOffsetContext) -> ParseResult<(Self, I)> {
                let mut items = Self::new();
                let mut input = input;
                while let Ok((item, remaining)) = T::parse_with_context(input, context) {
                    items.$push_fn(item);
                    input = remaining;
                }
                Ok((items, input))
            }
        }
    };
}

parse_collection!(Vec<T>, push);
parse_collection!(std::collections::HashSet<T>, insert; std::hash::Hash, Eq);
parse_collection!(std::collections::BTreeSet<T>, insert; Ord);
parse_collection!(std::collections::VecDeque<T>, push_back);
parse_collection!(std::collections::BinaryHeap<T>, push; Ord);

macro_rules! parse_wrapper {
    ($ty:ty $(; $($bound:ident),*)?) => {
        impl<'a, T, I> Parse<'a, I> for $ty
        where
            I: Input<'a>,
            T: Parse<'a, I> $($(+ $bound)*)?,
        {
            fn parse_with_context(
                input: I,
                context: &mut ParseOffsetContext,
            ) -> ParseResult<(Self, I)> {
                let (value, remaining) = T::parse_with_context(input, context)?;
                Ok((Self::from(value), remaining))
            }
        }
    };
}

parse_wrapper!(std::rc::Rc<T>);
parse_wrapper!(std::sync::Arc<T>);
parse_wrapper!(std::cell::Cell<T>);
parse_wrapper!(std::cell::RefCell<T>);
parse_wrapper!(std::cell::OnceCell<T>);
parse_wrapper!(std::sync::Mutex<T>);
parse_wrapper!(std::sync::RwLock<T>);
parse_wrapper!(std::sync::OnceLock<T>);

macro_rules! parse_tuple {
    ($($ty:ident),+) => {
        impl<'a, I, $($ty),+> Parse<'a, I> for ($($ty),+)
        where
            I: Input<'a>,
            $($ty: Parse<'a, I>),+
        {
            fn parse_with_context(
                mut input: I,
                context: &mut ParseOffsetContext,
            ) -> ParseResult<(Self, I)> {
                #[allow(non_snake_case)]
                let ($($ty),+) = (
                    $(
                        {
                            let (value, remaining) = $ty::parse_with_context(input, context)?;
                            input = remaining;
                            value
                        }
                    ),+
                );
                Ok((($($ty),+), input))
            }
        }
    };
}

parse_tuple!(A, B);
parse_tuple!(A, B, C);
parse_tuple!(A, B, C, D);
parse_tuple!(A, B, C, D, E);
parse_tuple!(A, B, C, D, E, F);
parse_tuple!(A, B, C, D, E, F, G);
parse_tuple!(A, B, C, D, E, F, G, H);
parse_tuple!(A, B, C, D, E, F, G, H, K);
parse_tuple!(A, B, C, D, E, F, G, H, K, J);

impl<'a, I, T> Parse<'a, I> for Cow<'a, T>
where
    I: Input<'a>,
    T: ToOwned + ?Sized + 'a,
    &'a T: Parse<'a, I>,
{
    fn parse_with_context(input: I, context: &mut ParseOffsetContext) -> ParseResult<(Self, I)> {
        let (value, remaining) = <&'a T>::parse_with_context(input, context)?;
        Ok((Cow::Borrowed(value), remaining))
    }
}

/// A wrapper type for nested parsing results,
///
/// allows for parsers to return nested structures without losing the ability to implement `Parse` for the inner type.
/// This was chosen instead of a blanket impl over `Box<T: Parse>`
/// since downstream users may want to implement `Parse` for `Box<T>` directly for some types, and this allows them to do so without conflicting with the blanket impl.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Nested<T>(Box<T>);

impl<'a, T, I> Parse<'a, I> for Nested<T>
where
    I: Input<'a>,
    T: Parse<'a, I>,
{
    fn parse_with_context(input: I, context: &mut ParseOffsetContext) -> ParseResult<(Self, I)> {
        let (value, remaining) = T::parse_with_context(input, context)?;
        Ok((Nested(Box::new(value)), remaining))
    }
}

impl<T> std::ops::Deref for Nested<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Nested<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, I, T> Parse<'a, I> for PhantomData<T>
where
    I: Input<'a>,
{
    fn parse_with_context(input: I, _context: &mut ParseOffsetContext) -> ParseResult<(Self, I)> {
        Ok((PhantomData, input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combinators::ws::Ws;
    use crate::input::Input;
    use crate::lit_token;

    lit_token!(HelloParser, "hello");

    #[test]
    fn test_option_parse_does_not_consume_input_on_failure() {
        let input = "not a number";
        let mut context = ParseOffsetContext::new();
        let (result, remaining) = Option::<i64>::parse_with_context(input, &mut context).unwrap();
        assert!(result.is_none());
        assert_eq!(remaining, input);
    }

    #[test]
    fn test_vec_parse_accumulates_all_successful_items() {
        let input = "1 2 three 4";
        let mut context = ParseOffsetContext::new();
        let (result, remaining) = Vec::<i64>::parse_with_context(input, &mut context).unwrap();
        assert_eq!(result, vec![1, 2]);
        assert_eq!(remaining, " three 4");
    }

    #[test]
    fn test_binary_heap_parse() {
        let input = "3 1 4 1 5";
        let mut context = ParseOffsetContext::new();
        let (result, remaining) =
            std::collections::BinaryHeap::<i64>::parse_with_context(input, &mut context).unwrap();
        let sorted: Vec<_> = result.into_sorted_vec();
        assert_eq!(sorted, vec![1, 1, 3, 4, 5]);
        assert!(remaining.trim().is_empty());
    }

    #[test]
    fn test_tuple_parse() {
        let input = "42 hello";
        let mut context = ParseOffsetContext::new();
        let (result, remaining) =
            <(i64, Ws<HelloParser>)>::parse_with_context(input, &mut context).unwrap();
        assert_eq!(result.0, 42);
        assert_eq!(result.1, HelloParser);
        assert_eq!(remaining, "");
    }

    #[test]
    fn test_nested_parses_recursive() {
        struct Recursive {
            value: Ws<i64>,
            inner: Option<Nested<Recursive>>,
        }

        impl<'a, I> Parse<'a, I> for Recursive
        where
            I: Input<'a>,
        {
            fn parse_with_context(
                input: I,
                context: &mut ParseOffsetContext,
            ) -> ParseResult<(Self, I)> {
                let (value, remaining) = Ws::<i64>::parse_with_context(input, context)?;
                let (inner, remaining) =
                    Option::<Nested<Recursive>>::parse_with_context(remaining, context)?;
                Ok((Recursive { value, inner }, remaining))
            }
        }

        let input = "1 2 3";
        let (parsed, remaining) = Recursive::parse(input).unwrap();
        assert_eq!(parsed.value, 1);
        let inner1 = parsed.inner.unwrap().0;
        assert_eq!(inner1.value, 2);
        let inner2 = inner1.inner.unwrap().0;
        assert_eq!(inner2.value, 3);
        assert!(inner2.inner.is_none());
        assert!(remaining.trim().is_empty());
    }
}
