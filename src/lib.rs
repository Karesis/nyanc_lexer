mod tokens;
#[cfg(test)]
mod tests;

use nyanc_core::errors::{CompilerError, LexerError, LexerErrorKind};
use nyanc_core::{FileId, Span};
use reporter::DiagnosticsEngine;
use tokens::{Token, TokenType};
// 引入标准库的 Peekable 迭代器，这是我们实现预读功能的核心
use std::iter::Peekable;
use std::str::Chars;

/// Lexer 负责将源代码字符串分解为 Token 序列。
pub struct Lexer<'a> {
    diagnostics: &'a DiagnosticsEngine,
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

                ':' => {
                    if self.peek() == Some(':') {
                        self.advance(); // 消耗第二个 ':'
                        self.make_token(TokenType::DoubleColon)
                    } else {
                        self.make_token(TokenType::Colon)
                    }
                },
                
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

                // 单独捕获分号
                ';' => {
                    // 当我们捕获到分号时...
                    let err_span = Span { file_id: self.file_id, start: self.start_pos, end: self.current_pos };
                    let err = LexerError::new(LexerErrorKind::UnnecessarySemicolon, err_span);
                    self.diagnostics.add_error(CompilerError::Lexer(err));
                    self.make_token(TokenType::Illegal)
                }
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

    /// 扫描一个完整的数字字面量，可以是整数或浮点数
    fn scan_number(&mut self) -> Token {
        // 1. 扫描整数部分
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        // 2. 检查是否是浮点数
        // 我们需要预读两位：一个 '.' 和它后面的一个数字
        let mut is_float = false;
        if self.peek() == Some('.') {
            // 创建一个临时的克隆迭代器来预读两位
            let mut ahead = self.chars.clone();
            ahead.next(); // 跳过 '.'
            if let Some(next_c) = ahead.next() {
                if next_c.is_ascii_digit() {
                    is_float = true;
                    self.advance(); // 确认是浮点数，消耗 '.'

                    // 3. 扫描小数部分
                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            }
        }
        
        if is_float {
            self.make_token(TokenType::Float)
        } else {
            self.make_token(TokenType::Integer)
        }
    }

    /// 扫描一个完整的字符串字面量，支持多行和转义字符。
    fn scan_string(&mut self) -> Token {
        // 不断循环，直到找到闭合的引号或文件末尾
        loop {
            match self.peek() {
                // --- 找到闭合引号：成功路径 ---
                Some('"') => {
                    self.advance(); // 消耗闭合的 "
                    return self.make_token(TokenType::String);
                }
                // --- 文件结尾：错误路径 ---
                None => {
                    // 我们到达了文件末尾，但字符串没有闭合
                    // 错误应该从字符串的起始位置（self.start_pos）到当前位置
                    let err_span = Span { file_id: self.file_id, start: self.start_pos, end: self.current_pos };
                    let err = LexerError::new(LexerErrorKind::UnterminatedString, err_span);
                    self.diagnostics.add_error(CompilerError::Lexer(err));

                    // 返回一个 Illegal Token，让编译器知道这里出了问题
                    return self.make_token(TokenType::Illegal);
                }
                // --- 换行符：支持多行字符串 ---
                Some('\n') => {
                    self.advance();
                    self.line += 1;
                    self.column = 1;
                }
                // --- 转义字符处理 ---
                Some('\\') => {
                    self.advance(); // 消耗 '\'
                    
                    // 查看转义的字符是什么
                    match self.peek() {
                        Some('"' | '\\' | 'n' | 'r' | 't') => {
                            // 合法的转义，直接消耗掉后面的字符
                            self.advance();
                        }
                        Some(c) => {
                            // 无效的转义
                            let err_start = self.current_pos - 1; // 错误位置从 '\' 开始
                            self.advance(); // 消耗这个无效字符
                            let err_span = Span { file_id: self.file_id, start: err_start, end: self.current_pos };
                            let err = LexerError::new(LexerErrorKind::InvalidEscapeSequence(c), err_span);
                            self.diagnostics.add_error(CompilerError::Lexer(err));
                        }
                        None => {
                            // '\' 后面直接是文件结尾，也属于未闭合
                            let err_span = Span { file_id: self.file_id, start: self.start_pos, end: self.current_pos };
                            let err = LexerError::new(LexerErrorKind::UnterminatedString, err_span);
                            self.diagnostics.add_error(CompilerError::Lexer(err));
                            return self.make_token(TokenType::Illegal);
                        }
                    }
                }
                // --- 其他普通字符 ---
                _ => {
                    self.advance();
                }
            }
        }
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
            "true" | "false" => TokenType::Bool,
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
