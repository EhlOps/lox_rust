use std::io::Write;

use crate::value::{
    Value,
    Value::*,
};
use crate::scanner::{
    self,
    Token,
    TokenType,
};
use crate::chunk::{Chunk, Line, Op, line};
use crate::object::{Heap, HeapData};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Precedence {
    None = 0,
    Assignment = 1,
    Or = 2,
    And = 3,
    Equality = 4,
    Comparison = 5,
    Term = 6,
    Factor = 7,
    Unary = 8,
    Call = 9,
    Primary = 10,
}

impl Precedence {
    fn increase(&self) -> Precedence {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::Primary,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Local {
    token: Token,
    depth: isize,
}

pub struct Compiler {
    locals: Vec<Local>,
    scope_depth: isize,
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler {
            locals: Vec::new(),
            scope_depth: 0,
        }
    }
    
}

#[derive(Clone)]
pub struct ParseRule {
    pub prefix: Option<fn(&mut Parser, &String, &mut Chunk, &mut scanner::Scanner, &mut Heap, bool)>,
    pub infix: Option<fn(&mut Parser, &String, &mut Chunk, &mut scanner::Scanner, &mut Heap, bool)>,
    pub precedence: Precedence,
}

pub struct Parser<> {
    current: scanner::Token,
    previous: scanner::Token,
    had_error: bool,
    panic_mode: bool,
    compiler: Compiler,
}

impl Parser {
    pub fn new() -> Parser {
        Parser {
            current: Token {
                token_type: TokenType::EOF,
                start: 0,
                length: 0,
                line: 0,
            },
            previous: Token {
                token_type: TokenType::EOF,
                start: 0,
                length: 0,
                line: 0,
            },
            had_error: false,
            panic_mode: false,
            compiler: Compiler::new(),
        }
    }

    pub fn compile(&mut self, source: String, chunk: &mut Chunk, heap: &mut Heap) -> bool {
        let mut scanner = scanner::Scanner::new();
        self.advance(&source, &mut scanner);
        
        while !self.match_token(TokenType::EOF, &source, &mut scanner) {
            self.declaration(&source, chunk, &mut scanner, heap);
        }

        !self.had_error
    }

    pub fn advance(&mut self, source: &String, scanner: &mut scanner::Scanner) {
        self.previous = self.current.clone();

        loop {
            self.current = scanner.scan_token(&source);
            
            if self.current.token_type != TokenType::Error {
                break;
            }

            self.error_at_current(self.current.start.to_string().as_str());
        }
    }

    pub fn error_at_current(&mut self, message: &str) {
        self.error_at(&self.current.clone(), message);
    }

    pub fn error_at_previous(&mut self, message: &str) {
        self.error_at(&self.previous.clone(), message);
    }

    pub fn error_at(&mut self, token: &Token, message: &str) {
        self.panic_mode = true;
        write!(std::io::stderr(), "[line {}] Error", token.line).unwrap();

        if token.token_type == TokenType::EOF {
            write!(std::io::stderr(), " at end").unwrap();
        } else if token.token_type == TokenType::Error {
            // Nothing.
        } else {
            write!(std::io::stderr(), " at '{:.*}'", token.length, token.start).unwrap();
        }

        writeln!(std::io::stderr(), ": {}", message).unwrap();
        self.had_error = true;
    }

    pub fn add_local(&mut self, token: Token) {
        self.compiler.locals.push(Local {
            token: token,
            depth: -1,
        });
    }

    pub fn block(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap) {
        while !(self.current.token_type == TokenType::RightBrace) && !(self.current.token_type == TokenType::EOF) {
            self.declaration(source, chunk, scanner, heap);
        }

        self.consume(source, TokenType::RightBrace, "Expect '}' after block.", scanner);
    }

    pub fn begin_scope(&mut self) {
        self.compiler.scope_depth += 1;
    }

    pub fn end_scope(&mut self, chunk: &mut Chunk) {
        self.compiler.scope_depth -= 1;

        let locals_to_pop: usize = self.compiler.locals.iter().rev()
            .take_while(|local| local.depth > self.compiler.scope_depth).collect::<Vec<_>>().len();

        for _ in 0..locals_to_pop {
            self.emit_byte(chunk, (Op::Pop, line(self.previous.line)));
        }
    }

