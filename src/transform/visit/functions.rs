use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::{visit::walk_mut, AstBuilder, VisitMut};

use rustc_hash::FxHashMap;

#[derive(Debug, Default, Clone)]
pub struct Function {
    pub offset: usize,
    pub list: Vec<String>,
}

#[derive(Debug)]
pub struct PendingFunction {
    pub name: String,
    pub offset: usize,
}

#[derive(Debug)]
pub struct Values {
    pub objects: FxHashMap<String, FxHashMap<String, i64>>,
    pub ints: FxHashMap<String, i64>,

    pub function: Function,
    pub function_name: String,
}

pub struct FunctionVisitor<'a> {
    ast: AstBuilder<'a>,

    pub functions: FxHashMap<String, Function>,
    pending_functions: FxHashMap<String, PendingFunction>,
    anon_function_counter: usize,
    lists: FxHashMap<String, Vec<String>>,
}

impl<'a> FunctionVisitor<'a> {
    pub fn new(allocator: &'a Allocator) -> Self {
        Self {
            ast: AstBuilder::new(allocator),
            functions: FxHashMap::default(),
            pending_functions: FxHashMap::default(),
            anon_function_counter: 0,
            lists: FxHashMap::default(),
        }
    }
}

impl<'a> VisitMut<'a> for FunctionVisitor<'a> {
    fn visit_function(&mut self, it: &mut oxc_ast::ast::Function<'a>, _: oxc_semantic::ScopeFlags) {
        let name = if let Some(id) = &it.id {
            id.name.to_string()
        } else {
            // Anonymous function handling
            let anon_name = format!("anon_function_{}", self.anon_function_counter);
            self.anon_function_counter += 1;
            anon_name
        };

        if let Some(body) = &mut it.body {
            let mut func_name = "";

            let mut indices_to_remove = Vec::new();
            let mut i = 0;
            for stmt in &body.statements {
                match stmt {
                    Statement::VariableDeclaration(var_decl) => {
                        for decl in &var_decl.declarations {
                            if let Some(init) = &decl.init {
                                match init {
                                    Expression::ArrayExpression(array) => {
                                        let mut list = Vec::new();
                                        for elem in &array.elements {
                                            if let ArrayExpressionElement::StringLiteral(lit) = elem
                                            {
                                                list.push(lit.value.to_string());
                                            }
                                        }

                                        if list.len() > 0 {
                                            self.lists.insert(name.to_string(), list.clone());
                                        }

                                        if let Some(func) =
                                            self.pending_functions.get(name.as_str())
                                        {
                                            self.functions.insert(
                                                func.name.to_string(),
                                                Function {
                                                    offset: func.offset,
                                                    list,
                                                },
                                            );
                                        }

                                        indices_to_remove.push(i);
                                    }

                                    Expression::CallExpression(call) => {
                                        if let Expression::Identifier(ident) = &call.callee {
                                            func_name = ident.name.as_str();
                                        }
                                    }

                                    _ => {}
                                }
                            }
                        }
                    }

                    Statement::ReturnStatement(retrn) => {
                        if let Some(arg) = &retrn.argument {
                            if let Expression::CallExpression(call) = arg {
                                if let Expression::ParenthesizedExpression(parenth) = &call.callee {
                                    if let Expression::AssignmentExpression(assign) =
                                        &parenth.expression
                                    {
                                        if let Expression::FunctionExpression(func) = &assign.right
                                        {
                                            if let Some(body) = &mut &func.body {
                                                for stmt in &body.statements {
                                                    if let Statement::ReturnStatement(retrn) = stmt
                                                    {
                                                        if let Some(arg) = &retrn.argument {
                                                            if let Expression::ComputedMemberExpression(computed) = arg {
                                                                    if let Expression::AssignmentExpression(assign) =
                                                                        &computed.expression
                                                                    {
                                                                        if let Expression::NumericLiteral(lit) = &assign.right {
                                                                            if let Some(list) = self.lists.get(func_name) {
                                                                                self.functions.insert(name.to_string(), Function{
                                                                                    offset: lit.value as usize,
                                                                                    list: list.to_vec(),
                                                                                });
                                                                            } else {
                                                                                self.pending_functions.insert(func_name.to_string(), PendingFunction{
                                                                                    name: name.to_string(),
                                                                                    offset: lit.value as usize,
                                                                                });
                                                                            }

                                                                            indices_to_remove.push(i);
                                                                        }
                                                                    }
                                                                }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    _ => {}
                }

                i += 1;
            }

            for index in indices_to_remove.iter().rev() {
                body.statements.remove(*index);
            }
        }

        walk_mut::walk_function(self, it, oxc_semantic::ScopeFlags::all());
    }
}
