use crate::chunk::{
    Chunk, Op, Op::*, Line,
};
use crate::value::Value;
use crate::object::Heap;

pub fn _dissassemble_chunk(chunk: &Chunk, heap: &mut Heap, name: &str) {
    println!("== {} ==", name);

    let mut offset = 0;
    while offset < chunk.code.len() {
        offset = dissassemble_instruction(chunk, heap, offset);
        println!();
    }
}

pub fn dissassemble_instruction(chunk: &Chunk, heap: &Heap, offset: usize) -> usize {
    print!("{:04} ", offset);
    let (instruction, line) = chunk.code.get(offset).unwrap();
    match instruction {
        Constant(const_idx) => constant_instruction(chunk, heap, offset, line, "OP_CONSTANT", const_idx),
        Op::Nil => simple_instruction(chunk, offset, line, "OP_NIL"),
        Op::True => simple_instruction(chunk, offset, line, "OP_TRUE"),
        Op::False => simple_instruction(chunk, offset, line, "OP_FALSE"),
        Pop => simple_instruction(chunk, offset, line, "OP_POP"),
        GetLocal(_) => byte_instruction(chunk, offset, line, "OP_GET_LOCAL"),
        SetLocal(_) => byte_instruction(chunk, offset, line, "OP_SET_LOCAL"),
        GetGlobal(const_idx) => constant_instruction(chunk, heap, offset, line, "OP_GET_GLOBAL", const_idx),
        DefineGlobal(const_idx) => constant_instruction(chunk, heap, offset, line, "OP_DEFINE_GLOBAL", const_idx),
        SetGlobal(const_idx) => constant_instruction(chunk, heap, offset, line, "OP_SET_GLOBAL", const_idx),
        Equal => simple_instruction(chunk, offset, line, "OP_EQUAL"),
        Greater => simple_instruction(chunk, offset, line, "OP_GREATER"),
        Less => simple_instruction(chunk, offset, line, "OP_LESS"),
        Add => simple_instruction(chunk, offset, line, "OP_ADD"),
        Subtract => simple_instruction(chunk, offset, line, "OP_SUBTRACT"),
        Multiply => simple_instruction(chunk, offset, line, "OP_MULTIPLY"),
        Divide => simple_instruction(chunk, offset, line, "OP_DIVIDE"),
        Not => simple_instruction(chunk, offset, line, "OP_NOT"),
        Negate => simple_instruction(chunk, offset, line, "OP_NEGATE"),
        Print => {
            let size = simple_instruction(chunk, offset, line, "OP_PRINT");
            size
        },
        JumpIfFalse(_) => jump_instruction(chunk, offset, line, "OP_JUMP_IF_FALSE", 1),
        Jump(_) => jump_instruction(chunk, offset, line, "OP_JUMP", 1),
        Loop(_) => jump_instruction(chunk, offset, line, "OP_LOOP", -1),
        Return => simple_instruction(chunk, offset, line, "OP_RETURN"),
    }
}

fn simple_instruction(chunk: &Chunk, offset: usize, line: &Line, name: &str) -> usize {
    let mut line_no = format!("{:04}", line.value);
    let previous_offset = if offset == 0 { 0 } else { offset - 1 };
    if previous_offset != offset && chunk.code.get(offset - 1).unwrap().1.value == line.value {
        line_no = "   |".to_string();
    }
    print!("\r {:4} {:<16}", line_no, name);
    for _ in 0..16 {
        print!(" ");
    }
    offset + 1
}

fn constant_instruction(chunk: &Chunk, heap: &Heap, offset: usize, line: &Line, name: &str, const_idx: &usize) -> usize {
    let mut line_no = format!("{:04}", line.value);
    let previous_offset = if offset == 0 { 0 } else { offset - 1 };
    if previous_offset != offset && chunk.code.get(offset - 1).unwrap().1.value == line.value {
        line_no = "   |".to_string();
    }
    print!("\r {:4} {:<16} {:4} '", line_no, name, const_idx);
    let val_len = print_value(chunk.constants.get(*const_idx).unwrap(), heap);
    print!("'");
    if val_len < 8 {
        for _ in 0..(8 - val_len) {
            print!(" ");
        }
    }
    offset + 1
}

fn byte_instruction(chunk: &Chunk, offset: usize, line: &Line, name: &str) -> usize {
    let mut line_no = format!("{:04}", line.value);
    let previous_offset = if offset == 0 { 0 } else { offset - 1 };
    if previous_offset != offset && chunk.code.get(offset - 1).unwrap().1.value == line.value {
        line_no = "   |".to_string();
    }
    let byte = chunk.code.get(offset).unwrap();
    print!("\r {:4} {:<20} {:4}", line_no, name, byte.0);
    offset + 2
}

fn jump_instruction(chunk: &Chunk, offset: usize, line: &Line, name: &str, sign: i32) -> usize {
    let jump = chunk.code.get(offset).unwrap();
    let mut line_no = format!("{:04}", line.value);
    let previous_offset = if offset == 0 { 0 } else { offset - 1 };
    if previous_offset != offset && chunk.code.get(offset - 1).unwrap().1.value == line.value {
        line_no = "   |".to_string();
    }
    print!("\r {:4} {:<16} {} -> {}", line_no, name, jump.0, offset as i32 + 2 + sign);
    offset + 2
}

pub fn print_value(value: &Value, heap: &Heap) -> usize {
    match value {
        Value::Number(num) => {
            let val = format!("{}", num);
            print!("\r{val}");
            return val.len();
        },
        Value::Nil => {
            print!("\rnil");
            return 3;
        },
        Value::Bool(b) => {
            let val = format!("{}", b);
            print!("\r{val}");
            return val.len();
        },
        Value::Obj(val) => {
            let obj_string = heap.get(val).unwrap();
            print!("\r{}", obj_string.as_string());
            return obj_string.as_string().len();
        },
    }
}