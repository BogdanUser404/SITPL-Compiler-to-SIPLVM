//! Генерация ассемблерного текста из AST с учётом таблицы символов.

use crate::ast::*;
use crate::semantic::SymbolTable;

pub struct CodeGen {
    output: Vec<String>,
    labels: usize,
    sym_table: SymbolTable,
}

impl CodeGen {
    pub fn new(sym_table: SymbolTable) -> Self {
        CodeGen {
            output: Vec::new(),
            labels: 0,
            sym_table,
        }
    }

    fn new_label(&mut self, prefix: &str) -> String {
        let lbl = format!("{}_{}", prefix, self.labels);
        self.labels += 1;
        lbl
    }

    fn emit(&mut self, line: String) {
        self.output.push(line);
    }

    pub fn generate(mut self, prog: &Program) -> String {
        // Секция .data
        self.emit(".data".to_string());
        // Глобальные переменные (и массивы) уже должны быть в таблице символов
        // Для простоты мы не генерируем .data для переменных, инициализированных нулём.
        // В VM регистры обнуляются при старте.
        // Но для строк и констант нужно добавить записи.
        // Здесь должен быть проход по AST и сбор строковых литералов.
        // Пропустим для краткости.

        self.emit("".to_string());

        // Точка входа
        self.emit("start:".to_string());

        // Генерация кода для глобальных инструкций
        for stmt in &prog.globals {
            self.gen_stmt(stmt);
        }

        // Генерация функций
        for func in &prog.functions {
            self.gen_function(func);
        }

        self.emit("    HALT 0 0 0".to_string());
        self.output.join("\n")
    }

