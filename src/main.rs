extern crate clap;

use clap::{Arg, Command};

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
    loop {
        vm.init_vm();
        let mut input = String::with_capacity(1024);
        std::io::stdin().read_line(&mut input).unwrap();
        vm.interpret(input);
    }
}