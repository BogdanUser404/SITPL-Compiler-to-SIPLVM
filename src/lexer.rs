//! Лексер, преобразующий исходный код в последовательность токенов.
//! Отслеживает отступы и генерирует токены Indent/Dedent.

use crate::ast::{BinOp, UnOp};

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // Ключевые слова
    Def, Var, If, Else, While, Return, Print, Input, SetOutput,
    True, False,
    // Типы (unit variants)
    IntType, FloatType, BoolType, StringType, CharType,
    // Литералы
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    CharLit(char),
    // Идентификаторы
    Identifier(String),
    // Операторы и пунктуация
    Assign, Plus, Minus, Star, Slash,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or, Not,
    LParen, RParen, LBrace, RBrace,
    LBracket, RBracket,
    Colon, Comma, Semicolon,
    // Специальные токены для отступов
    Newline,
    Indent,
    Dedent,
    Eof,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    indent_stack: Vec<usize>,
    tokens: Vec<Token>,
    line_start: bool,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            indent_stack: vec![0],
            tokens: Vec::new(),
            line_start: true,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += 1;
        Some(ch)
    }

    fn scan_number(&mut self, first: char) -> Token {
        let mut num_str = first.to_string();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '.' {
                num_str.push(ch);
                self.pos += 1;
            } else {
                break;
            }
        }
        if num_str.contains('.') {
            Token::FloatLit(num_str.parse().unwrap())
        } else {
            Token::IntLit(num_str.parse().unwrap())
        }
    }

    fn scan_identifier(&mut self, first: char) -> Token {
        let mut ident = first.to_string();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.pos += 1;
            } else {
                break;
            }
        }
        match ident.as_str() {
            "def" => Token::Def,
            "var" => Token::Var,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "return" => Token::Return,
            "print" => Token::Print,
            "input" => Token::Input,
            "set_output" => Token::SetOutput,
            "true" => Token::True,
            "false" => Token::False,
            "int" => Token::IntType,
            "float" => Token::FloatType,
            "bool" => Token::BoolType,
            "string" => Token::StringType,
            "char" => Token::CharType,
            _ => Token::Identifier(ident),
        }
    }

    fn scan_string(&mut self) -> Token {
        let mut s = String::new();
        while let Some(ch) = self.next_char() {
            if ch == '"' {
                break;
            }
            if ch == '\\' {
                if let Some(next) = self.next_char() {
                    match next {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        _ => s.push(next),
                    }
                }
            } else {
                s.push(ch);
            }
        }
        Token::StringLit(s)
    }

    fn scan_char(&mut self) -> Token {
        let ch = self.next_char().unwrap();
        if self.next_char() != Some('\'') {
            panic!("Invalid character literal");
        }
        Token::CharLit(ch)
    }

    fn handle_indent(&mut self, spaces: usize) {
        let current = *self.indent_stack.last().unwrap();
        if spaces > current {
            self.indent_stack.push(spaces);
            self.tokens.push(Token::Indent);
        } else {
            while spaces < *self.indent_stack.last().unwrap() {
                self.indent_stack.pop();
                self.tokens.push(Token::Dedent);
            }
        }
    }

    pub fn tokenize(mut self) -> Vec<Token> {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                self.pos += 1;
                self.line_start = true;
                self.tokens.push(Token::Newline);
                continue;
            }
            if self.line_start {
                let mut spaces = 0;
                while let Some(c) = self.peek() {
                    if c == ' ' {
                        spaces += 1;
                        self.pos += 1;
                    } else if c == '\t' {
                        spaces += 4; // упрощение
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                self.handle_indent(spaces);
                self.line_start = false;
                continue;
            }
            if ch.is_whitespace() && ch != '\n' {
                self.pos += 1;
                continue;
            }

            let token = match ch {
                '0'..='9' => self.scan_number(ch),
                'a'..='z' | 'A'..='Z' | '_' => self.scan_identifier(ch),
                '"' => { self.pos += 1; self.scan_string() }
                '\'' => { self.pos += 1; self.scan_char() }
                '=' => {
                    self.pos += 1;
                    if self.peek() == Some('=') {
                        self.pos += 1;
                        Token::Eq
                    } else {
                        Token::Assign
                    }
                }
                '!' => {
                    self.pos += 1;
                    if self.peek() == Some('=') {
                        self.pos += 1;
                        Token::Ne
                    } else {
                        Token::Not
                    }
                }
                '<' => {
                    self.pos += 1;
                    if self.peek() == Some('=') {
                        self.pos += 1;
                        Token::Le
                    } else {
                        Token::Lt
                    }
                }
                '>' => {
                    self.pos += 1;
                    if self.peek() == Some('=') {
                        self.pos += 1;
                        Token::Ge
                    } else {
                        Token::Gt
                    }
                }
                '&' => {
                    self.pos += 1;
                    if self.peek() == Some('&') {
                        self.pos += 1;
                        Token::And
                    } else {
                        panic!("Unexpected '&'");
                    }
                }
                '|' => {
                    self.pos += 1;
                    if self.peek() == Some('|') {
                        self.pos += 1;
                        Token::Or
                    } else {
                        panic!("Unexpected '|'");
                    }
                }
                '+' => { self.pos += 1; Token::Plus }
                '-' => { self.pos += 1; Token::Minus }
                '*' => { self.pos += 1; Token::Star }
                '/' => { self.pos += 1; Token::Slash }
                '(' => { self.pos += 1; Token::LParen }
                ')' => { self.pos += 1; Token::RParen }
                '{' => { self.pos += 1; Token::LBrace }
                '}' => { self.pos += 1; Token::RBrace }
                '[' => { self.pos += 1; Token::LBracket }
                ']' => { self.pos += 1; Token::RBracket }
                ':' => { self.pos += 1; Token::Colon }
                ',' => { self.pos += 1; Token::Comma }
                ';' => { self.pos += 1; Token::Semicolon }
                _ => panic!("Unexpected character: {}", ch),
            };
            self.tokens.push(token);
        }
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            self.tokens.push(Token::Dedent);
        }
        self.tokens.push(Token::Eof);
        self.tokens
    }
}