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
use crate::object::{Heap, HeapVal, HeapData};

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

#[derive(Clone)]
pub struct ParseRule {
    pub prefix: Option<fn(&mut Parser, &String, &mut Chunk, &mut scanner::Scanner, &mut Heap)>,
    pub infix: Option<fn(&mut Parser, &String, &mut Chunk, &mut scanner::Scanner, &mut Heap)>,
    pub precedence: Precedence,
}

pub struct Parser<> {
    current: scanner::Token,
    previous: scanner::Token,
    had_error: bool,
    panic_mode: bool,
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
        }
    }

    pub fn compile(&mut self, source: String, chunk: &mut Chunk, heap: &mut Heap) -> bool {
        let mut scanner = scanner::Scanner::new();
        self.advance(&source, &mut scanner);
        self.expression(&source, chunk, &mut scanner, heap);
        self.consume(&source, TokenType::EOF, "Expect end of expression.", &mut scanner);
        self.emit_byte(chunk, (Op::Return, line(self.previous.line)));

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

    pub fn consume(&mut self, source: &String, token_type: TokenType, message: &str, scanner: &mut scanner::Scanner) {
        if self.current.token_type == token_type {
            self.advance(source, scanner);
            return;
        }

        self.error_at_current(message);
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

    pub fn literal(&mut self, _source: &String, chunk: &mut Chunk, _scanner: &mut scanner::Scanner, _heap: &mut Heap) {
        match self.previous.token_type {
            TokenType::False => self.emit_byte(chunk, (Op::False, line(self.previous.line))),
            TokenType::Nil => self.emit_byte(chunk, (Op::Nil, line(self.previous.line))),
            TokenType::True => self.emit_byte(chunk, (Op::True, line(self.previous.line))),
            _ => (),
        }
    }

    pub fn string(&mut self, source: &String, chunk: &mut Chunk, _scanner: &mut scanner::Scanner, heap: &mut Heap) {
        let string = source.chars().skip(self.previous.start + 1).take(self.previous.length - 2).collect::<String>();
        let heap_id = heap.allocate(HeapData::String(string));
        self.emit_constant(chunk, Obj(heap_id), line(self.previous.line));
    }

    pub fn expression(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap) {
        self.parse_precedence(source, chunk, Precedence::Assignment, scanner, heap);
    }

    pub fn number(&mut self, source: &String, chunk: &mut Chunk, _scanner: &mut scanner::Scanner, _heap: &mut Heap) {
        let val = source[self.previous.start..self.previous.start + self.previous.length].parse::<f64>().unwrap();
        self.emit_constant(chunk, Number(val), line(self.previous.line));
    }

    pub fn grouping(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap) {
        self.expression(source, chunk, scanner, heap);
        self.consume(source, TokenType::RightParen, "Expect ')' after expression.", scanner);
    }

    pub fn unary(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap) {
        let operator_type = self.previous.clone().token_type;
        self.parse_precedence(source, chunk,  Precedence::Unary, scanner, heap);
        match operator_type {
            TokenType::Bang => self.emit_byte(chunk, (Op::Not, line(self.previous.line))),
            TokenType::Minus => self.emit_byte(chunk, (Op::Negate, line(self.previous.line))),
            _ => (),
        }
    }

    pub fn binary(&mut self, source: &String, chunk: &mut Chunk, scanner: &mut scanner::Scanner, heap: &mut Heap) {
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
                prefix: None,
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
        if let Some(prefix) = prefix_rule {
            prefix(self, source, chunk, scanner, heap);
        }

        while precedence <= self.get_rule(&self.current.token_type).precedence {
            self.advance(source, scanner);
            let infix_rule = self.get_rule(&self.previous.token_type).infix;
            if let Some(infix) = infix_rule {
                infix(self, source, chunk, scanner, heap);
            }
        }
    }
}