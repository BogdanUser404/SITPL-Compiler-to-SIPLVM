//! Ассемблер, преобразующий текст на ассемблере VM в бинарный файл.

use crate::vm_format::{self, opcode_from_mnemonic};
use std::collections::HashMap;

/// Структура для записи в секции DATA.
struct DataEntry {
    reg: u16,
    typ: u8,
    value: [u8; 29],
}

/// Структура для хранения инструкции на этапе парсинга.
#[derive(Debug, Clone)]
struct ParsedIns {
    mnemonic: String,
    dst: String,
    src1: String,
    src2: String,
}

/// Ошибки ассемблирования.
#[derive(Debug)]
pub enum AssemblerError {
    ParseError(String),
    InvalidOpcode(String),
    InvalidDataType(String),
    LabelNotFound(String),
    InvalidImmediate(String),
    StringTooLong(String),
    InvalidChar(String),
    DuplicateLabel(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AssemblerError::ParseError(s) => write!(f, "Parse error: {}", s),
            AssemblerError::InvalidOpcode(s) => write!(f, "Invalid opcode: {}", s),
            AssemblerError::InvalidDataType(s) => write!(f, "Invalid data type: {}", s),
            AssemblerError::LabelNotFound(s) => write!(f, "Label not found: {}", s),
            AssemblerError::InvalidImmediate(s) => write!(f, "Invalid immediate value: {}", s),
            AssemblerError::StringTooLong(s) => write!(f, "String too long (max 28 bytes): {}", s),
            AssemblerError::InvalidChar(s) => write!(f, "Invalid character: {}", s),
            AssemblerError::DuplicateLabel(s) => write!(f, "Duplicate label: {}", s),
            AssemblerError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for AssemblerError {}

/// Результат ассемблирования.
type Result<T> = std::result::Result<T, AssemblerError>;

/// Основная функция ассемблера.
pub fn assemble(source: &str) -> Result<Vec<u8>> {
    let lines: Vec<&str> = source.lines().collect();
    let mut data_entries = Vec::new(); // (reg, typ, packed_value)
    let mut parsed_ins = Vec::new();   // (mnemonic, dst, src1, src2)
    let mut labels = HashMap::new();   // имя метки -> номер инструкции

    // Первый проход: собираем метки и записи DATA, а также список инструкций (без разрешения меток).
    for (line_num, raw_line) in lines.iter().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        // Метка?
        if line.ends_with(':') {
            let label = line[..line.len()-1].trim().to_string();
            if labels.contains_key(&label) {
                return Err(AssemblerError::DuplicateLabel(label));
            }
            labels.insert(label, parsed_ins.len()); // метка указывает на следующую инструкцию
            continue;
        }

        // Директива .data
        if line.starts_with(".data") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                return Err(AssemblerError::ParseError(format!(".data requires at least 3 arguments at line {}", line_num+1)));
            }
            let typ = parts[1];
            let reg = parts[2].parse::<u16>()
                .map_err(|_| AssemblerError::InvalidImmediate(format!("Invalid register number: {}", parts[2])))?;
            let value_str = parts[3..].join(" ");
            let (typ_code, packed) = pack_data(typ, &value_str)?;
            data_entries.push((reg, typ_code, packed));
            continue;
        }

        // Обычная инструкция
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let mnemonic = parts[0].to_uppercase();
        // Ожидаем три операнда (dst, src1, src2) – все могут быть числами или метками
        // Если операндов меньше, дополним нулями (строками "0")
        let mut dst = "0".to_string();
        let mut src1 = "0".to_string();
        let mut src2 = "0".to_string();
        if parts.len() >= 2 { dst = parts[1].to_string(); }
        if parts.len() >= 3 { src1 = parts[2].to_string(); }
        if parts.len() >= 4 { src2 = parts[3].to_string(); }

