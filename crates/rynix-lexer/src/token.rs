use rynix_span::Span;

/// The kind of a lexical token.
///
/// `#[repr(u8)]` keeps [`Token`] at 12 bytes (`u8` kind + 8-byte span with
/// padding, `Copy`). Keywords are distinct variants so the parser matches on
/// a single discriminant instead of comparing strings.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum TokenKind {
    // --- Literals (3) -----------------------------------------------------
    IntLit,
    FloatLit,
    StrLit,

    // --- Identifier (1) ---------------------------------------------------
    Ident,

    // --- Keywords (26) ----------------------------------------------------
    Def,
    End,
    Let,
    Mut,
    If,
    Elif,
    Else,
    Loop,
    For,
    In,
    Break,
    Continue,
    Return,
    Struct,
    Enum,
    Type,
    Import,
    Pub,
    True,
    False,
    Nil,
    And,
    Or,
    Not,
    As,
    Spawn,

    // --- Reserved keywords (4), rejected in Phase 2+ ----------------------
    Match,
    Agent,
    Signal,
    Tensor,

    // --- Delimiters (6) ---------------------------------------------------
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,

    // --- Punctuation and operators (24) -----------------------------------
    Comma,
    Dot,
    DotDot,
    DotDotEq,
    Colon,
    ColonColon,
    Arrow,
    Eq,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,

    // --- Structural (6) ---------------------------------------------------
    /// One line terminator (`\n`, `\r\n`, or a lone `\r`): the statement
    /// separator. The parser ignores these inside bracketed groups.
    Newline,
    /// Spaces and tabs (trivia).
    Whitespace,
    /// `# ...` to end of line (trivia).
    Comment,
    /// `## ...` to end of line (significant: attached to the next item).
    DocComment,
    /// A byte or run that cannot start any token; carries `RYX0001`/`RYX0003`.
    Unknown,
    /// The zero-width end-of-input marker.
    Eof,
}

impl TokenKind {
    /// Trivia is skipped by the parser without affecting the grammar.
    #[inline]
    pub fn is_trivia(self) -> bool {
        matches!(self, TokenKind::Whitespace | TokenKind::Comment)
    }

    /// Whether this kind is a keyword (including reserved ones).
    #[inline]
    pub fn is_keyword(self) -> bool {
        (self as u8) >= (TokenKind::Def as u8) && (self as u8) <= (TokenKind::Tensor as u8)
    }

    /// Reserved-for-future-use keywords: lexed as keywords so that using one
    /// as an identifier is a clear error rather than a confusing parse.
    #[inline]
    pub fn is_reserved(self) -> bool {
        matches!(
            self,
            TokenKind::Match | TokenKind::Agent | TokenKind::Signal | TokenKind::Tensor
        )
    }

    /// The canonical source spelling for kinds with a fixed one, used by
    /// diagnostics and the future formatter.
    pub fn spelling(self) -> Option<&'static str> {
        use TokenKind::{
            Agent, And, Arrow, As, BangEq, Break, Colon, ColonColon, Comma, Continue, Def, Dot,
            DotDot, DotDotEq, Elif, Else, End, Enum, Eq, EqEq, False, For, Gt, GtEq, If, Import,
            In, LBrace, LBracket, LParen, Let, Loop, Lt, LtEq, Match, Minus, MinusEq, Mut, Nil,
            Not, Or, Percent, PercentEq, Plus, PlusEq, Pub, RBrace, RBracket, RParen, Return,
            Signal, Slash, SlashEq, Spawn, Star, StarEq, Struct, Tensor, True, Type,
        };
        Some(match self {
            Def => "def",
            End => "end",
            Let => "let",
            Mut => "mut",
            If => "if",
            Elif => "elif",
            Else => "else",
            Loop => "loop",
            For => "for",
            In => "in",
            Break => "break",
            Continue => "continue",
            Return => "return",
            Struct => "struct",
            Enum => "enum",
            Type => "type",
            Import => "import",
            Pub => "pub",
            True => "true",
            False => "false",
            Nil => "nil",
            And => "and",
            Or => "or",
            Not => "not",
            As => "as",
            Spawn => "spawn",
            Match => "match",
            Agent => "agent",
            Signal => "signal",
            Tensor => "tensor",
            LParen => "(",
            RParen => ")",
            LBracket => "[",
            RBracket => "]",
            LBrace => "{",
            RBrace => "}",
            Comma => ",",
            Dot => ".",
            DotDot => "..",
            DotDotEq => "..=",
            Colon => ":",
            ColonColon => "::",
            Arrow => "->",
            Eq => "=",
            EqEq => "==",
            BangEq => "!=",
            Lt => "<",
            LtEq => "<=",
            Gt => ">",
            GtEq => ">=",
            Plus => "+",
            Minus => "-",
            Star => "*",
            Slash => "/",
            Percent => "%",
            PlusEq => "+=",
            MinusEq => "-=",
            StarEq => "*=",
            SlashEq => "/=",
            PercentEq => "%=",
            _ => return None,
        })
    }

    /// A short human-facing description used in diagnostics.
    pub fn describe(self) -> &'static str {
        match self {
            TokenKind::IntLit => "integer literal",
            TokenKind::FloatLit => "float literal",
            TokenKind::StrLit => "string literal",
            TokenKind::Ident => "identifier",
            TokenKind::Newline => "newline",
            TokenKind::Whitespace => "whitespace",
            TokenKind::Comment => "comment",
            TokenKind::DocComment => "doc comment",
            TokenKind::Unknown => "unknown token",
            TokenKind::Eof => "end of file",
            other => other.spelling().unwrap_or("token"),
        }
    }
}

