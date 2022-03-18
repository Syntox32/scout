use rustpython_parser::{
    ast::{
        ExceptHandler, Expression, ExpressionType, ImportSymbol, Keyword, Operator, Parameters,
        Statement, StatementType, Suite, WithItem,
    },
    location::Location,
};

#[allow(unused_variables)]
pub trait AstVisitor {
    fn visit_return(&mut self, value: &Option<Expression>) {}
    fn visit_import(&mut self, location: &Location, names: &Vec<ImportSymbol>) {}

    fn visit_import_from(
        &mut self,
        location: &Location,
        level: &usize,
        module: &Option<String>,
        names: &Vec<ImportSymbol>,
    ) {
    }

    fn visit_call(
        &mut self,
        function: &Box<Expression>,
        args: &Vec<Expression>,
        keywords: &Vec<Keyword>,
    ) {
        self.walk_expression(function);
        self.walk_expressions(args);
        keywords
            .iter()
            .for_each(|kw| self.walk_expression(&kw.value));
    }

    fn visit_assign(&mut self, target: &Vec<Expression>, value: &Expression) {
        self.walk_expressions(target);
        self.walk_expression(value);
    }

    fn visit_function_def(
        &mut self,
        is_async: bool,
        name: &String,
        args: &Box<Parameters>,
        body: &Suite,
        decorator_list: &Vec<Expression>,
        returns: &Option<Expression>,
    ) {
        self.walk_statements(body);
        self.walk_expressions(decorator_list);
        self.walk_opt_expression(returns);
    }

    fn visit_class_def(
        &mut self,
        name: String,
        body: &Suite,
        bases: &Vec<Expression>,
        keywords: &Vec<Keyword>,
        decorator_list: &Vec<Expression>,
    ) {
        self.walk_statements(body);
        self.walk_expressions(bases);
        // TODO: handle keyword name
        keywords
            .iter()
            .for_each(|kw| self.walk_expression(&kw.value));
        self.walk_expressions(decorator_list);
    }

    // In case you want to override it
    fn visit_expression(&mut self, expr: &Expression) {}

    fn visit_break(&mut self) {}
    fn visit_continue(&mut self) {}
    fn visit_pass(&mut self) {}

    fn visit_assert(&mut self, test: &Expression, msg: &Option<Expression>) {
        self.walk_expression(test);
        self.walk_opt_expression(msg);
    }

    fn visit_delete(&mut self, targets: &Vec<Expression>) {
        self.walk_expressions(targets);
    }

    fn visit_aug_assign(&mut self, target: &Expression, op: &Operator, value: &Expression) {
        self.walk_expression(target);
        self.walk_expression(value);
    }

    fn visit_ann_assign(
        &mut self,
        target: &Box<Expression>,
        annotation: &Box<Expression>,
        value: &Option<Expression>,
    ) {
        self.walk_expression(target);
        self.walk_expression(annotation);
        self.walk_opt_expression(value);
    }

    fn visit_global(&mut self, names: &Vec<String>) {}
    fn visit_nonlocal(&mut self, names: &Vec<String>) {}

    fn visit_if(&mut self, test: &Expression, body: &Suite, orelse: &Option<Suite>) {
        self.walk_expression(test);
        self.walk_statements(body);
        self.walk_opt_statements(orelse);
    }

    fn visit_while(&mut self, test: &Expression, body: &Suite, orelse: &Option<Suite>) {
        self.walk_expression(test);
        self.walk_statements(body);
        self.walk_opt_statements(orelse);
    }

    fn visit_with(&mut self, is_async: bool, items: &Vec<WithItem>, body: &Suite) {
        items.iter().for_each(|wi| {
            self.walk_expression(&wi.context_expr);
            self.walk_opt_expression(&wi.optional_vars);
        });
        self.walk_statements(body);
    }

    fn visit_for(
        &mut self,
        is_async: bool,
        target: &Box<Expression>,
        iter: &Box<Expression>,
        body: &Suite,
        orelse: &Option<Suite>,
    ) {
        self.walk_expression(target);
        self.walk_expression(iter);
        self.walk_statements(body);
        self.walk_opt_statements(orelse);
    }

    fn visit_raise(&mut self, exception: &Option<Expression>, cause: &Option<Expression>) {
        self.walk_opt_expression(exception);
        self.walk_opt_expression(cause);
    }

    fn visit_try(
        &mut self,
        body: &Suite,
        handlers: &Vec<ExceptHandler>,
        orelse: &Option<Suite>,
        finalbody: &Option<Suite>,
    ) {
        self.walk_statements(body);

        handlers.iter().for_each(|e| {
            self.walk_opt_expression(&e.typ);
            // TODO handle name
            self.walk_statements(&e.body);
        });

        self.walk_opt_statements(orelse);
        self.walk_opt_statements(finalbody);
    }

    fn walk_opt_expression(&mut self, expr: &Option<Expression>) {
        match expr {
            Some(e) => self.walk_expression(&e),
            None => {}
        }
    }

