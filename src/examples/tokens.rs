//! Example token definitions for demonstration and testing purposes.
//!
//! This module provides common tokens that can be used as building blocks
//! for more complex parsers.

use crate::token::Token;
use crate::{define_tokens, lit_token};

// ============================================================================
// Keywords
// ============================================================================

define_tokens!(
    /// The `let` keyword token.
    KwLet, "let";
    /// The `=` operator token.
    KwEq, "=";
    /// The `;` semicolon token.
    Semi, ";";
    /// The `(` left parenthesis token.
    LParen, "(";
    /// The `)` right parenthesis token.
    RParen, ")";
);
