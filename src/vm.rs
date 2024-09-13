use crate::chunk::{
    Chunk,
    Op,
    Op::*,
};
use crate::debug::{
    dissassemble_instruction,
    print_value,
};
use crate::value::{
    Value,
    Value::*,
};
use crate::compile::Parser;

pub struct VM {
    pub chunk: Chunk,
    pub ip: usize,
    pub stack: Vec<Value>,
}

const STACK_MAX: usize = 256;
const DEBUG_TRACE_EXECUTION: bool = true;

macro_rules! binary_op {
    ($vm:expr, $op:tt) => {
        {
            if let Number(b) = $vm.pop() {
                if let Number(a) = $vm.pop() {
                    $vm.push(Number(a $op b));
                }
            } else {
                return InterpretResult::RuntimeError;
            }
        }
    };
}


impl VM {
    pub fn new() -> VM {
        VM {
            chunk: Chunk::new(),
            ip: 0,
            stack: Vec::with_capacity(STACK_MAX)
        }
    }

    pub fn init_vm(&mut self) {
        self.reset_stack();
        self.chunk = Chunk::new();
        self.ip = 0;
    }

    pub fn reset_stack(&mut self) {
        self.stack.clear();
    }

    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    pub fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    pub fn interpret(&mut self, source: String) -> InterpretResult {
        let mut parser = Parser::new();
        
        let mut chunk: Chunk = Chunk::new();
        if !parser.compile(source, &mut chunk) {
            return InterpretResult::CompileError;
        }

        self.chunk = chunk;
        self.run()
    }

    fn read_byte(&mut self) -> Op {
        let operation = self.chunk.code[self.ip].clone().0;
        self.ip += 1;
        operation
    }

    pub fn run(&mut self) -> InterpretResult {
        loop {
            if DEBUG_TRACE_EXECUTION {
                print!("          ");
                for value in self.stack.iter() {
                    print!("[ ");
                    print!("{:^10?} ", value);
                    print!("]");
                }
                println!();
                dissassemble_instruction(&self.chunk, self.ip);
            }
            let instruction = self.read_byte();
            match instruction {
                Constant(const_idx) => {
                    let constant = self.chunk.constants[const_idx].clone();
                    self.stack.push(constant);
                },
                Add => {
                    binary_op!(self, +);
                },
                Subtract => {
                    binary_op!(self, -);
                },
                Multiply => {
                    binary_op!(self, *);
                },
                Divide => {
                    binary_op!(self, /);
                },
                Negate => {
                    if let Number(num) = self.pop() {
                        self.push(Number(-num));
                    }
                    return InterpretResult::RuntimeError;
                },
                Return => {
                    println!("\n");
                    print!("Value: ");
                    print_value(&self.pop());
                    println!();
                    return InterpretResult::Ok;
                },
            }
        }
    }
}

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}