    fn walk_opt_expressions(&mut self, exprs: &Option<Vec<Expression>>) {
        match exprs {
            Some(e) => e.iter().for_each(|e| self.walk_expression(&e)),
            None => {}
        }
    }

    fn walk_opt_statement(&mut self, stmt: &Option<Statement>) {
        match stmt {
            Some(s) => self.walk_statement(s),
            None => {}
        }
    }

    fn walk_opt_statements(&mut self, stmts: &Option<Suite>) {
        match stmts {
            Some(s) => s.iter().for_each(|stmt| self.walk_statement(&stmt)),
            None => {}
        }
    }

    fn walk_expressions(&mut self, exprs: &Vec<Expression>) {
        exprs.iter().for_each(|expr| self.walk_expression(expr));
    }

    fn walk_statements(&mut self, stmts: &Suite) {
        stmts.iter().for_each(|stmt| self.walk_statement(&stmt));
    }

    fn walk_expression(&mut self, expr: &Expression) {
        self.visit_expression(expr);
        match &expr.node {
            ExpressionType::BoolOp { op, values } => {}
            ExpressionType::Binop { a, op, b } => {}
            ExpressionType::Subscript { a, b } => {}
            ExpressionType::Unop { op, a } => {}
            ExpressionType::Await { value } => {}
            ExpressionType::Yield { value } => {}
            ExpressionType::YieldFrom { value } => {}
            ExpressionType::Compare { vals, ops } => {}
            ExpressionType::Attribute { value, name } => {}
            ExpressionType::Call {
                function,
                args,
                keywords,
            } => {
                self.visit_call(function, args, keywords);
            }
            ExpressionType::Number { value } => {}
            ExpressionType::List { elements } => {}
            ExpressionType::Tuple { elements } => {}
            ExpressionType::Dict { elements } => {}
            ExpressionType::Set { elements } => {}
            ExpressionType::Comprehension { kind, generators } => {}
            ExpressionType::Starred { value } => {}
            ExpressionType::Slice { elements } => {}
            ExpressionType::String { value } => {}
            ExpressionType::Bytes { value } => {}
            ExpressionType::Identifier { name } => {}
            ExpressionType::Lambda { args, body } => {}
            ExpressionType::IfExpression { test, body, orelse } => {}
            ExpressionType::NamedExpression { left, right } => {}
            ExpressionType::True => {}
            ExpressionType::False => {}
            ExpressionType::None => {}
            ExpressionType::Ellipsis => {}
        }
    }

    fn walk_statement(&mut self, stmt: &Statement) {
        // let location = stmt.location.to_owned();
        match &stmt.node {
            StatementType::Break => self.visit_break(),
            StatementType::Continue => self.visit_continue(),
            StatementType::Return { value } => self.visit_return(value),
            StatementType::Import { names } => self.visit_import(&stmt.location, names),
            StatementType::ImportFrom {
                level,
                module,
                names,
            } => self.visit_import_from(&stmt.location, level, module, names),
            StatementType::Pass => self.visit_pass(),
            StatementType::Assert { test, msg } => self.visit_assert(test, msg),
            StatementType::Delete { targets } => self.visit_delete(targets),
            StatementType::Assign { targets, value } => self.visit_assign(targets, value),
            StatementType::AugAssign { target, op, value } => {
                self.visit_aug_assign(target, op, value)
            }
            StatementType::AnnAssign {
                target,
                annotation,
                value,
            } => self.visit_ann_assign(target, annotation, value),
            StatementType::Expression { expression } => self.walk_expression(expression),
            StatementType::Global { names } => self.visit_global(names),
            StatementType::Nonlocal { names } => self.visit_nonlocal(names),
            StatementType::If { test, body, orelse } => self.visit_if(test, body, orelse),
            StatementType::While { test, body, orelse } => self.visit_while(test, body, orelse),
            StatementType::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => self.visit_try(body, handlers, orelse, finalbody),
            StatementType::With {
                is_async,
                items,
                body,
            } => self.visit_with(*is_async, items, body),
            StatementType::For {
                is_async,
                target,
                iter,
                body,
                orelse,
            } => self.visit_for(*is_async, target, iter, body, orelse),
            StatementType::Raise { exception, cause } => self.visit_raise(exception, cause),
            StatementType::FunctionDef {
                is_async,
                name,
                args,
                body,
                decorator_list,
                returns,
            } => self.visit_function_def(*is_async, name, args, body, decorator_list, returns),
            StatementType::ClassDef {
                name,
                body,
                bases,
                keywords,
                decorator_list,
            } => self.visit_class_def(name.clone(), body, bases, keywords, decorator_list),
        }
    }
}

pub struct AstWalker;

impl AstWalker {
    pub fn visit<T>(visitor: &mut T, statements: &Suite)
    where
        T: AstVisitor,
    {
        for statement in statements {
            visitor.walk_statement(statement);
        }
    }
}
