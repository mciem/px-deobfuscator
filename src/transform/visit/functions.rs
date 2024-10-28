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
    pub ints: FxHashMap<String, i64>,
    pub function: Function,
    pub function_name: String,
}

pub struct FunctionVisitor<'a> {
    ast: AstBuilder<'a>,

    pub functions: FxHashMap<String, Function>,
    pending_functions: FxHashMap<String, PendingFunction>,
    lists: FxHashMap<String, Vec<String>>,
}

impl<'a> FunctionVisitor<'a> {
    pub fn new(allocator: &'a Allocator) -> Self {
        Self {
            ast: AstBuilder::new(allocator),
            functions: FxHashMap::default(),
            pending_functions: FxHashMap::default(),
            lists: FxHashMap::default(),
        }
    }
}

impl<'a> VisitMut<'a> for FunctionVisitor<'a> {
    fn visit_function(&mut self, it: &mut oxc_ast::ast::Function<'a>, _: oxc_semantic::ScopeFlags) {
        if let Some(id) = &it.id {
            let name = id.name.to_string();
            if let Some(body) = &mut it.body {
                let mut func_name = "";

                for stmt in &body.statements {
                    match stmt {
                        Statement::VariableDeclaration(var_decl) => {
                            for decl in &var_decl.declarations {
                                if let Some(init) = &decl.init {
                                    match init {
                                        Expression::ArrayExpression(array) => {
                                            let mut list = Vec::new();
                                            for elem in &array.elements {
                                                if let ArrayExpressionElement::StringLiteral(lit) =
                                                    elem
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
                                    if let Expression::ParenthesizedExpression(parenth) =
                                        &call.callee
                                    {
                                        if let Expression::AssignmentExpression(assign) =
                                            &parenth.expression
                                        {
                                            if let Expression::FunctionExpression(func) =
                                                &assign.right
                                            {
                                                if let Some(body) = &mut &func.body {
                                                    for stmt in &body.statements {
                                                        if let Statement::ReturnStatement(retrn) =
                                                            stmt
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
                }

                /*
                for index in indices_to_remove.iter().rev() {
                    body.statements.remove(*index);
                }
                */
            }
        }

        walk_mut::walk_function(self, it, oxc_semantic::ScopeFlags::all());
    }
}
