use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::Parser;
use oxc_span::SourceType;

use std::env::args;

use px_deobfuscator::transform::Transform;

pub fn parse(js_code: &str) -> () {
    let source_type: SourceType = SourceType::default().with_module(true);
    let allocator = Allocator::default();

    let parsed = Parser::new(&allocator, js_code, source_type).parse();

    let program = allocator.alloc(parsed.program);
    let mut transform = Transform::new(&allocator);

    transform.transform(program);

    let source_text = Codegen::new()
        .with_options(CodegenOptions::default())
        .build(program)
        .code;

    std::fs::write("output.js", source_text).unwrap();
}

fn main() {
    let start = std::time::Instant::now();

    let args = args().collect::<Vec<String>>();
    let mut file = &args[1]; // ./px-deobfuscator <file>
    if file == "." {
        // cargo run . <file>
        file = &args[2];
    }

    let js_code = std::fs::read_to_string(file).unwrap();

    parse(js_code.as_str());

    println!("Elapsed: {:?}", start.elapsed());
}