        parsed_ins.push(ParsedIns { mnemonic, dst, src1, src2 });
    }

    // Второй проход: генерация кода с разрешением меток.
    let mut code_bytes = Vec::new();
    for (i, ins) in parsed_ins.iter().enumerate() {
        let opcode = opcode_from_mnemonic(&ins.mnemonic)
            .ok_or_else(|| AssemblerError::InvalidOpcode(ins.mnemonic.clone()))?;

        // Разрешаем операнды: либо число, либо метка (только для src1/src2 в переходах)
        let resolve_operand = |operand: &str, current_ip: usize| -> Result<u16> {
            if operand.is_empty() { return Ok(0); }
            // Попробуем распарсить как число
            if let Ok(num) = operand.parse::<i64>() {
                // Для переходов смещение должно быть в пределах i16
                if ins.mnemonic.starts_with("JMP") {
                    if num < i16::MIN as i64 || num > i16::MAX as i64 {
                        return Err(AssemblerError::InvalidImmediate(format!("Jump offset out of range: {}", num)));
                    }
                    // Возвращаем как беззнаковое (дополнительный код)
                    return Ok((num as i16) as u16);
                } else {
                    // Для обычных регистров число должно быть в пределах u16
                    if num < 0 || num > u16::MAX as i64 {
                        return Err(AssemblerError::InvalidImmediate(format!("Register number out of range: {}", num)));
                    }
                    return Ok(num as u16);
                }
            }
            // Если не число, значит метка
            if let Some(&target_ip) = labels.get(operand) {
                // Смещение = target_ip - current_ip
                let offset = target_ip as i64 - current_ip as i64;
                if offset < i16::MIN as i64 || offset > i16::MAX as i64 {
                    return Err(AssemblerError::InvalidImmediate(format!("Jump offset too large for label {}: {}", operand, offset)));
                }
                return Ok((offset as i16) as u16);
            }
            Err(AssemblerError::LabelNotFound(operand.to_string()))
        };

        let dst_val = resolve_operand(&ins.dst, i)?;
        let src1_val = resolve_operand(&ins.src1, i)?;
        let src2_val = resolve_operand(&ins.src2, i)?;

        // Упаковка в little-endian
        code_bytes.extend_from_slice(&opcode.to_le_bytes());
        code_bytes.extend_from_slice(&dst_val.to_le_bytes());
        code_bytes.extend_from_slice(&src1_val.to_le_bytes());
        code_bytes.extend_from_slice(&src2_val.to_le_bytes());
    }

    // Формирование бинарного файла
    let mut bin = Vec::new();

    // Заголовок программы (8 байт)
    bin.extend_from_slice(&(vm_format::PROG_MAGIC as i16).to_le_bytes());
    bin.extend_from_slice(&(data_entries.len() as u16).to_le_bytes());
    bin.extend_from_slice(&(parsed_ins.len() as u32).to_le_bytes());

    // Заголовок DATA (2 байта)
    bin.extend_from_slice(&(vm_format::DATA_MAGIC as i16).to_le_bytes());

    // Секция DATA
    for (reg, typ, packed) in data_entries {
        bin.extend_from_slice(&reg.to_le_bytes());
        bin.push(typ);
        bin.extend_from_slice(&packed);
    }

    // Заголовок CODE (2 байта)
    bin.extend_from_slice(&(vm_format::CODE_MAGIC as i16).to_le_bytes());

    // Секция CODE
    bin.extend_from_slice(&code_bytes);

    Ok(bin)
}

/// Упаковывает значение в 29 байт согласно типу.
fn pack_data(typ: &str, value_str: &str) -> Result<(u8, [u8; 29])> {
    let mut data = [0u8; 29];
    let typ_code = match typ.to_lowercase().as_str() {
        "int" => {
            let val = value_str.parse::<i64>()
                .map_err(|_| AssemblerError::InvalidImmediate(format!("Invalid int: {}", value_str)))?;
            data[0..8].copy_from_slice(&val.to_le_bytes());
            1
        }
        "float" => {
            let val = value_str.parse::<f64>()
                .map_err(|_| AssemblerError::InvalidImmediate(format!("Invalid float: {}", value_str)))?;
            data[0..8].copy_from_slice(&val.to_le_bytes());
            2
        }
        "bool" => {
            let v = value_str.to_lowercase();
            let val = if v == "true" || v == "1" { 1 } else if v == "false" || v == "0" { 0 } else {
                return Err(AssemblerError::InvalidImmediate(format!("Invalid bool: {}", value_str)));
            };
            data[0] = val;
            3
        }
        "string" => {
            // Убираем кавычки, если они есть
            let s = if value_str.starts_with('"') && value_str.ends_with('"') {
                &value_str[1..value_str.len()-1]
            } else {
                value_str
            };
            let bytes = s.as_bytes();
            if bytes.len() > 28 {
                return Err(AssemblerError::StringTooLong(s.to_string()));
            }
            data[0..bytes.len()].copy_from_slice(bytes);
            4
        }
        "char" => {
            let s = if value_str.starts_with('\'') && value_str.ends_with('\'') {
                &value_str[1..value_str.len()-1]
            } else {
                value_str
            };
            if s.len() != 1 {
                return Err(AssemblerError::InvalidChar(format!("Char must be a single character: {}", s)));
            }
            let ch = s.chars().next().unwrap();
            let codepoint = ch as u32;
            data[0..4].copy_from_slice(&codepoint.to_le_bytes());
            5
        }
        _ => return Err(AssemblerError::InvalidDataType(typ.to_string())),
    };
    Ok((typ_code, data))
}