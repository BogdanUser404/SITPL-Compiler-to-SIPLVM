mod lexer;
mod parser;
mod ast;
mod semantic;
mod codegen;
mod vm_format;
mod assembler;

use std::env;
use std::fs;
use std::process;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: sitplc <source.sitpl>");
        process::exit(1);
    }

    let source = fs::read_to_string(&args[1])?;
    let lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse_program()?;

    let mut sym_table = semantic::SymbolTable::new();
    sym_table.check_program(&program)?;

    let codegen = codegen::CodeGen::new(sym_table);
    let asm_code = codegen.generate(&program);
    let binary = assembler::assemble(&asm_code)?;
    fs::write("output.bin", binary)?;

    Ok(())
}