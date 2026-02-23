//! Определения AST для языка SITPL.

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Char,
    Array(Box<Type>, usize), // элемент и размер
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
}

#[derive(Debug, Clone)]
pub enum UnOp {
    Neg, Not,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Integer(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Char(char),
    Variable(String),
    ArrayElement(String, Box<Expr>), // имя массива, индекс
    BinaryOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryOp(UnOp, Box<Expr>),
    Call(String, Vec<Expr>), // вызов функции
}

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl {
        name: String,
        typ: Type,
        init: Option<Expr>,
    },
    Assign {
        target: String,
        index: Option<Box<Expr>>, // если Some, то это присваивание элементу массива
        expr: Expr,
    },
    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Return(Expr),
    ExprStmt(Expr), // выражение как инструкция (например, вызов функции)
    Print(Vec<Expr>), // print с несколькими аргументами
    Input(String),   // input(переменная)
    SetOutput(Expr), // set_output(имя_файла)
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
    pub globals: Vec<Stmt>, // глобальные инструкции (будут помещены в начало main)
}