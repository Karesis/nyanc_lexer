mod tokens;

use nyanc_core::errors::{CompilerError, LexerError};
use nyanc_core::{FileId, Span};
use reporter::DiagnosticsEngine;
use tokens::{Token, TokenType};
// 引入标准库的 Peekable 迭代器，这是我们实现预读功能的核心
use std::iter::Peekable;
use std::str::Chars;

// 暂时还没用到 CompilationContext，但先把架子搭好
// use nyanc::CompilationContext;

/// Lexer 负责将源代码字符串分解为 Token 序列。
pub struct Lexer<'a> {
    diagnostics: &'a DiagnosticsEngine,
    // context: &'a CompilationContext, // 暂时注释掉，直到我们开始处理错误
    source: &'a str,            // 完整的源代码引用，用于从 Span 中提取 lexeme
    chars: Peekable<Chars<'a>>, // 带有预读能力的字符迭代器

    file_id: FileId, // 当前正在处理的文件 ID

    // --- 位置追踪 ---
    /// 扫描器在整个源文件字符串中的当前位置（字节索引）
    current_pos: usize,
    /// 当前正在扫描的 Token 的起始位置
    start_pos: usize,

    /// 当前行号 (从 1 开始)
    line: u32,
    /// 当前 Token 在当前行中的起始列号 (从 1 开始)
    column: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, file_id: FileId, diagnostics: &'a DiagnosticsEngine) -> Self {
        Self {
            diagnostics,
            // context,
            source,
            chars: source.chars().peekable(),
            file_id,
            current_pos: 0,
            start_pos: 0,
            line: 1,
            column: 1,
        }
    }

    /// 工具函数：消耗当前字符，并向前移动一个位置。
    /// 返回被消耗的字符。
    fn advance(&mut self) -> Option<char> {
        match self.chars.next() {
            Some(c) => {
                self.current_pos += c.len_utf8(); // 支持 UTF-8
                self.column += 1;
                Some(c)
            }
            None => None,
        }
    }

    /// 工具函数：预读（查看）下一个字符，但不消耗它。
    fn peek(&mut self) -> Option<char> {
        // .peek() 返回的是 &char，我们需要拷贝它
        self.chars.peek().copied()
    }

    /// 工具函数：根据当前位置和 Token 类型，创建一个完整的 Token。
    fn make_token(&self, kind: TokenType) -> Token {
        let lexeme = &self.source[self.start_pos..self.current_pos];
        let span = Span {
            file_id: self.file_id,
            start: self.start_pos,
            end: self.current_pos,
        };
        Token {
            kind,
            lexeme: lexeme.to_string(), // 约定 MVP 阶段使用 String
            span,
        }
    }

    /// 这是 Lexer 的心脏。它遵循“准备->标记->分派->返回”的节律。
    pub fn next_token(&mut self) -> Token {
        // 1. 准备：跳过空白
        self.skip_whitespace();

        // 2. 标记起点
        self.start_pos = self.current_pos;

        // 3. 识别与分派
        if let Some(c) = self.advance() {
            match c {
                // --- 单字符 Token ---
                '{' => self.make_token(TokenType::LeftBrace),
                '}' => self.make_token(TokenType::RightBrace),
                '(' => self.make_token(TokenType::LeftParen),
                ')' => self.make_token(TokenType::RightParen),
                ':' => self.make_token(TokenType::Colon),
                '=' => self.make_token(TokenType::Equal),
                '^' => self.make_token(TokenType::Caret),
                '&' => self.make_token(TokenType::Ampersand),
                '.' => self.make_token(TokenType::Dot),
                ',' => self.make_token(TokenType::Comma),
                '+' => self.make_token(TokenType::Plus),
                '*' => self.make_token(TokenType::Star),
                '/' => self.make_token(TokenType::Slash),

                // --- 可能的多字符 Token ---
                '-' => {
                    if self.peek() == Some('>') {
                        self.advance(); // 消耗 '>'
                        self.make_token(TokenType::Arrow)
                    } else {
                        self.make_token(TokenType::Minus)
                    }
                }

                // --- 换行符 ---
                '\n' => {
                    let token = self.make_token(TokenType::Newline);
                    self.line += 1;
                    self.column = 1; // 新的一行，列号重置为 1
                    token
                }

                // --- 复杂模式：分派给专门的扫描函数 ---
                '"' => self.scan_string(),
                c if c.is_ascii_digit() => self.scan_number(),
                c if c.is_alphabetic() || c == '_' => self.scan_identifier(),

                // --- 未知字符 ---
                _ => self.make_token(TokenType::Illegal),
            }
        } else {
            // 4. 文件末尾
            let span = Span {
                file_id: self.file_id,
                start: self.current_pos,
                end: self.current_pos,
            };
            Token {
                kind: TokenType::Eof,
                lexeme: "".to_string(),
                span,
            }
        }
    }

    /// 跳过所有连续的空白字符（不包括换行符）
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// 扫描一个完整的数字字面量
    fn scan_number(&mut self) -> Token {
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        self.make_token(TokenType::Integer)
    }

    /// 扫描一个完整的字符串字面量
    fn scan_string(&mut self) -> Token {
        // 简单实现：扫描直到下一个 "
        // 注意：这个实现没有处理转义字符 `\"` 或字符串跨行的情况
        while let Some(c) = self.peek() {
            if c != '"' {
                self.advance();
            } else {
                break;
            }
        }

        // 如果没有闭合的引号，这是一个错误，但我们暂时简化处理
        if self.peek() == Some('"') {
            self.advance(); // 消耗末尾的 "
        } else {
            // TODO: 报告一个未闭合的字符串错误
        }

        self.make_token(TokenType::String)
    }

    /// 扫描一个完整的标识符，并检查它是否是关键字
    fn scan_identifier(&mut self) -> Token {
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let lexeme = &self.source[self.start_pos..self.current_pos];
        let kind = match lexeme {
            "fun" => TokenType::Fun,
            "let" => TokenType::Let,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "while" => TokenType::While,
            "struct" => TokenType::Struct,
            "return" => TokenType::Return,
            "pub" => TokenType::Pub,
            "use" => TokenType::Use,
            _ => TokenType::Identifier,
        };
        self.make_token(kind)
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token();
        if token.kind == TokenType::Eof {
            None // 迭代器结束
        } else {
            Some(token)
        }
    }
}
