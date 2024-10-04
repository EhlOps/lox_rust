use crate::value::Value;

#[derive(Debug, Clone)]
pub enum Op {
    Constant(usize),
    Nil,
    True,
    False,
    Pop,
    GetLocal(usize),
    SetLocal(usize),
    GetGlobal(usize),
    DefineGlobal(usize),
    SetGlobal(usize),
    Equal,
    Greater,
    Less,
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Negate,
    Print,
    JumpIfFalse(usize),
    Jump(usize),
    Loop(usize),
    Return,
}

impl std::fmt::Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    pub value: usize,
}


pub fn line(value: usize) -> Line {
    Line { value }
}


#[derive(Debug, Default, Clone)]
pub struct Chunk {
    pub code: Vec<(Op, Line)>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk {
            code: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn add_constant(&mut self, val: Value) -> usize {
        self.constants.push(val);
        self.constants.len() - 1
    } 
}