mod frontend;

use std::env::args;
use std::io::Write;
use koopa::back::KoopaGenerator;
use lalrpop_util::lalrpop_mod;

lalrpop_mod!(sysy);

fn main() -> std::io::Result<()> {
    let mut args = args();
    args.next();
    let _mode = args.next().unwrap();
    let input = args.next().unwrap();
    args.next();
    let output = args.next().unwrap();

    let input = std::fs::read_to_string(input)?;
    let ast = sysy::CompUnitParser::new().parse(&input).unwrap();
    // Dump AST
    println!("{:?}", ast);
    let ir = frontend::generate_ir(&ast).unwrap();

    // Open output file and write IR to output file
    let mut output = std::fs::File::create(output)?;
    let mut gen = KoopaGenerator::new(Vec::new());
    gen.generate_on(&ir)?;
    let text_form_ir = std::str::from_utf8(&gen.writer()).unwrap().to_string();
    output.write_all(text_form_ir.as_bytes())?;

    Ok(())
}
