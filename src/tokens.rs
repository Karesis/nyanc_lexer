use nyanc_core::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenType,
    pub lexeme: String,
    pub span: Span,
}

// TokenType 枚举，轻量且可复制
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    // --- 标点与操作符 ---
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    Colon,
    Equal,
    Arrow,
    Caret,
    Ampersand,
    Dot,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,

    // --- 字面量 ---
    Identifier,
    Integer,
    String,

    // --- 关键字 ---
    Let,
    If,
    Else,
    While,
    Struct,
    Fun,
    Return,
    Pub,
    Use,

    // --- 特殊 Token ---
    Newline,
    Eof,
    Illegal,
}
