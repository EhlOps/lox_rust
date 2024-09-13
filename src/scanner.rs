pub struct Scanner {
    start: usize,
    current: usize,
    line: usize,
}

impl Scanner {
    pub fn new() -> Scanner {
        Scanner {
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_token(&mut self, source: &String) -> Token {
        self.skip_whitespace(source);
        self.start = self.current;

        if self.is_at_end(source) {
            return self.make_token(TokenType::EOF);
        }

        match self.advance(source) {
            '(' => return self.make_token(TokenType::LeftParen),
            ')' => return self.make_token(TokenType::RightParen),
            '{' => return self.make_token(TokenType::LeftBrace),
            '}' => return self.make_token(TokenType::RightBrace),
            ';' => return self.make_token(TokenType::Semicolon),
            ',' => return self.make_token(TokenType::Comma),
            '.' => return self.make_token(TokenType::Dot),
            '-' => return self.make_token(TokenType::Minus),
            '+' => return self.make_token(TokenType::Plus),
            '/' => return self.make_token(TokenType::Slash),
            '*' => return self.make_token(TokenType::Star),
            '!' => {
                let m = self.match_next(source, '=');
                return self.make_token(if m {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                });
            },
            '=' => {
                let m = self.match_next(source, '=');
                return self.make_token(if m {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                });
            },
            '<' => {
                let m = self.match_next(source, '=');
                return self.make_token(if m {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                });
            },
            '>' => {
                let m = self.match_next(source, '=');
                return self.make_token(if m {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                });
            },
            '"' => {
                return self.string_token(source);
            },
            '0'..='9' => {
                return self.number_token(source);
            },
            'a'..='z' | 'A'..='Z' => {
                while self.peek(source).is_alphanumeric() {
                    self.advance(source);
                }

                return self.make_token(self.identifier_type(source));
            },
            _ => ()
        }
    
        self.error_token("Unexpected character.")
    }

    fn make_token(&self, token_type: TokenType) -> Token {
        Token {
            token_type,
            start: self.start,
            length: self.current - self.start,
            line: self.line,
        }
    }

    fn error_token(&self, message: &str) -> Token {
        Token {
            token_type: TokenType::Error,
            start: 0,
            length: message.len(),
            line: self.line,
        }
    }

    fn identifier_type(&self, source: &String) -> TokenType {
        match source.chars().nth(self.start).unwrap() {
            'a' => return self.check_keyword(source, 1, 2, "nd", TokenType::And),
            'c' => return self.check_keyword(source, 1, 4, "lass", TokenType::Class),
            'e' => return self.check_keyword(source, 1, 3, "lse", TokenType::Else),
            'f' => {
                match source.chars().nth(self.start + 1).unwrap() {
                    'a' => return self.check_keyword(source, 2, 3, "lse", TokenType::False),
                    'o' => return self.check_keyword(source, 2, 1, "r", TokenType::For),
                    'u' => return self.check_keyword(source, 2, 1, "n", TokenType::Fun),
                    _ => ()
                }
            },
            'i' => return self.check_keyword(source, 1, 1, "f", TokenType::If),
            'n' => return self.check_keyword(source, 1, 2, "il", TokenType::Nil),
            'o' => return self.check_keyword(source, 1, 1, "r", TokenType::Or),
            'p' => return self.check_keyword(source, 1, 4, "rint", TokenType::Print),
            'r' => return self.check_keyword(source, 1, 5, "eturn", TokenType::Return),
            's' => return self.check_keyword(source, 1, 4, "uper", TokenType::Super),
            't' => {
                match source.chars().nth(self.start + 1).unwrap() {
                    'h' => return self.check_keyword(source, 2, 2, "is", TokenType::This),
                    'r' => return self.check_keyword(source, 2, 2, "ue", TokenType::True),
                    _ => ()
                }
            },
            'v' => {
                return self.check_keyword(source, 1, 2, "ar", TokenType::Var)
            },
            'w' => return self.check_keyword(source, 1, 4, "hile", TokenType::While),
            _ => ()
        }

        return TokenType::Identifier;
    }

    fn check_keyword(&self, source: &String, start: usize, length: usize, rest: &str, token_type: TokenType) -> TokenType {
        let some_chars = &source[self.start + start..self.start + start + length];
        if some_chars == rest {
            return token_type;
        }

        TokenType::Identifier
    }

    fn advance(&mut self, source: &String) -> char {
        self.current += 1;
        source.chars().nth(self.current - 1).unwrap()
    }

    fn peek(&self, source: &String) -> char {
        if self.is_at_end(source) {
            return '\0';
        }
        source.chars().nth(self.current).unwrap()
    }

    fn peek_next(&self, source: &String) -> char {
        if self.is_at_end(source) {
            return '\0';
        }
        source.chars().nth(self.current + 1).unwrap()
    }

    fn skip_whitespace(&mut self, source: &String) {
        loop {
            let c: char = self.peek(source);
            match c {
                ' ' | '\r' | '\t' => {
                    self.advance(source);
                },
                '\n' => {
                    self.line += 1;
                    self.advance(source);
                },
                '/' => {
                    if self.peek_next(source) == '/' {
                        while self.peek(source) != '\n' && !self.is_at_end(source) {
                            self.advance(source);
                        }
                    } else {
                        break;
                    }
                },
                _ => {
                    break;
                }
            }
        }
    }

    fn match_next(&mut self, source: &String, expected: char) -> bool {
        if self.is_at_end(source) {
            return false;
        }
        if source.chars().nth(self.current).unwrap() != expected {
            return false;
        }

        self.current += 1;
        true
    }

    fn string_token(&mut self, source: &String) -> Token {
        while self.peek(source) != '"' && !self.is_at_end(source) {
            if self.peek(source) == '\n' {
                self.line += 1;
            }
            self.advance(source);
        }

        if self.is_at_end(source) {
            return self.error_token("Unterminated string.");
        }

        self.advance(source);
        self.make_token(TokenType::String)
    }

    fn number_token(&mut self, source: &String) -> Token {
        while self.peek(source).is_digit(10) {
            self.advance(source);
        }

        if self.peek(source) == '.' && self.peek_next(source).is_digit(10) {
            self.advance(source);
            while self.peek(source).is_digit(10) {
                self.advance(source);
            }
        }

        self.make_token(TokenType::Number)
    }

    fn is_at_end(&self, source: &String) -> bool {
        self.current >= source.len()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub start: usize,
    pub length: usize,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Identifier,
    String,
    Number,
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    Error,
    EOF,
}