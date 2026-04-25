use crate::{error::ParseResult, parse::Parse, prelude::Separated};

/// A combinator for left-associative operations, such as addition or multiplication.
///
/// Inspired by [parsel's `LeftAssoc`](https://docs.rs/parsel/latest/parsel/ast/enum.LeftAssoc.html)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LeftAssoc<O, R> {
    Binary {
        left: Box<LeftAssoc<O, R>>,
        operator: O,
        right: R,
    },
    Rhs(R),
}

impl<'a, I, O, R> Parse<'a, I> for LeftAssoc<O, R>
where
    I: crate::parse::ParseOffsetInput<'a>,
    O: Parse<'a, I>,
    R: Parse<'a, I>,
{
    #[inline]
    fn parse_with_context(
        input: I,
        _context: &mut crate::parse::ParseOffsetContext,
    ) -> ParseResult<(Self, I)> {
        let (seq, rest) = Separated::<R, O>::parse(input)?;
        let (rhs, operators) = seq.into_parts();
        let mut rhs = rhs.into_iter();
        let first = rhs.next().ok_or_else(|| {
            crate::error::ParseError::custom(
                "Expected at least one operand for left-associative operator",
            )
        })?;
        let mut expr = LeftAssoc::Rhs(first);
        for (operator, right) in operators.into_iter().zip(rhs) {
            expr = LeftAssoc::Binary {
                left: Box::new(expr),
                operator,
                right,
            };
        }
        Ok((expr, rest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::literals::Plus;

    #[test]
    fn test_left_assoc_single_operand() {
        // A single number with no operators should parse to Rhs
        let result = LeftAssoc::<Plus, u32>::parse("42");
        assert!(result.is_ok());
        let (parsed, rest) = result.unwrap();
        assert!(rest.is_empty());
        assert_eq!(parsed, LeftAssoc::Rhs(42));
    }

    #[test]
    fn test_left_assoc_two_operands() {
        // "1 + 2" should parse to Binary { left: Rhs(1), operator: +, right: 2 }
        let result = LeftAssoc::<Plus, u32>::parse("1+2");
        assert!(result.is_ok());
        let (parsed, rest) = result.unwrap();
        assert!(rest.is_empty());
        match parsed {
            LeftAssoc::Binary {
                left,
                operator: _,
                right,
            } => {
                assert_eq!(*left, LeftAssoc::Rhs(1));
                assert_eq!(right, 2);
            }
            _ => panic!("Expected Binary variant"),
        }
    }

    #[test]
    fn test_left_assoc_three_operands() {
        // "1 + 2 + 3" should parse as ((1 + 2) + 3) - left associative
        let result = LeftAssoc::<Plus, u32>::parse("1+2+3");
        assert!(result.is_ok());
        let (parsed, rest) = result.unwrap();
        assert!(rest.is_empty());

        // Should be: Binary { left: Binary { left: Rhs(1), op: +, right: 2 }, op: +, right: 3 }
        match &parsed {
            LeftAssoc::Binary {
                left,
                operator: _,
                right,
            } => {
                assert_eq!(*right, 3);
                match left.as_ref() {
                    LeftAssoc::Binary {
                        left,
                        operator: _,
                        right,
                    } => {
                        assert_eq!(**left, LeftAssoc::Rhs(1));
                        assert_eq!(*right, 2);
                    }
                    _ => panic!("Expected nested Binary for left operand"),
                }
            }
            _ => panic!("Expected Binary variant"),
        }
    }

    #[test]
    fn test_left_assoc_four_operands() {
        // "1 + 2 + 3 + 4" should parse as (((1 + 2) + 3) + 4)
        let result = LeftAssoc::<Plus, u32>::parse("1+2+3+4");
        assert!(result.is_ok());
        let (parsed, rest) = result.unwrap();
        assert!(rest.is_empty());

        // Verify left-associative structure: (((1+2)+3)+4)
        let mut current = &parsed;
        let mut expected_values = vec![4, 3, 2, 1];
        for expected_right in expected_values.drain(..3) {
            match current {
                LeftAssoc::Binary {
                    left,
                    operator: _,
                    right,
                } => {
                    assert_eq!(*right, expected_right);
                    current = left;
                }
                _ => panic!("Expected Binary variant"),
            }
        }
        // The innermost should be Rhs(1)
        assert_eq!(*current, LeftAssoc::Rhs(1));
    }

    #[test]
    fn test_left_assoc_with_trailing_input() {
        // "1+2 rest" should parse "1+2" and leave " rest"
        let result = LeftAssoc::<Plus, u32>::parse("1+2 rest");
        assert!(result.is_ok());
        let (parsed, rest) = result.unwrap();
        assert_eq!(rest, " rest");
        match parsed {
            LeftAssoc::Binary {
                left,
                operator: _,
                right,
            } => {
                assert_eq!(*left, LeftAssoc::Rhs(1));
                assert_eq!(right, 2);
            }
            _ => panic!("Expected Binary variant"),
        }
    }

    #[test]
    fn test_left_assoc_empty_input_fails() {
        // Empty input should fail because we need at least one operand
        let result = LeftAssoc::<Plus, u32>::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_left_assoc_operator_without_right_operand_fails() {
        // "1+" should fail because there's no right operand after the operator
        let result = LeftAssoc::<Plus, u32>::parse("1+");
        assert!(result.is_err());
    }

    #[test]
    fn test_left_assoc_operator_without_left_operand_fails() {
        // "+1" should fail because there's no left operand before the operator
        let result = LeftAssoc::<Plus, u32>::parse("+1");
        assert!(result.is_err());
    }

    #[test]
    fn test_left_assoc_preserves_left_associativity() {
        // Verify that "1+2+3+4+5" produces a left-associative tree
        // Structure should be: ((((1+2)+3)+4)+5)
        let result = LeftAssoc::<Plus, u32>::parse("1+2+3+4+5");
        assert!(result.is_ok());
        let (parsed, _) = result.unwrap();

        // Count the depth of the tree - should be 5 levels for 5 operands
        fn count_depth<O, R>(node: &LeftAssoc<O, R>) -> usize {
            match node {
                LeftAssoc::Rhs(_) => 1,
                LeftAssoc::Binary { left, .. } => 1 + count_depth(left),
            }
        }

        assert_eq!(count_depth(&parsed), 5);
    }

    #[test]
    fn test_left_assoc_with_large_numbers() {
        // Test with larger numbers to ensure parsing works correctly
        let input = format!("{}+{}+{}", u32::MAX, u32::MAX - 1, u32::MAX - 2);
        let result = LeftAssoc::<Plus, u32>::parse(input.as_str());
        assert!(result.is_ok());
        let (parsed, rest) = result.unwrap();
        assert!(rest.is_empty());

        match &parsed {
            LeftAssoc::Binary {
                left,
                operator: _,
                right,
            } => {
                assert_eq!(*right, u32::MAX - 2);
                match left.as_ref() {
                    LeftAssoc::Binary {
                        left,
                        operator: _,
                        right,
                    } => {
                        assert_eq!(**left, LeftAssoc::Rhs(u32::MAX));
                        assert_eq!(*right, u32::MAX - 1);
                    }
                    _ => panic!("Expected nested Binary"),
                }
            }
            _ => panic!("Expected Binary variant"),
        }
    }
}
