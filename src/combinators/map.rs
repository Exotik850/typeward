/// A parser that applies `F` to the output of `T` to get `U`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Map<T, U, F = IntoMap> {
    value: U,
    _marker: std::marker::PhantomData<(T, F)>,
}

/// A trait for mapping a value of type `T` to a value of type `U`.
pub trait MapFunction<T, U> {
    fn map(value: T) -> U;
}

/// A default mapping function that uses `From` to convert `T` into `U`.
pub struct IntoMap;
impl<T, U> MapFunction<T, U> for IntoMap
where
    U: From<T>,
{
    fn map(value: T) -> U {
        U::from(value)
    }
}

impl<T, U, F> Map<T, U, F> {
    pub fn new(value: U) -> Self {
        Self {
            value,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn into_inner(self) -> T
    where
        U: Into<T>,
    {
        self.value.into()
    }
}

impl<T, U, F> std::ops::Deref for Map<T, U, F> {
    type Target = U;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T, U: PartialEq<U>, F> PartialEq<U> for Map<T, U, F> {
    fn eq(&self, other: &U) -> bool {
        &self.value == other
    }
}

impl<'a, I, T, U, F> crate::parse::Parse<'a, I> for Map<T, U, F>
where
    I: crate::parse::ParseOffsetInput<'a>,
    T: crate::parse::Parse<'a, I>,
    F: MapFunction<T, U>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        context: &mut crate::parse::ParseOffsetContext,
    ) -> crate::error::ParseResult<(Self, I)> {
        let (value, rest) = T::parse_with_context(input, context)?;
        Ok((Self::new(F::map(value)), rest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::Parse;

    #[test]
    fn test_map() {
        let input = "123";
        let (result, rest) = Map::<u16, u32>::parse(input).unwrap();
        assert_eq!(*result, 123);
        assert_eq!(rest, "");
    }

    #[test]
    fn test_map_custom() {
        struct DoubleMap;
        impl MapFunction<u16, u32> for DoubleMap {
            fn map(value: u16) -> u32 {
                (value as u32) * 2
            }
        }

        let input = "123";
        let (result, rest) = Map::<u16, u32, DoubleMap>::parse(input).unwrap();
        assert_eq!(*result, 246);
        assert_eq!(rest, "");
    }
}
