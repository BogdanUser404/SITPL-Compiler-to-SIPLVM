//! Семантический анализ: проверка типов, разрешение имён, выделение регистров.

use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Symbol {
    pub reg: usize,       // номер регистра
    pub typ: Type,
    pub is_array: bool,
    pub size: Option<usize>, // для массивов
}

pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
    next_reg: usize,
    pub current_function: Option<String>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut global = HashMap::new();
        // можно добавить предопределённые символы (например, константы)
        SymbolTable {
            scopes: vec![global],
            next_reg: 0,
            current_function: None,
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn declare(&mut self, name: &str, typ: Type, is_array: bool, size: Option<usize>) -> Result<usize, String> {
        let scope = self.scopes.last_mut().unwrap();
        if scope.contains_key(name) {
            return Err(format!("Variable {} already declared", name));
        }
        let reg = self.next_reg;
        self.next_reg += 1;
        // Если массив, занимаем несколько регистров
        if let Some(sz) = size {
            self.next_reg += sz - 1;
        }
        scope.insert(name.to_string(), Symbol { reg, typ, is_array, size });
        Ok(reg)
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.get(name) {
                return Some(sym);
            }
        }
        None
    }

    // Вспомогательные функции для кодогенератора
    pub fn get_reg(&self, name: &str) -> Option<usize> {
        self.lookup(name).map(|s| s.reg)
    }

    pub fn get_type(&self, name: &str) -> Option<&Type> {
        self.lookup(name).map(|s| &s.typ)
    }

    // Проверка программы
    pub fn check_program(&mut self, prog: &Program) -> Result<(), String> {
        // Сначала обработаем глобальные переменные и функции
        for func in &prog.functions {
            // функция не занимает регистр, но её параметры будут локальными
            // добавим имя функции в глобальную область как метку? Не нужно для семантики
        }
        for stmt in &prog.globals {
            self.check_stmt(stmt)?;
        }
        for func in &prog.functions {
            self.check_function(func)?;
        }
        Ok(())
    }

    fn check_function(&mut self, func: &Function) -> Result<(), String> {
        self.current_function = Some(func.name.clone());
        self.enter_scope();
        for (name, typ) in &func.params {
            self.declare(name, typ.clone(), false, None)?;
        }
        for stmt in &func.body {
            self.check_stmt(stmt)?;
        }
        self.exit_scope();
        self.current_function = None;
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::VarDecl { name, typ, init } => {
                let (is_array, size) = match typ {
                    Type::Array(_, sz) => (true, Some(*sz)),
                    _ => (false, None),
                };
                self.declare(name, typ.clone(), is_array, size)?;
                if let Some(init_expr) = init {
                    let expr_type = self.check_expr(init_expr)?;
                    // проверить соответствие типов
                    // упрощённо: типы должны совпадать
                }
                Ok(())
            }
            Stmt::Assign { target, index, expr } => {
                let sym = self.lookup(target).ok_or_else(|| format!("Variable {} not declared", target))?;
                if index.is_some() && !sym.is_array {
                    return Err(format!("{} is not an array", target));
                }
                let expr_type = self.check_expr(expr)?;
                // проверка типа
                Ok(())
            }
            Stmt::If { cond, then_block, else_block } => {
                let cond_type = self.check_expr(cond)?;
                // cond_type должно быть Bool
                self.enter_scope();
                for s in then_block { self.check_stmt(s)?; }
                self.exit_scope();
                self.enter_scope();
                for s in else_block { self.check_stmt(s)?; }
                self.exit_scope();
                Ok(())
            }
            Stmt::While { cond, body } => {
                let cond_type = self.check_expr(cond)?;
                self.enter_scope();
                for s in body { self.check_stmt(s)?; }
                self.exit_scope();
                Ok(())
            }
            Stmt::Return(expr) => {
                if self.current_function.is_none() {
                    return Err("return outside function".to_string());
                }
                let _ = self.check_expr(expr)?;
                Ok(())
            }
            Stmt::ExprStmt(expr) => {
                let _ = self.check_expr(expr)?;
                Ok(())
            }
            Stmt::Print(args) => {
                for arg in args {
                    let _ = self.check_expr(arg)?;
                }
                Ok(())
            }
            Stmt::Input(var) => {
                let sym = self.lookup(var).ok_or_else(|| format!("Variable {} not declared", var))?;
                // тип должен быть string
                Ok(())
            }
            Stmt::SetOutput(expr) => {
                let typ = self.check_expr(expr)?;
                // тип должен быть string
                Ok(())
            }
        }
    }

    fn check_expr(&self, expr: &Expr) -> Result<Type, String> {
        match expr {
            Expr::Integer(_) => Ok(Type::Int),
            Expr::Float(_) => Ok(Type::Float),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::String(_) => Ok(Type::String),
            Expr::Char(_) => Ok(Type::Char),
            Expr::Variable(name) => {
                let sym = self.lookup(name).ok_or_else(|| format!("Variable {} not declared", name))?;
                Ok(sym.typ.clone())
            }
            Expr::ArrayElement(name, idx) => {
                let sym = self.lookup(name).ok_or_else(|| format!("Array {} not declared", name))?;
                if !sym.is_array {
                    return Err(format!("{} is not an array", name));
                }
                let idx_type = self.check_expr(idx)?;
                // idx_type должен быть Int
                if let Type::Array(elem_type, _) = &sym.typ {
                    Ok(*elem_type.clone())
                } else {
                    unreachable!()
                }
            }
            Expr::BinaryOp(lhs, op, rhs) => {
                let lt = self.check_expr(lhs)?;
                let rt = self.check_expr(rhs)?;
                // упрощённо: для арифметики оба int или оба float
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        if lt == rt {
                            Ok(lt)
                        } else {
                            Err("Type mismatch in arithmetic".to_string())
                        }
                    }
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        if lt == rt {
                            Ok(Type::Bool)
                        } else {
                            Err("Type mismatch in comparison".to_string())
                        }
                    }
                    BinOp::And | BinOp::Or => {
                        if lt == Type::Bool && rt == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            Err("Logical operators require bool".to_string())
                        }
                    }
                }
            }
            Expr::UnaryOp(op, sub) => {
                let st = self.check_expr(sub)?;
                match op {
                    UnOp::Neg => {
                        if st == Type::Int || st == Type::Float {
                            Ok(st)
                        } else {
                            Err("Negation requires numeric type".to_string())
                        }
                    }
                    UnOp::Not => {
                        if st == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            Err("Not requires bool".to_string())
                        }
                    }
                }
            }
            Expr::Call(name, args) => {
                // Проверка вызова функции – пока пропустим
                Ok(Type::Int) // заглушка
            }
        }
    }
}