    fn gen_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl { name, typ, init } => {
                let reg = self.sym_table.get_reg(name).unwrap();
                if let Some(expr) = init {
                    self.gen_expr_into(expr, reg);
                }
            }
            Stmt::Assign { target, index, expr } => {
                let base_reg = self.sym_table.get_reg(target).unwrap();
                if let Some(idx_expr) = index {
                    // индекс должен быть константой (упрощение)
                    // вычислим индекс и получим номер регистра
                    let idx = self.eval_const_int(idx_expr).unwrap();
                    let target_reg = base_reg + idx;
                    self.gen_expr_into(expr, target_reg);
                } else {
                    self.gen_expr_into(expr, base_reg);
                }
            }
            Stmt::If { cond, then_block, else_block } => {
                let cond_reg = self.gen_expr(cond);
                let else_label = self.new_label("else");
                let end_label = self.new_label("endif");
                self.emit(format!("    JMP_FALSE {} {}", cond_reg, else_label));
                for stmt in then_block {
                    self.gen_stmt(stmt);
                }
                self.emit(format!("    JMP {}", end_label));
                self.emit(format!("{}:", else_label));
                for stmt in else_block {
                    self.gen_stmt(stmt);
                }
                self.emit(format!("{}:", end_label));
            }
            Stmt::While { cond, body } => {
                let start_label = self.new_label("while_start");
                let end_label = self.new_label("while_end");
                self.emit(format!("{}:", start_label));
                let cond_reg = self.gen_expr(cond);
                self.emit(format!("    JMP_FALSE {} {}", cond_reg, end_label));
                for stmt in body {
                    self.gen_stmt(stmt);
                }
                self.emit(format!("    JMP {}", start_label));
                self.emit(format!("{}:", end_label));
            }
            Stmt::Return(expr) => {
                self.gen_expr_into(expr, 0);
                self.emit("    RET".to_string());
            }
            Stmt::ExprStmt(expr) => {
                self.gen_expr(expr);
            }
            Stmt::Print(args) => {
                for arg in args {
                    self.gen_print_arg(arg);
                }
            }
            Stmt::Input(var) => {
                let reg = self.sym_table.get_reg(var).unwrap();
                self.emit(format!("    READ_STR {} 0 0", reg));
            }
            Stmt::SetOutput(expr) => {
                let reg = self.gen_expr(expr);
                self.emit(format!("    SET_OUT_FILE {} 0 0", reg));
            }
        }
    }

    fn gen_expr(&mut self, expr: &Expr) -> usize {
        // вычисляет выражение, возвращает номер регистра с результатом
        // создаёт временные регистры (увеличивает счётчик)
        // упрощённо: используем регистры начиная с 100 для временных
        let tmp_reg = 100; // в реальности нужно управлять временными регистрами
        self.gen_expr_into(expr, tmp_reg);
        tmp_reg
    }

    fn gen_expr_into(&mut self, expr: &Expr, target: usize) {
        match expr {
            Expr::Integer(n) => {
                self.emit(format!("    LOAD_INT {} {}", target, n));
            }
            Expr::Float(f) => {
                self.emit(format!("    LOAD_FLOAT {} {}", target, f));
            }
            Expr::Bool(b) => {
                self.emit(format!("    LOAD_BOOL {} {}", target, if *b { 1 } else { 0 }));
            }
            Expr::String(s) => {
                // строку надо поместить в .data, но пока просто заглушка
                self.emit(format!("    ; STRING {} \"{}\"", target, s));
            }
            Expr::Char(c) => {
                self.emit(format!("    LOAD_CHAR {} {}", target, *c as u32));
            }
            Expr::Variable(name) => {
                let reg = self.sym_table.get_reg(name).unwrap();
                self.emit(format!("    ADD_I {} {} 0", target, reg));
            }
            Expr::ArrayElement(name, idx) => {
                let base = self.sym_table.get_reg(name).unwrap();
                let idx_reg = self.gen_expr(idx);
                // нет косвенной адресации, поэтому индекс должен быть константой
                // здесь мы предполагаем, что idx вычисляется в константу
                // для простоты используем ADD с нулём и надеемся на константность
                self.emit(format!("    ADD_I {} {} {}", target, base, idx_reg));
            }
            Expr::BinaryOp(lhs, op, rhs) => {
                let left_reg = self.gen_expr(lhs);
                let right_reg = self.gen_expr(rhs);
                let instr = match op {
                    BinOp::Add => "ADD_I",
                    BinOp::Sub => "SUB_I",
                    BinOp::Mul => "MUL_I",
                    BinOp::Div => "DIV_I",
                    BinOp::Eq => "IF_I",
                    BinOp::Ne => "IF_I",
                    BinOp::Lt => "CMP_LT",
                    BinOp::Le => "CMP_LE",
                    BinOp::Gt => "CMP_GT",
                    BinOp::Ge => "CMP_GE",
                    BinOp::And => "AND_B",
                    BinOp::Or => "OR_B",
                };
                self.emit(format!("    {} {} {} {}", instr, target, left_reg, right_reg));
                if *op == BinOp::Ne {
                    self.emit(format!("    NOT_B {} {}", target, target));
                }
            }
            Expr::UnaryOp(op, sub) => {
                let sub_reg = self.gen_expr(sub);
                match op {
                    UnOp::Neg => self.emit(format!("    SUB_I {} 0 {}", target, sub_reg)),
                    UnOp::Not => self.emit(format!("    NOT_B {} {}", target, sub_reg)),
                }
            }
            Expr::Call(name, args) => {
                // передача аргументов через регистры 0..N-1
                for (i, arg) in args.iter().enumerate() {
                    self.gen_expr_into(arg, i);
                }
                self.emit(format!("    CALL func_{}", name));
                if target != 0 {
                    // скопировать результат из r0 в target
                    self.emit(format!("    ADD_I {} 0 0", target));
                }
            }
        }
    }

    fn gen_print_arg(&mut self, arg: &Expr) {
        // временный регистр для преобразования
        let tmp = 100; // опять упрощение
        match arg {
            Expr::String(s) => {
                // нужно создать строку в .data
                // пропущено
                self.emit(format!("    ; PRINT_STR {}", tmp));
            }
            _ => {
                let reg = self.gen_expr(arg);
                // преобразование в строку в зависимости от типа
                // здесь нужно определить тип выражения
                // упрощённо: INT2STR для всего
                self.emit(format!("    INT2STR {} {}", tmp, reg));
                self.emit(format!("    PRINT_STR {} 0 0", tmp));
            }
        }
    }

    fn gen_function(&mut self, func: &Function) {
        self.emit(format!("func_{}:", func.name));
        // параметры уже лежат в регистрах 0..N-1
        for stmt in &func.body {
            self.gen_stmt(stmt);
        }
        // если нет return, добавим RET
        if !func.body.iter().any(|s| matches!(s, Stmt::Return(_))) {
            self.emit("    RET".to_string());
        }
    }

    fn eval_const_int(&self, expr: &Expr) -> Option<usize> {
        match expr {
            Expr::Integer(n) => Some(*n as usize),
            _ => None,
        }
    }
}