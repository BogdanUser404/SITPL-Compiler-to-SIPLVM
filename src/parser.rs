//! Парсер, строящий AST из потока токенов.

use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let tok = self.peek().cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if let Some(tok) = self.next() {
            if tok == expected {
                Ok(())
            } else {
                Err(format!("Expected {:?}, found {:?}", expected, tok))
            }
        } else {
            Err("Unexpected end of file".to_string())
        }
    }

    fn skip_newlines(&mut self) {
        while let Some(Token::Newline) = self.peek() {
            self.next();
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut functions = Vec::new();
        let mut globals = Vec::new();

        while let Some(tok) = self.peek() {
            match tok {
                Token::Def => {
                    let func = self.parse_function()?;
                    functions.push(func);
                }
                Token::Var | Token::If | Token::While | Token::Print | Token::Input | Token::SetOutput | Token::Identifier(_) => {
                    let stmt = self.parse_stmt()?;
                    globals.push(stmt);
                }
                Token::Eof => break,
                _ => return Err(format!("Unexpected token at top level: {:?}", tok)),
            }
            self.skip_newlines();
        }
        Ok(Program { functions, globals })
    }

    fn parse_function(&mut self) -> Result<Function, String> {
        self.expect(Token::Def)?;
        let name = match self.next() {
            Some(Token::Identifier(id)) => id,
            _ => return Err("Expected function name".to_string()),
        };
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        while let Some(tok) = self.peek() {
            if *tok == Token::RParen { break; }
            let param_name = match self.next() {
                Some(Token::Identifier(id)) => id,
                _ => return Err("Expected parameter name".to_string()),
            };
            self.expect(Token::Colon)?;
            let param_type = self.parse_type()?;
            params.push((param_name, param_type));
            if let Some(Token::Comma) = self.peek() {
                self.next();
            }
        }
        self.expect(Token::RParen)?;
        self.expect(Token::Colon)?;
        self.expect(Token::Indent)?;
        let body = self.parse_block()?;
        self.expect(Token::Dedent)?;
        Ok(Function { name, params, body })
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        match self.next() {
            Some(Token::IntType) => Ok(Type::Int),
            Some(Token::FloatType) => Ok(Type::Float),
            Some(Token::BoolType) => Ok(Type::Bool),
            Some(Token::StringType) => Ok(Type::String),
            Some(Token::CharType) => Ok(Type::Char),
            _ => Err("Expected type".to_string()),
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while let Some(tok) = self.peek() {
            match tok {
                Token::Dedent | Token::Eof => break,
                _ => {
                    let stmt = self.parse_stmt()?;
                    stmts.push(stmt);
                }
            }
            self.skip_newlines();
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        self.skip_newlines();
        match self.peek() {
            Some(Token::Var) => self.parse_var_decl(),
            Some(Token::If) => self.parse_if(),
            Some(Token::While) => self.parse_while(),
            Some(Token::Return) => self.parse_return(),
            Some(Token::Print) => self.parse_print(),
            Some(Token::Input) => self.parse_input(),
            Some(Token::SetOutput) => self.parse_set_output(),
            Some(Token::Identifier(_)) => self.parse_assign_or_call(),
            _ => Err(format!("Unexpected token in statement: {:?}", self.peek())),
        }
    }

    fn parse_var_decl(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Var)?;
        let name = match self.next() {
            Some(Token::Identifier(id)) => id,
            _ => return Err("Expected variable name".to_string()),
        };
        self.expect(Token::Colon)?;
        let typ = self.parse_type()?;
        // Проверка на массив
        let typ = if let Some(Token::LBracket) = self.peek() {
            self.next();
            let size = match self.next() {
                Some(Token::IntLit(n)) => n as usize,
                _ => return Err("Expected array size".to_string()),
            };
            self.expect(Token::RBracket)?;
            Type::Array(Box::new(typ), size)
        } else {
            typ
        };
        let init = if let Some(Token::Assign) = self.peek() {
            self.next();
            let expr = self.parse_expr(0)?;
            Some(expr)
        } else {
            None
        };
        self.expect(Token::Semicolon)?;
        Ok(Stmt::VarDecl { name, typ, init })
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.expect(Token::If)?;
        let cond = self.parse_expr(0)?;
        self.expect(Token::Colon)?;
        self.expect(Token::Indent)?;
        let then_block = self.parse_block()?;
        self.expect(Token::Dedent)?;
        let else_block = if let Some(Token::Else) = self.peek() {
            self.next();
            self.expect(Token::Colon)?;
            self.expect(Token::Indent)?;
            let block = self.parse_block()?;
            self.expect(Token::Dedent)?;
            block
        } else {
            Vec::new()
        };
        Ok(Stmt::If { cond, then_block, else_block })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.expect(Token::While)?;
        let cond = self.parse_expr(0)?;
        self.expect(Token::Colon)?;
        self.expect(Token::Indent)?;
        let body = self.parse_block()?;
        self.expect(Token::Dedent)?;
        Ok(Stmt::While { cond, body })
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Return)?;
        let expr = self.parse_expr(0)?;
        self.expect(Token::Semicolon)?;
        Ok(Stmt::Return(expr))
    }

    fn parse_print(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Print)?;
        self.expect(Token::LParen)?;
        let mut args = Vec::new();
        while let Some(tok) = self.peek() {
            if *tok == Token::RParen { break; }
            let expr = self.parse_expr(0)?;
            args.push(expr);
            if let Some(Token::Comma) = self.peek() {
                self.next();
            }
        }
        self.expect(Token::RParen)?;
        self.expect(Token::Semicolon)?;
        Ok(Stmt::Print(args))
    }

    fn parse_input(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Input)?;
        self.expect(Token::LParen)?;
        let var = match self.next() {
            Some(Token::Identifier(id)) => id,
            _ => return Err("Expected variable name in input".to_string()),
        };
        self.expect(Token::RParen)?;
        self.expect(Token::Semicolon)?;
        Ok(Stmt::Input(var))
    }

    fn parse_set_output(&mut self) -> Result<Stmt, String> {
        self.expect(Token::SetOutput)?;
        self.expect(Token::LParen)?;
        let expr = self.parse_expr(0)?;
        self.expect(Token::RParen)?;
        self.expect(Token::Semicolon)?;
        Ok(Stmt::SetOutput(expr))
    }

    fn parse_assign_or_call(&mut self) -> Result<Stmt, String> {
        // Сохраняем позицию на случай backtracking
        let start = self.pos;
        let name = match self.next() {
            Some(Token::Identifier(id)) => id,
            _ => return Err("Expected identifier".to_string()),
        };
        // Проверяем, что дальше: '[' (массив), '=' (присваивание), '(' (вызов) или что-то ещё
        match self.peek() {
            Some(Token::LBracket) => {
                // элемент массива: name[expr]
                self.next();
                let index = self.parse_expr(0)?;
                self.expect(Token::RBracket)?;
                // теперь должно быть '=' или что-то ещё
                if let Some(Token::Assign) = self.peek() {
                    self.next();
                    let expr = self.parse_expr(0)?;
                    self.expect(Token::Semicolon)?;
                    Ok(Stmt::Assign { target: name, index: Some(Box::new(index)), expr })
                } else {
                    // это выражение (например, в составе другого выражения) – не должно встречаться как отдельный stmt
                    self.pos = start;
                    let expr = self.parse_expr(0)?;
                    self.expect(Token::Semicolon)?;
                    Ok(Stmt::ExprStmt(expr))
                }
            }
            Some(Token::Assign) => {
                // простое присваивание
                self.next();
                let expr = self.parse_expr(0)?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::Assign { target: name, index: None, expr })
            }
            Some(Token::LParen) => {
                // вызов функции
                self.pos = start; // откатываемся, чтобы parse_call_expr прочитал имя
                let expr = self.parse_expr(0)?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::ExprStmt(expr))
            }
            _ => {
                // просто выражение (например, инкремент? нет инкремента)
                self.pos = start;
                let expr = self.parse_expr(0)?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::ExprStmt(expr))
            }
        }
    }

    // Парсер выражений с приоритетами (рекурсивный спуск)
    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        let mut lhs = self.parse_primary()?;
        loop {
            let (bp, op) = match self.peek() {
                Some(Token::Plus) => (10, BinOp::Add),
                Some(Token::Minus) => (10, BinOp::Sub),
                Some(Token::Star) => (20, BinOp::Mul),
                Some(Token::Slash) => (20, BinOp::Div),
                Some(Token::Eq) => (5, BinOp::Eq),
                Some(Token::Ne) => (5, BinOp::Ne),
                Some(Token::Lt) => (5, BinOp::Lt),
                Some(Token::Le) => (5, BinOp::Le),
                Some(Token::Gt) => (5, BinOp::Gt),
                Some(Token::Ge) => (5, BinOp::Ge),
                Some(Token::And) => (3, BinOp::And),
                Some(Token::Or) => (2, BinOp::Or),
                _ => break,
            };
            if bp < min_bp { break; }
            self.next();
            let rhs = self.parse_expr(bp)?;
            lhs = Expr::BinaryOp(Box::new(lhs), op, Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Token::IntLit(n)) => Ok(Expr::Integer(n)),
            Some(Token::FloatLit(f)) => Ok(Expr::Float(f)),
            Some(Token::True) => Ok(Expr::Bool(true)),
            Some(Token::False) => Ok(Expr::Bool(false)),
            Some(Token::StringLit(s)) => Ok(Expr::String(s)),
            Some(Token::CharLit(c)) => Ok(Expr::Char(c)),
            Some(Token::Identifier(id)) => {
                // может быть переменная, элемент массива или вызов функции
                match self.peek() {
                    Some(Token::LBracket) => {
                        // массив: name[expr]
                        self.next();
                        let index = self.parse_expr(0)?;
                        self.expect(Token::RBracket)?;
                        Ok(Expr::ArrayElement(id, Box::new(index)))
                    }
                    Some(Token::LParen) => {
                        // вызов функции
                        self.next();
                        let mut args = Vec::new();
                        while let Some(tok) = self.peek() {
                            if *tok == Token::RParen { break; }
                            let arg = self.parse_expr(0)?;
                            args.push(arg);
                            if let Some(Token::Comma) = self.peek() {
                                self.next();
                            }
                        }
                        self.expect(Token::RParen)?;
                        Ok(Expr::Call(id, args))
                    }
                    _ => Ok(Expr::Variable(id)),
                }
            }
            Some(Token::LParen) => {
                let expr = self.parse_expr(0)?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Some(Token::Not) => {
                let sub = self.parse_expr(10)?;
                Ok(Expr::UnaryOp(UnOp::Not, Box::new(sub)))
            }
            Some(Token::Minus) => {
                let sub = self.parse_expr(10)?;
                Ok(Expr::UnaryOp(UnOp::Neg, Box::new(sub)))
            }
            _ => Err(format!("Unexpected token in expression: {:?}", self.peek())),
        }
    }
}