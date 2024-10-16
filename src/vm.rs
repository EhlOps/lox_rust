use std::any::{type_name, Any};
use std::collections::HashMap;

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

use crate::object::{
    Heap,
    HeapData,
};

pub struct VM {
    pub chunk: Chunk,
    pub ip: usize,
    pub stack: Vec<Value>,
    pub globals: HashMap<String, Value>,
    pub heap: Heap,
}

const STACK_MAX: usize = 256;
pub static mut DEBUG_TRACE_EXECUTION: bool = false;

macro_rules! binary_op {
    ($vm:expr, $valType:path, $op:tt) => {
        {
            if let Number(b) = $vm.pop() {
                if let Number(a) = $vm.pop() {
                    $vm.push($valType((a $op b)));
                }
            } else {
                $vm.runtime_error(format!("Operands must be numbers"));
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
            stack: Vec::with_capacity(STACK_MAX),
            globals: HashMap::new(),
            heap: Heap::new(),
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

    pub fn peek(&self, distance: usize) -> Value {
        self.stack[self.stack.len() - 1 - distance].clone()
    }

    pub fn equal(&self, a: Value, b: Value) -> bool {
        if a.type_id() != b.type_id() {
            return false;
        } else {
            match a {
                Bool(a) => {
                    if let Bool(b) = b {
                        a == b
                    } else {
                        false
                    }
                },
                Number(a) => {
                    if let Number(b) = b {
                        a == b
                    } else {
                        false
                    }
                },
                Value::Nil => {
                    return true;
                },
                Obj(obj_id_a) => {
                    if let Obj(obj_id_b) = b {
                        let obj_a = self.heap.get(&obj_id_a).unwrap();
                        let obj_b = self.heap.get(&obj_id_b).unwrap();
                        match obj_a {
                            HeapData::String(a) => {
                                if let HeapData::String(b) = obj_b {
                                    a == b
                                } else {
                                    false
                                }
                            }
                            _ => {
                                false
                            }
                        }
                    } else {
                        false
                    }
                },
                _ => {
                    return false;
                }
            }
        }
    }

    pub fn interpret(&mut self, source: String) -> InterpretResult {
        let mut parser = Parser::new();
        
        let mut chunk: Chunk = Chunk::new();

        if !parser.compile(source, &mut chunk, &mut self.heap) {
            return InterpretResult::CompileError;
        }

        self.chunk = chunk;
        self.ip = 0;
        self.stack.clear();

        let result = self.run();
        result
    }

    fn read_byte(&mut self) -> Op {
        let operation = self.chunk.code[self.ip].clone().0;
        self.ip += 1;
        operation
    }

    fn runtime_error(&mut self, format: String) {
        println!("{}", format);
        let instruction = self.chunk.code[self.ip - 1].clone();
        println!("{:?}", instruction.0);
        println!("[line {:?}] in script", instruction.1);
        self.stack.clear();
    } 

    pub fn run(&mut self) -> InterpretResult {
        loop {
            self.debug_trace_stack();
            let instruction = self.read_byte();
            match instruction {
                Constant(const_idx) => {
                    let constant = self.chunk.constants[const_idx].clone();
                    self.stack.push(constant);
                },
                Op::Nil => {
                    self.stack.push(Value::Nil);
                },
                Op::True => {
                    self.stack.push(Bool(true));
                },
                Op::False => {
                    self.stack.push(Bool(false));
                },
                Pop => {
                    self.pop();
                },
                GetLocal(local_idx) => {
                    let value = self.stack[local_idx].clone();
                    self.push(value);
                },
                SetLocal(local_idx) => {
                    let value = self.peek(0);
                    self.stack[local_idx] = value;
                },
                GetGlobal(const_idx) => {
                    if let Obj(heap_id) = self.chunk.constants[const_idx].clone() {
                        let obj = self.heap.get(&heap_id).unwrap();
                        match obj {
                            HeapData::String(string) => {
                                match self.globals.get(string) {
                                    Some(value) => {
                                        self.push(value.clone());
                                    },
                                    None => {
                                        self.runtime_error(format!("Undefined variable '{}'", string));
                                        return InterpretResult::RuntimeError;
                                    }
                                }
                            }
                            _ => {
                                self.runtime_error(format!("Expected string as global variable name"));
                                return InterpretResult::RuntimeError;
                            }
                        }
                    }
                },
                DefineGlobal(const_idx) => {
                    let val = self.pop();
                    if let Obj(heap_id) = self.chunk.constants[const_idx].clone() {
                        let obj = self.heap.get(&heap_id).unwrap();
                        match obj {
                            HeapData::String(string) => {
                                self.globals.insert(string.clone(), val);
                            },
                            _ => {
                                self.runtime_error(format!("Expected string as global variable name"));
                                return InterpretResult::RuntimeError;
                            }
                        }
                    }
                    if self.ip >= self.chunk.code.len() {
                        return InterpretResult::Ok;
                    }
                },
                SetGlobal(const_idx) => {
                    let val = self.pop();
                    if let Obj(heap_id) = self.chunk.constants[const_idx].clone() {
                        let obj = self.heap.get(&heap_id).unwrap();
                        match obj {
                            HeapData::String(string) => {
                                if self.globals.contains_key(string) {
                                    self.globals.insert(string.clone(), val);
                                } else {
                                    self.runtime_error(format!("Undefined variable '{}'", string));
                                    return InterpretResult::RuntimeError;
                                }
                            }
                            _ => {
                                self.runtime_error(format!("Expected string as global variable name"));
                                return InterpretResult::RuntimeError;
                            }
                        }
                    }
                    if self.ip >= self.chunk.code.len() {
                        return InterpretResult::Ok;
                    }
                }
                Equal => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Bool(self.equal(a, b)));
                },
                Greater => {
                    binary_op!(self, Bool, >);
                },
                Less => {
                    binary_op!(self, Bool, <);
                },
                Add => {
                    let b = self.pop();
                    let a = self.pop();
                    if let Number(a) = a {
                        if let Number(b) = b {
                            self.push(Number(a + b));
                        }
                        else if let Obj(b) = b {
                            let obj_b = self.heap.get(&b).unwrap();
                            match obj_b {
                                HeapData::String(b_string) => {
                                    let mut new_string = b_string.clone();
                                    new_string.insert_str(0, &a.to_string());
                                    let new_obj = self.heap.allocate(HeapData::String(new_string));
                                    self.push(Obj(new_obj));
                                    self.heap.free(b);
                                },
                                _ => {
                                    self.runtime_error(format!("Operands must be two numbers or two strings or one of each"));
                                    return InterpretResult::RuntimeError;
                                }
                            }
                        }
                    } else if let Obj(a) = a {
                        if let Obj(b) = b {
                            let obj_a = self.heap.get(&a).unwrap();
                            let obj_b = self.heap.get(&b).unwrap();
                            match obj_a {
                                HeapData::String(a_string) => {
                                    match obj_b {
                                        HeapData::String(b_string) => {
                                            let mut new_string = a_string.clone();
                                            new_string.push_str(&b_string);
                                            let new_obj = self.heap.allocate(HeapData::String(new_string));
                                            self.push(Obj(new_obj));
                                            self.heap.free(a);
                                            self.heap.free(b);
                                        },
                                        _ => {
                                            self.runtime_error(format!("Operands must be two numbers or two strings or one of each"));
                                            return InterpretResult::RuntimeError;
                                        }
                                    }
                                },
                                _ => {
                                    self.runtime_error(format!("Operands must be two numbers or two strings or one of each"));
                                    return InterpretResult::RuntimeError;
                                }
                            }
                        } else if let Number(b) = b {
                            let obj_a = self.heap.get(&a).unwrap();
                            match obj_a {
                                HeapData::String(a_string) => {
                                    let mut new_string = a_string.clone();
                                    new_string.push_str(&b.to_string());
                                    let new_obj = self.heap.allocate(HeapData::String(new_string));
                                    self.push(Obj(new_obj));
                                    self.heap.free(a);
                                },
                                _ => {
                                    self.runtime_error(format!("Operands must be two numbers or two strings or one of each"));
                                    return InterpretResult::RuntimeError;
                                }
                            }

                        }
                    } else {
                        self.runtime_error(format!("Operands must be two numbers or two strings or one of each"));
                        return InterpretResult::RuntimeError;
                    }
                },
                Subtract => {
                    binary_op!(self, Number, -);
                },
                Multiply => {
                    binary_op!(self, Number, *);
                },
                Divide => {
                    binary_op!(self, Number, /);
                },
                Not => {
                    let val = self.pop();
                    if let Bool(value) = val {
                        self.push(Bool(!value));
                    } else if val == Value::Nil {
                        self.push(Bool(true));
                    } else {
                        self.runtime_error(format!("Operand must be a boolean"));
                        return InterpretResult::RuntimeError;
                    }
                }
                Negate => {
                    if let Number(num) = self.pop() {
                        self.push(Number(-num));
                    } else {
                        self.runtime_error(format!("Operand must be a number"));
                        return InterpretResult::RuntimeError;
                    }
                },
                Print => {
                    if unsafe{!DEBUG_TRACE_EXECUTION} {
                        print_value(&self.pop(), &self.heap);
                        println!();
                    }
                    if self.ip >= self.chunk.code.len() {
                        return InterpretResult::Ok;
                    }
                },
                JumpIfFalse(offset) => {
                    let val = self.peek(0);
                    if val == Value::Nil || (val == Bool(false)) {
                        self.ip += offset;
                    }
                    if self.ip >= self.chunk.code.len() {
                        return InterpretResult::Ok;
                    }
                },
                Jump(offset) => {
                    self.ip += offset;
                    if self.ip >= self.chunk.code.len() {
                        return InterpretResult::Ok;
                    }
                },
                Loop(offset) => {
                    self.ip -= offset;
                    if self.ip >= self.chunk.code.len() {
                        return InterpretResult::Ok;
                    }
                },
                Return => {
                    return InterpretResult::Ok;
                },
            }
            if self.ip >= self.chunk.code.len() {
                return InterpretResult::Ok;
            }
        }
    }

    fn debug_trace_stack(&self) {
        unsafe {
            if !DEBUG_TRACE_EXECUTION {
                return;
            }
        }
        print!("          ");
        for value in self.stack.iter() {
            if let Value::Obj(obj) = value {
                let obj = self.heap.get(obj).unwrap();
                print!("[ ");
                print!("Obj({:^10}) ", format!("\"{}\"", obj.as_string()));
                print!("]");
            } else {
                print!("[ ");
                print!("{:^10?} ", value);
                print!("]");
            }
        }
        print!("\r\n");
        dissassemble_instruction(&self.chunk, &self.heap, self.ip);
    }
}

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}