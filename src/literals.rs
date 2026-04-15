use crate::define_tokens;

define_tokens!(
    /// The `true` boolean literal token.
    KwTrue, "true";
    /// The `false` boolean literal token.
    KwFalse, "false";
    /// The `null` literal token.
    KwNull, "null";
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