    pub fn consume(&mut self, source: &String, token_type: TokenType, message: &str, scanner: &mut scanner::Scanner) {
        if self.current.token_type == token_type {
            self.advance(source, scanner);
            return;
        }

        self.error_at_current(message);
    }

    pub fn declaration(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap) {
        if self.match_token(TokenType::Var, source, scanner) {
            self.var_declaration(source, chunk, scanner, heap);
        } else {
            self.statement(source, chunk, scanner, heap);
        } 
        
        if self.panic_mode {
            self.synchronize(source, scanner);
        }
    }

    pub fn declare_variable(&mut self, source: &String) {
        if self.compiler.scope_depth == 0 {
            return;
        }

        let name = self.previous.clone();
        let scope_depth = self.compiler.scope_depth;
        let locals = self.compiler.locals.clone();

        let is_same_token = |token1: &Token, token2: &Token| -> bool {
            source.chars().skip(token1.start).take(token1.length).collect::<String>() == source.chars().skip(token2.start).take(token2.length).collect::<String>()
        };

        for local in locals.iter().rev() {
            if local.depth != scope_depth {
                break;
            }

            if is_same_token(&name, &local.token) {
                self.error_at_current("Variable with this name already declared in this scope.");
            }
        }

        self.add_local(name);
    }

    pub fn define_variable(&mut self, chunk: &mut Chunk, global: usize) {
        if self.compiler.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_byte(chunk, (Op::DefineGlobal(global), line(self.previous.line)));
    }

    pub fn emit_byte(&mut self, chunk: &mut Chunk, (op, line): (Op, Line)) {
        chunk.code.push((op, line));
    }

    pub fn emit_bytes(&mut self, chunk: &mut Chunk, (op1, line1): (Op, Line), (op2, line2): (Op, Line)) {
        self.emit_byte(chunk, (op1, line1));
        self.emit_byte(chunk, (op2, line2));
    }

    pub fn emit_constant(&mut self, chunk: &mut Chunk, value: Value, line: Line) {
        if let Number(val) = value {
            let constant = chunk.add_constant(Number(val));
            if constant > 255 {
                self.error_at_previous("Too many constants in one chunk.");
            }
            self.emit_byte(chunk, (Op::Constant(constant), line));
        } else if let Obj(val) = value {
            let constant = chunk.add_constant(Obj(val));
            if constant > 255 {
                self.error_at_previous("Too many constants in one chunk.");
            }
            self.emit_byte(chunk, (Op::Constant(constant), line));
        }
    }

    pub fn identifier_constant(&mut self, source: &String, chunk: &mut Chunk, heap: &mut Heap) -> usize {
        let identifier = source.chars().skip(self.previous.start).take(self.previous.length).collect::<String>();
        chunk.add_constant(Value::Obj(heap.allocate(HeapData::String(identifier))))
    }

    pub fn match_token(&mut self, token_type: TokenType, source: &String, scanner: &mut scanner::Scanner, ) -> bool {
        if self.current.token_type != token_type {
            return false;
        }
        self.advance(source, scanner);
        true
    }

    pub fn mark_initialized(&mut self) {
        if self.compiler.scope_depth == 0 {
            return;
        }
        self.compiler.locals.last_mut().unwrap().depth = self.compiler.scope_depth;
    }

    pub fn named_variable(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap, can_assign: bool) {
        let global = self.identifier_constant(source, chunk, heap);
        let local = self.resolve_local(source);
        if can_assign && self.match_token(TokenType::Equal, source, scanner) {
            self.expression(source, chunk, scanner, heap, can_assign);
            if local != -1 {
                self.emit_byte(chunk, (Op::SetLocal(local as usize), line(self.previous.line)));
            } else {
                self.emit_byte(chunk, (Op::SetGlobal(global), line(self.previous.line)));
            }
        } else {
            if local != -1 {
                self.emit_byte(chunk, (Op::GetLocal(local as usize), line(self.previous.line)));
            } else {
                self.emit_byte(chunk, (Op::GetGlobal(global), line(self.previous.line)));
            }
        }
    }

    pub fn parse_variable(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap) -> usize {
        self.consume(source, TokenType::Identifier, "Expect variable name.", scanner);

        self.declare_variable(source);
        if self.compiler.scope_depth > 0 {
            return 0;
        }

        self.identifier_constant(source, chunk, heap)
    }

