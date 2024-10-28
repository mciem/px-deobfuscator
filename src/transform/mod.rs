use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::AstBuilder;
use oxc_ast::VisitMut;
use std::collections::HashMap;

mod visit;

use visit::{functions::FunctionVisitor, strings::StringsVisitor};

pub struct Transform<'a> {
    pub allocator: &'a Allocator,
    pub ast: AstBuilder<'a>,
    pub imports: HashMap<String, String>,
}

impl<'a> Transform<'a> {
    pub fn new(allocator: &'a Allocator) -> Self {
        Self {
            allocator,
            ast: AstBuilder::new(allocator),
            imports: HashMap::new(),
        }
    }

    pub fn get_imports(&self) -> &HashMap<String, String> {
        &self.imports
    }

    pub fn transform(&mut self, program: &mut Program<'a>) {
        let mut func_visitor = FunctionVisitor::new(self.allocator);
        func_visitor.visit_program(program);

        StringsVisitor::new(self.allocator, func_visitor.functions).visit_program(program);
    }
}