/// A lexical token: a kind plus the source range it covers.
///
/// Tokens never own text; the text is always a slice of the memory-mapped
/// source (`SourceMap::span_text`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    #[inline]
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }

    #[inline]
    pub fn is_trivia(self) -> bool {
        self.kind.is_trivia()
    }

    #[inline]
    pub fn is_eof(self) -> bool {
        self.kind == TokenKind::Eof
    }
}

/// Maps an identifier's bytes to its keyword kind, if any.
///
/// Dispatching on length first keeps this to at most a handful of byte
/// comparisons; the compiler turns each arm into a fixed-width memcmp.
#[inline]
pub(crate) fn keyword_kind(bytes: &[u8]) -> Option<TokenKind> {
    use TokenKind::{
        Agent, And, As, Break, Continue, Def, Elif, Else, End, Enum, False, For, If, Import, In,
        Let, Loop, Match, Mut, Nil, Not, Or, Pub, Return, Signal, Spawn, Struct, Tensor, True,
        Type,
    };
    Some(match bytes.len() {
        2 => match bytes {
            b"if" => If,
            b"in" => In,
            b"or" => Or,
            b"as" => As,
            _ => return None,
        },
        3 => match bytes {
            b"def" => Def,
            b"end" => End,
            b"let" => Let,
            b"mut" => Mut,
            b"for" => For,
            b"pub" => Pub,
            b"nil" => Nil,
            b"and" => And,
            b"not" => Not,
            _ => return None,
        },
        4 => match bytes {
            b"elif" => Elif,
            b"else" => Else,
            b"loop" => Loop,
            b"true" => True,
            b"type" => Type,
            b"enum" => Enum,
            _ => return None,
        },
        5 => match bytes {
            b"break" => Break,
            b"false" => False,
            b"spawn" => Spawn,
            b"match" => Match,
            b"agent" => Agent,
            _ => return None,
        },
        6 => match bytes {
            b"return" => Return,
            b"struct" => Struct,
            b"import" => Import,
            b"signal" => Signal,
            b"tensor" => Tensor,
            _ => return None,
        },
        8 => match bytes {
            b"continue" => Continue,
            _ => return None,
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_twelve_bytes() {
        assert_eq!(std::mem::size_of::<Token>(), 12);
        assert_eq!(std::mem::size_of::<TokenKind>(), 1);
    }

    #[test]
    fn keyword_table_round_trips_every_keyword() {
        // Every kind with a keyword spelling must be recognized from bytes,
        // and every recognized keyword must map back to the same kind.
        let all = [
            TokenKind::Def,
            TokenKind::End,
            TokenKind::Let,
            TokenKind::Mut,
            TokenKind::If,
            TokenKind::Elif,
            TokenKind::Else,
            TokenKind::Loop,
            TokenKind::For,
            TokenKind::In,
            TokenKind::Break,
            TokenKind::Continue,
            TokenKind::Return,
            TokenKind::Struct,
            TokenKind::Enum,
            TokenKind::Type,
            TokenKind::Import,
            TokenKind::Pub,
            TokenKind::True,
            TokenKind::False,
            TokenKind::Nil,
            TokenKind::And,
            TokenKind::Or,
            TokenKind::Not,
            TokenKind::As,
            TokenKind::Spawn,
            TokenKind::Match,
            TokenKind::Agent,
            TokenKind::Signal,
            TokenKind::Tensor,
        ];
        assert_eq!(all.len(), 30, "26 keywords + 4 reserved");
        for kind in all {
            let text = kind.spelling().expect("keyword has a spelling");
            assert_eq!(keyword_kind(text.as_bytes()), Some(kind), "{text}");
            assert!(kind.is_keyword(), "{text} must classify as a keyword");
        }
    }

    #[test]
    fn non_keywords_are_not_recognized() {
        for word in [
            "", "x", "de", "defx", "End", "WHILE", "while", "async", "await", "fn", "func",
        ] {
            assert_eq!(keyword_kind(word.as_bytes()), None, "{word}");
        }
    }

    #[test]
    fn classification_helpers() {
        assert!(TokenKind::Whitespace.is_trivia());
        assert!(TokenKind::Comment.is_trivia());
        assert!(
            !TokenKind::DocComment.is_trivia(),
            "doc comments are significant"
        );
        assert!(
            !TokenKind::Newline.is_trivia(),
            "newlines terminate statements"
        );
        assert!(TokenKind::Tensor.is_reserved());
        assert!(!TokenKind::Def.is_reserved());
        assert!(!TokenKind::Ident.is_keyword());
        assert_eq!(TokenKind::IntLit.describe(), "integer literal");
        assert_eq!(TokenKind::Arrow.describe(), "->");
    }
}
