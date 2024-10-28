use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::{visit::walk_mut, AstBuilder, VisitMut};
use oxc_span::SPAN;

use rustc_hash::FxHashMap;

use super::functions::{Function, Values};

pub struct StringsVisitor<'a> {
    ast: AstBuilder<'a>,

    functions: FxHashMap<String, Function>,
    proxies: FxHashMap<String, String>,

    values: FxHashMap<String, Values>,
    current_function: Option<String>,
}

impl<'a> StringsVisitor<'a> {
    pub fn new(allocator: &'a Allocator, functions: FxHashMap<String, Function>) -> Self {
        Self {
            ast: AstBuilder::new(allocator),
            functions,
            proxies: FxHashMap::default(),
            values: FxHashMap::default(),
            current_function: None,
        }
    }
}

impl<'a> VisitMut<'a> for StringsVisitor<'a> {
    fn visit_function(&mut self, it: &mut oxc_ast::ast::Function<'a>, _: oxc_semantic::ScopeFlags) {
        let previous_function = self.current_function.take();
        if let Some(id) = &it.id {
            self.current_function = Some(id.name.to_string());
        }

        if let Some(id) = &it.id {
            let name = id.name.to_string();
            if let Some(body) = &mut it.body {
                let mut values: FxHashMap<String, i64> = FxHashMap::default();
                let mut function = Function::default();
                let mut function_name = String::new();

                for stmt in &body.statements {
                    match stmt {
                        Statement::VariableDeclaration(var_decl) => {
                            for decl in &var_decl.declarations {
                                if let Some(init) = &decl.init {
                                    match init {
                                        Expression::NumericLiteral(lit) => {
                                            if let BindingPatternKind::BindingIdentifier(
                                                binding_ident,
                                            ) = &decl.id.kind
                                            {
                                                values.insert(
                                                    binding_ident.name.to_string(),
                                                    lit.value as i64,
                                                );
                                            }
                                        }

                                        Expression::Identifier(ident) => {
                                            if let BindingPatternKind::BindingIdentifier(
                                                binding_ident,
                                            ) = &decl.id.kind
                                            {
                                                function_name = binding_ident.name.to_string();
                                            }

                                            if let Some(proxy) =
                                                self.proxies.get(ident.name.as_str())
                                            {
                                                if let Some(func) = self.functions.get(proxy) {
                                                    function = func.clone();
                                                }
                                            } else {
                                                if let Some(func) =
                                                    self.functions.get(ident.name.as_str())
                                                {
                                                    function = func.clone();
                                                }
                                            }
                                        }

                                        _ => {}
                                    }
                                }
                            }
                        }

                        _ => {}
                    }
                }

                if !values.is_empty() {
                    self.values.insert(
                        name.to_string(),
                        Values {
                            ints: values,
                            function,
                            function_name,
                        },
                    );
                }
            }
        }

        walk_mut::walk_function(self, it, oxc_semantic::ScopeFlags::all());
        self.current_function = previous_function;
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        match it {
            Expression::CallExpression(call_expr) => {
                if let Expression::Identifier(ident) = &call_expr.callee {
                    if let Some(current_fn) = &self.current_function {
                        if let Some(values) = self.values.get(current_fn) {
                            for arg in &call_expr.arguments {
                                match arg {
                                    Argument::Identifier(ident_2) => {
                                        if let Some(value) = values.ints.get(ident_2.name.as_str())
                                        {
                                            let index = *value as usize - values.function.offset;
                                            if index < values.function.list.len() {
                                                *it = Expression::StringLiteral(self.ast.alloc(
                                                    StringLiteral {
                                                        span: SPAN,
                                                        value: self.ast.atom(
                                                            values.function.list[index].as_str(),
                                                        ),
                                                    },
                                                ));

                                                return;
                                            }
                                        }
                                    }

                                    Argument::NumericLiteral(lit) => {
                                        if ident.name == values.function_name {
                                            let index = lit.value as usize - values.function.offset;
                                            if index < values.function.list.len() {
                                                *it = Expression::StringLiteral(self.ast.alloc(
                                                    StringLiteral {
                                                        span: SPAN,
                                                        value: self.ast.atom(
                                                            values.function.list[index].as_str(),
                                                        ),
                                                    },
                                                ));

                                                return;
                                            }
                                        }
                                    }

                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            _ => {}
        }

        walk_mut::walk_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &mut VariableDeclarator<'a>) {
        if let Some(init) = &it.init {
            match init {
                Expression::Identifier(ident) => {
                    if let BindingPatternKind::BindingIdentifier(binding_ident) = &it.id.kind {
                        if ident.name.len() == 2 && binding_ident.name.len() == 2 {
                            self.proxies
                                .insert(binding_ident.name.to_string(), ident.name.to_string());
                        }
                    }
                }

                _ => {}
            }
        }

        walk_mut::walk_variable_declarator(self, it);
    }
}
