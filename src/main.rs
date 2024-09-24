extern crate clap;

use clap::{Arg, Command};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{enable_raw_mode, disable_raw_mode},
    ExecutableCommand,
};
use std::io::{self, Write};

use std::fs;

mod chunk;
mod debug;
mod value;
mod vm;
mod compile;
mod scanner;
mod object;

const INPUT: &str = "Script";

fn get_input(matches: &clap::ArgMatches) -> Option<&String> {
    if let Some(file_input) = matches.get_one::<String>(INPUT) {
        return Some(file_input);
    }
    None
}

fn main() {
    let mut vm: vm::VM = vm::VM::new();
    vm.init_vm();

    let arg_matches = Command::new("lox")
        .version("0.1.0")
        .author("Sam Ehlers")
        .about("A lox interpreter created from the book Crafting Interpreters")
        .arg(Arg::new(INPUT)
            .help("lox script to run")
            .required(false)
            .index(1))
        .get_matches();

    if let Some(input) = get_input(&arg_matches) {
        run_file(&mut vm, input);
    } else {
        repl(&mut vm);
    }

    drop(vm);
}

fn run_file(vm: &mut vm::VM, input: &String) {
    let source = read_file(input.to_string());
    println!("{source}");
    let result: vm::InterpretResult  = vm.interpret(source);
    match result {
        vm::InterpretResult::Ok => (),
        vm::InterpretResult::CompileError => std::process::exit(65),
        vm::InterpretResult::RuntimeError => std::process::exit(70),
    }
}

fn read_file(file: String) -> String {
    match fs::read_to_string(file) {
        Ok(input) => return input,
        Err(err) => {
            eprintln!("Error reading file: {}", err);
            std::process::exit(74);
        }
    }
}

fn repl(vm: &mut vm::VM) {
    vm.init_vm();

    let mut input_history: Vec<String> = Vec::new();
    let mut current_input: String = String::new();
    let mut history_index: Option<usize> = None;

    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    stdout.execute(crossterm::terminal::Clear(crossterm::terminal::ClearType::All)).unwrap();

    print!("Welcome to Lox!\r\n");
    print!("Type `exit` to exit the console.\r\n");
    print!("> ");

    loop {
        io::stdout().flush().unwrap();        
        if event::poll(std::time::Duration::from_millis(100)).unwrap() {
            if let Event::Key(key_event) = event::read().unwrap() {
                match key_event.code {
                    KeyCode::Char(c) => {
                        current_input.push(c);
                        print!("{}", c);
                        io::stdout().flush().unwrap();
                    },
                    KeyCode::Enter => {
                        if !current_input.is_empty() {
                            if current_input == "exit" {
                                break;
                            }
                            if current_input == "stack" {
                                current_input.clear();
                                unsafe {
                                    vm::DEBUG_TRACE_EXECUTION = true;
                                }
                                print!("\r\nFurther commands will show the stack...\r\n\r\n");
                                print!("> ");
                                continue;
                            }
                            if current_input == "nostack" {
                                current_input.clear();
                                unsafe {
                                    vm::DEBUG_TRACE_EXECUTION = false;
                                }
                                print!("\r\nFurther commands will not show the stack...\r\n\r\n");
                                print!("> ");
                                continue;
                            }
                            print!("\r\n");
                            input_history.push(current_input.clone());
                            vm.interpret(current_input.clone());
                            print!("\r\n");
                            clear_line();
                            print!("> ");
                            current_input.clear();
                            history_index = None;
                        }
                    },
                    KeyCode::Backspace => {
                        if !current_input.is_empty() {
                            current_input.pop();
                            print!("\r{}", clear_line());
                            print!("> {}", current_input);
                            io::stdout().flush().unwrap();
                        }
                    },
                    KeyCode::Up => {
                        if let Some(idx) = history_index {
                            if idx > 0 {
                                history_index = Some(idx - 1);
                            }
                        } else if !input_history.is_empty() {
                            history_index = Some(input_history.len() - 1);
                        }
                        if let Some(idx) = history_index {
                            current_input = input_history[idx].clone();
                            print!("\r{}", clear_line());
                            print!("> {}", current_input);
                            io::stdout().flush().unwrap();
                        }
                    },
                    KeyCode::Down => {
                        if let Some(idx) = history_index {
                            if idx < input_history.len() - 1 {
                                history_index = Some(idx + 1);
                            } else {
                                history_index = None;
                                current_input.clear();
                                print!("\r{}", clear_line());
                                print!("> {}", current_input);
                            }
                        }
                        if let Some(idx) = history_index {
                            current_input = input_history[idx].clone();
                            print!("\r{}", clear_line());
                            print!("> {}", current_input);
                            io::stdout().flush().unwrap();
                        }
                    },
                    _ => (),
                }
            }
        }
    }
    disable_raw_mode().unwrap();
    println!();
    println!("Goodbye!");
}

fn clear_line() -> &'static str {
    "\x1b[2K\x1b[1G"
}