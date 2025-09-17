use super::*;
use reporter::DiagnosticsEngine;

/// 这是一个测试辅助函数，用于简化测试用例的编写。
/// 它接收一段源代码和期望的 Token 类型序列，然后断言 Lexer 的输出是否与之匹配。
fn check_lexing(source: &str, expected_tokens: &[TokenType]) {
    let diagnostics = DiagnosticsEngine::new();
    // file_id 在测试中可以是任意的，比如 0
    let lexer = Lexer::new(source, 0, &diagnostics);

    let mut generated_tokens = Vec::new();
    for token in lexer {
        generated_tokens.push(token.kind);
    }

    // 使用 assert_eq! 来比较生成的 Token 序列和期望的序列
    assert_eq!(generated_tokens, expected_tokens, "Mismatch between generated and expected tokens for source: '{}'", source);
    // 我们也断言在“快乐路径”测试中不应该有任何错误
    assert!(!diagnostics.has_errors(), "Lexer reported unexpected errors for source: '{}'", source);
}

// --- 现在开始编写具体的测试用例 ---

#[test]
fn test_single_char_tokens() {
    let source = "(){}:.,+*^&";
    let expected = &[
        TokenType::LeftParen, TokenType::RightParen,
        TokenType::LeftBrace, TokenType::RightBrace,
        TokenType::Colon, TokenType::Dot, TokenType::Comma,
        TokenType::Plus, TokenType::Star, TokenType::Caret,
        TokenType::Ampersand,
    ];
    check_lexing(source, expected);
}

#[test]
fn test_multi_char_tokens() {
    let source = "-> :: =";
    let expected = &[
        TokenType::Arrow,
        TokenType::DoubleColon,
        TokenType::Equal,
    ];
    check_lexing(source, expected);
}

#[test]
fn test_keywords_and_identifiers() {
    let source = "fun my_var = struct";
    let expected = &[
        TokenType::Fun,
        TokenType::Identifier,
        TokenType::Equal,
        TokenType::Struct,
    ];
    check_lexing(source, expected);
}

#[test]
fn test_variable_declarations() {
    let source = "a: bool = true\ncount: int = 123";
    let expected = &[
        // a: bool = true
        TokenType::Identifier, // "a"
        TokenType::Colon,      // ":"
        TokenType::Identifier, // "bool" (这是一个类型名，在词法阶段也被认为是标识符)
        TokenType::Equal,      // "="
        TokenType::Bool,       // "true"
        TokenType::Newline,    // "\n"

        // count: int = 123
        TokenType::Identifier, // "count"
        TokenType::Colon,      // ":"
        TokenType::Identifier, // "int"
        TokenType::Equal,      // "="
        TokenType::Integer,    // "123"
    ];
    check_lexing(source, expected);
}


#[test]
fn test_numbers() {
    let source = "123 9876 3.14159 0.5";
    let expected = &[
        TokenType::Integer,
        TokenType::Integer,
        TokenType::Float,
        TokenType::Float,
    ];
    check_lexing(source, expected);
}

#[test]
fn test_string_literal() {
    let source = r#" "hello\nworld" "#; // 使用 Rust 的原始字符串字面量来写测试，很方便
    let expected = &[TokenType::String];
    check_lexing(source, expected);
}

#[test]
fn test_a_simple_function() {
    let source = r#"
        fun add(a: int) -> int {
            return a + 1
        }
    "#;
    let expected = &[
        TokenType::Newline,
        TokenType::Fun, TokenType::Identifier, TokenType::LeftParen,
        TokenType::Identifier, TokenType::Colon, TokenType::Identifier,
        TokenType::RightParen, TokenType::Arrow, TokenType::Identifier,
        TokenType::LeftBrace, TokenType::Newline,
        TokenType::Return, TokenType::Identifier, TokenType::Plus, TokenType::Integer,
        TokenType::Newline,
        TokenType::RightBrace, TokenType::Newline,
    ];
    check_lexing(source, expected);
}

// --- 错误处理的测试 ---

#[test]
fn test_unterminated_string() {
    let source = r#" "hello world "#;
    let diagnostics = DiagnosticsEngine::new();
    let lexer = Lexer::new(source, 0, &diagnostics);

    // 消耗掉整个迭代器来触发所有可能的错误
    let tokens: Vec<Token> = lexer.collect();
    
    // 我们期望只生成一个 Illegal Token
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenType::Illegal);

    // 最重要的是，我们断言诊断引擎捕获到了一个错误
    assert!(diagnostics.has_errors(), "Lexer failed to report an error for an unterminated string.");
    
    // 如果需要，我们还可以进一步检查错误的类型
    // let errors = diagnostics.errors.borrow();
    // assert_matches!(&errors[0], CompilerError::Lexer(LexerError { kind: LexerErrorKind::UnterminatedString, .. }));
}