    pub fn print_statement(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap) {
        self.expression(source, chunk, scanner, heap, false);
        self.consume(source, TokenType::Semicolon, "Expect ';' after value.", scanner);
        self.emit_byte(chunk, (Op::Print, line(self.previous.line)));
    }

    pub fn resolve_local(&mut self, source: &String) -> isize {
        let name = source.chars().skip(self.previous.start).take(self.previous.length).collect::<String>();
        let locals = self.compiler.locals.clone();
        for (i, local) in locals.iter().enumerate().rev() {
            let local_name = source.chars().skip(local.token.start).take(local.token.length).collect::<String>();

            if name == local_name {
                if local.depth == -1 {
                    self.error_at_current("Cannot read local variable in its own initializer.");
                }
                return i as isize;
            }
        }
        -1
    }

    pub fn statement(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap) {
        if self.match_token(TokenType::Print, source, scanner) {
            self.print_statement(source, chunk, scanner, heap);
        } else if self.match_token(TokenType::LeftBrace, source, scanner) { 
            self.begin_scope();
            self.block(source, chunk, scanner, heap);
            self.end_scope(chunk);
        } else {
            self.expression(source, chunk, scanner, heap, false);
            self.consume(source, TokenType::Semicolon, "Expect ';' after expression.", scanner);

            // This might be used later.
            // self.emit_byte(chunk, (Op::Pop, line(self.previous.line)));
        }
    }

    pub fn synchronize(&mut self, source: &String, scanner: &mut scanner::Scanner) {
        self.panic_mode = false;

        while self.current.token_type != TokenType::EOF {
            if self.previous.token_type == TokenType::Semicolon {
                return;
            }

            match self.current.token_type {
                TokenType::Class | TokenType::Fun | TokenType::Var | TokenType::For | TokenType::If | TokenType::While | TokenType::Print | TokenType::Return => {
                    return;
                },
                _ => (),
            }

            self.advance(source, scanner);
        }
    }

    pub fn var_declaration(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap) {
        let global = self.parse_variable(source, chunk, scanner, heap);
        if self.match_token(TokenType::Equal, source, scanner) {
            self.expression(source, chunk, scanner, heap, false);
        } else {
            self.emit_byte(chunk, (Op::Nil, line(self.previous.line)));
        }
        self.consume(source, TokenType::Semicolon, "Expect ';' after variable declaration.", scanner);
        self.define_variable(chunk, global);
    }

    // PREFIXES AND INFIXES ----------------------------------------------------

    pub fn literal(&mut self, _source: &String, chunk: &mut Chunk, _scanner: &mut scanner::Scanner, _heap: &mut Heap, _can_assign: bool) {
        match self.previous.token_type {
            TokenType::False => self.emit_byte(chunk, (Op::False, line(self.previous.line))),
            TokenType::Nil => self.emit_byte(chunk, (Op::Nil, line(self.previous.line))),
            TokenType::True => self.emit_byte(chunk, (Op::True, line(self.previous.line))),
            _ => (),
        }
    }

    pub fn string(&mut self, source: &String, chunk: &mut Chunk, _scanner: &mut scanner::Scanner, heap: &mut Heap, _can_assign: bool) {
        let string = source.chars().skip(self.previous.start + 1).take(self.previous.length - 2).collect::<String>();
        let heap_id = heap.allocate(HeapData::String(string));
        self.emit_constant(chunk, Obj(heap_id), line(self.previous.line));
    }

    pub fn expression(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap, _can_assign: bool) {
        self.parse_precedence(source, chunk, Precedence::Assignment, scanner, heap);
    }

    pub fn variable(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap, can_assign: bool) {
        self.named_variable(source, chunk, scanner, heap, can_assign);
    }

    pub fn number(&mut self, source: &String, chunk: &mut Chunk, _scanner: &mut scanner::Scanner, _heap: &mut Heap, _can_assign: bool) {
        let val = source[self.previous.start..self.previous.start + self.previous.length].parse::<f64>().unwrap();
        self.emit_constant(chunk, Number(val), line(self.previous.line));
    }

    pub fn grouping(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap, _can_assign: bool) {
        self.expression(source, chunk, scanner, heap, false);
        self.consume(source, TokenType::RightParen, "Expect ')' after expression.", scanner);
    }

