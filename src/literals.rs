use crate::{define_tokens, prelude::Kw};

define_tokens!(
    /// The `true` boolean literal token.
    True, "true";
    /// The `false` boolean literal token.
    False, "false";
    /// The `null` literal token.
    Null, "null";
    /// The `(` left parenthesis token.
    LParen, "(";
    /// The `)` right parenthesis token.
    RParen, ")";
    /// The `[` left bracket token.
    LBracket, "[";
    /// The `]` right bracket token.
    RBracket, "]";
    /// The `{` left brace token.
    LBrace, "{";
    /// The `}` right brace token.
    RBrace, "}";
    /// The `.` dot token.
    Dot, ".";
    /// The `,` comma token.
    Comma, ",";
    /// The `:` colon token.
    Colon, ":";
    /// The `;` semicolon token.
    Semi, ";";
    /// The `=` equals token.
    Eq, "=";
    /// The `+` plus token.
    Plus, "+";
    /// The `-` minus token.
    Minus, "-";
    /// The `*` asterisk token.
    Star, "*";
    /// The `/` slash token.
    Slash, "/";
    /// The `!` exclamation token.
    Bang, "!";
    /// The `&` ampersand token.
    Amp, "&";
    /// The `|` pipe token.
    Pipe, "|";
    /// The `<` less-than token.
    Lt, "<";
    /// The `>` greater-than token.
    Gt, ">";
    /// The `?` question mark token.
    Question, "?";
    /// The `@` at symbol token.
    At, "@";
    /// The `#` hash symbol token.
    Hash, "#";
    /// The `$` dollar sign token.
    Dollar, "$";
    /// The `%` percent sign token.
    Percent, "%";
    /// The `^` caret token.
    Caret, "^";
    /// The `\t` tab character token.
    Tab, "\t";
    /// The `\n` newline character token.
    Newline, "\n";
    /// The `\r` carriage return character token.
    CarriageReturn, "\r";
    /// The `\0` null character token.
    NullChar, "\0";
    /// The `\` backslash character token.
    Backslash, "\\";
    /// The `"` double quote character token.
    DoubleQuote, "\"";
    /// The `\r\n` Clrf newline sequence token.
    CrLf, "\r\n";
    /// The ` ` space character token.
    Space, " ";
);

pub type KwTrue = Kw<True>;
pub type KwFalse = Kw<False>;
pub type KwNull = Kw<Null>;

/// A helper macro for defining single-character tokens, which can be parsed directly from the input without needing to match a longer string.
macro_rules! define_char_token {
    ($($ch:tt),*) => {
        $(
            pastey::paste! {
                define_tokens!(
                    #[doc = "The `" $ch "` character token."]
                    #[allow(non_camel_case_types)]
                    [<$ch:replace("\"", "") Char>], $ch;
                );
            }
        )*
    };
}

define_char_token!(
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s",
    "t", "u", "v", "w", "x", "y", "z", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L",
    "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z"
);
