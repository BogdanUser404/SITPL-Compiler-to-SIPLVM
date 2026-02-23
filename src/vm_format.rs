//! Константы и функции для создания бинарного файла VM.

pub const PROG_MAGIC: i16 = -32766;
pub const DATA_MAGIC: i16 = -32767;
pub const CODE_MAGIC: i16 = 32767;

pub const OPCODES: &[(&str, u16)] = &[
    ("ADD_I", 0x0001), ("SUB_I", 0x0002), ("MUL_I", 0x0003), ("DIV_I", 0x0004),
    ("ADD_F", 0x0005), ("SUB_F", 0x0006), ("MUL_F", 0x0007), ("DIV_F", 0x0008),
    ("IF_I", 0x0010), ("IF_F", 0x0011), ("IF_CHAR", 0x0012), ("IF_STR", 0x0013),
    ("STR2INT", 0x0034), ("INT2STR", 0x0035), ("STR2F", 0x0036), ("F2STR", 0x0037),
    ("STR2BOOL", 0x0038), ("BOOL2STR", 0x0039), ("STR2CHAR", 0x003A), ("CHAR2STR", 0x003B),
    ("SET_OUT_FILE", 0x0051), ("PRINT_STR", 0x0052), ("READ_STR", 0x0070),
    ("JMP", 0x0060), ("JMP_TRUE", 0x0061), ("JMP_FALSE", 0x0062), ("HALT", 0x0063),
    ("CALL", 0x0070), ("RET", 0x0071), ("STR_ADD", 0x0032),
];

pub fn opcode_from_mnemonic(mnem: &str) -> Option<u16> {
    OPCODES.iter().find(|(name, _)| *name == mnem).map(|(_, code)| *code)
}