    pub fn unary(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap, _can_assign: bool) {
        let operator_type = self.previous.clone().token_type;
        self.parse_precedence(source, chunk,  Precedence::Unary, scanner, heap);
        match operator_type {
            TokenType::Bang => self.emit_byte(chunk, (Op::Not, line(self.previous.line))),
            TokenType::Minus => self.emit_byte(chunk, (Op::Negate, line(self.previous.line))),
            _ => (),
        }
    }

    pub fn binary(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap, _can_assign: bool) {
        let operator_type = self.previous.clone().token_type;
        let rule = self.get_rule(&operator_type);
        self.parse_precedence(source, chunk, rule.precedence.clone().increase(), scanner, heap);
        match operator_type {
            TokenType::BangEqual => {
                self.emit_bytes(chunk, (Op::Equal, line(self.previous.line)), (Op::Not, line(self.previous.line)));
            },
            TokenType::EqualEqual => self.emit_byte(chunk, (Op::Equal, line(self.previous.line))),
            TokenType::Greater => self.emit_byte(chunk, (Op::Greater, line(self.previous.line))),
            TokenType::GreaterEqual => {
                self.emit_byte(chunk, (Op::Less, line(self.previous.line)));
                self.emit_byte(chunk, (Op::Not, line(self.previous.line)));
            },
            TokenType::Less => self.emit_byte(chunk, (Op::Less, line(self.previous.line))),
            TokenType::LessEqual => {
                self.emit_byte(chunk, (Op::Greater, line(self.previous.line)));
                self.emit_byte(chunk, (Op::Not, line(self.previous.line)));
            },
            TokenType::Plus => self.emit_byte(chunk, (Op::Add, line(self.previous.line))),
            TokenType::Minus => self.emit_byte(chunk, (Op::Subtract, line(self.previous.line))),
            TokenType::Star => self.emit_byte(chunk, (Op::Multiply, line(self.previous.line))),
            TokenType::Slash => self.emit_byte(chunk, (Op::Divide, line(self.previous.line))),
            _ => (),
        }

    }

    pub fn get_rule(&self, token_type: &TokenType) -> ParseRule {
        match token_type {
            TokenType::LeftParen => ParseRule {
                prefix: Some(Parser::grouping),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::RightParen => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::LeftBrace => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::RightBrace => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Comma => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Dot => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Minus => ParseRule {
                prefix: Some(Parser::unary),
                infix: Some(Parser::binary),
                precedence: Precedence::Term,
            },
            TokenType::Plus => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Term,
            },
            TokenType::Semicolon => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Slash => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Factor,
            },
            TokenType::Star => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Factor,
            },
            TokenType::Bang => ParseRule {
                prefix: Some(Parser::unary),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::BangEqual => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Equality,
            },
            TokenType::Equal => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::EqualEqual => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Equality,
            },
            TokenType::Greater => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },
            TokenType::GreaterEqual => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },
            TokenType::Less => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },
            TokenType::LessEqual => ParseRule {
                prefix: None,
                infix: Some(Parser::binary),
                precedence: Precedence::Comparison,
            },
            TokenType::Identifier => ParseRule {
                prefix: Some(Parser::variable),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::String => ParseRule {
                prefix: Some(Parser::string),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Number => ParseRule {
                prefix: Some(Parser::number),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::And => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Class => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Else => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::False => ParseRule {
                prefix: Some(Parser::literal),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::For => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Fun => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::If => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Nil => ParseRule {
                prefix: Some(Parser::literal),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Or => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Print => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Return => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Super => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::This => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::True => ParseRule {
                prefix: Some(Parser::literal),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Var => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::While => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Error => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::EOF => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        }
    }

    pub fn parse_precedence(&mut self, source: &String, chunk: &mut Chunk, precedence: Precedence, scanner: &mut scanner::Scanner, heap: &mut Heap) {
        self.advance(source, scanner);
        let prefix_rule = self.get_rule(&self.previous.token_type).prefix;
        let can_assign = precedence <= Precedence::Assignment;
        if let Some(prefix) = prefix_rule {
            prefix(self, source, chunk, scanner, heap, can_assign);
        }

        while precedence <= self.get_rule(&self.current.token_type).precedence {
            self.advance(source, scanner);
            let infix_rule = self.get_rule(&self.previous.token_type).infix;
            if let Some(infix) = infix_rule {
                infix(self, source, chunk, scanner, heap, false);
            }
        }
    }
}