use crate::chunk::{
    Chunk, Op::*, Line,
};
use crate::value::{
    Value,
    Value::*,
};

fn dissassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {} ==", name);

    let mut offset = 0;
    while offset < chunk.code.len() {
        offset = dissassemble_instruction(chunk, offset);
        println!();
    }
}

pub fn dissassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    print!("{:04} ", offset);

    let (instruction, line) = chunk.code.get(offset).unwrap();
    match instruction {
        Constant(const_idx) => constant_instruction(chunk, offset, line, "OP_CONSTANT", const_idx),
        Add => simple_instruction(chunk, offset, line, "OP_ADD"),
        Subtract => simple_instruction(chunk, offset, line, "OP_SUBTRACT"),
        Multiply => simple_instruction(chunk, offset, line, "OP_MULTIPLY"),
        Divide => simple_instruction(chunk, offset, line, "OP_DIVIDE"),
        Negate => simple_instruction(chunk, offset, line, "OP_NEGATE"),
        Return => simple_instruction(chunk, offset, line, "OP_RETURN"),
    }
}

fn simple_instruction(chunk: &Chunk, offset: usize, line: &Line, name: &str) -> usize {
    let mut line_no = format!("{:04}", line.value);
    let previous_offset = if offset == 0 { 0 } else { offset - 1 };
    if previous_offset != offset && chunk.code.get(offset - 1).unwrap().1.value == line.value {
        line_no = "   |".to_string();
    }
    print!(" {:4} {:<16}", line_no, name);
    for _ in 0..16 {
        print!(" ");
    }
    offset + 1
}

fn constant_instruction(chunk: &Chunk, offset: usize, line: &Line, name: &str, const_idx: &usize) -> usize {
    let mut line_no = format!("{:04}", line.value);
    let previous_offset = if offset == 0 { 0 } else { offset - 1 };
    if previous_offset != offset && chunk.code.get(offset - 1).unwrap().1.value == line.value {
        line_no = "   |".to_string();
    }
    print!(" {:4} {:<16} {:4} '", line_no, name, const_idx);
    let val_len = print_value(chunk.constants.get(*const_idx).unwrap());
    print!("'");
    if val_len < 14 {
        for _ in 0..(8 - val_len) {
            print!(" ");
        }
    }
    offset + 1
}

pub fn print_value(value: &Value) -> usize {
    match value {
        Number(num) => {
            let val = format!("{}", num);
            print!("{val}");
            return val.len();
        },
        Nil => {
            print!("nil");
            return 3;
        },
        Bool(b) => {
            let val = format!("{}", b);
            print!("{val}");
            return val.len();
        },
    }
}