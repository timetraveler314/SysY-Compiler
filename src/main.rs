mod frontend;
mod backend;
mod common;

use std::fs::File;
use std::io::Write;
use koopa::back::KoopaGenerator;
use lalrpop_util::lalrpop_mod;
use common::environment::{AsmEnvironment, ROContext};
use crate::backend::generate_asm::GenerateAsm;

lalrpop_mod!(sysy);

fn main() -> std::io::Result<()> {
    let (mode, input_file, output_file) = parse_args(std::env::args().collect());

    let input = std::fs::read_to_string(input_file)?;
    let ast = sysy::CompUnitParser::new().parse(&input).unwrap();
    println!("AST Dump: {:?}", ast);
    let ir = frontend::generate_ir(&ast).unwrap();

    match mode {
        Mode::Koopa => {
            let mut output = File::create(&output_file)?;
            let mut gen = KoopaGenerator::new(Vec::new());
            gen.generate_on(&*ir.borrow())?;
            let text_form_ir = std::str::from_utf8(&gen.writer()).unwrap().to_string();
            println!("Writing IR to file: {}", output_file);
            output.write_all(text_form_ir.as_bytes())?;
        }
        Mode::Riscv => {
            let mut asm_program = backend::asm::AsmProgram {
                sections: Vec::new(),
            };
            let mut program = ir.borrow();
            let mut env = AsmEnvironment::new(&*program);
            (&*program).generate(&mut asm_program, &mut env);

            let mut riscv_output = File::create(output_file)?;
            println!("{:?}", asm_program);
            asm_program.emit(&mut riscv_output).expect("Failed to emit target code");
        }
        Mode::Unknown => unreachable!()
    }

    Ok(())
}

enum Mode {
    Koopa,
    Riscv,
    Unknown,
}

fn parse_args(args: Vec<String>) -> (Mode, String, String) {
    let mut mode = Mode::Unknown;
    let mut input_file = String::new();
    let mut output_file = String::new();

    for i in 1..args.len() {
        match args[i].as_str() {
            "-koopa" => {
                mode = Mode::Koopa;
            }
            "-riscv" => {
                mode = Mode::Riscv;
            }
            "-o" => {
                output_file = args[i + 1].clone();
            }
            _ => {
                if i >= 2 && args[i - 1] != "-o" {
                    input_file = args[i].clone();
                }
            }
        }
    }

    match mode {
        Mode::Unknown => {
            println!("One of -koopa or -riscv must be specified");
            std::process::exit(1);
        }
        _ => {}
    }

    if input_file.is_empty() || output_file.is_empty() {
        println!("Usage: {} [-koopa|-riscv] <input_file> -o <output_file>", args[0]);
        std::process::exit(1);
    }

    (mode, input_file, output_file)
}