use crate::enums::table_state::TableState;
use crate::functions::emit_warning::emit_warning;
use crate::functions::follow_type::follow_type_id;
use crate::functions::get_type_alt_j::get_type_id;
use crate::functions::is_string::is_string;
use crate::records::lint_table_operations::LintTableOperations;
use crate::records::table_type::TableType;
use luaur_ast::records::ast_expr::AstExpr;
use luaur_config::enums::code::Code;

impl LintTableOperations {
    pub fn check_indexer(&mut self, node: *mut AstExpr, expr: *mut AstExpr, op: &str) {
        let ty = unsafe { (*self.context).get_type(expr) };
        if ty.is_none() {
            return;
        }

        let ty = ty.unwrap();
        let followed = unsafe { follow_type_id(ty) };
        let tty = unsafe { get_type_id::<TableType>(followed) };

        if tty.is_null() {
            return;
        }

        let tty_ref = unsafe { &*tty };

        if tty_ref.indexer.is_none()
            && !tty_ref.props.is_empty()
            && tty_ref.state != TableState::Generic
        {
            let msg = format!(
                "Using '{}' on a table without an array part is likely a bug",
                op
            );
            // Pass `format_args!` straight into the call: a `fmt::Arguments`
            // borrows its operands, so storing it in a `let` and using it in a
            // later statement is the shape that dangles if an operand is ever a
            // temporary (the E0716 fixed in luaur-vm's pusherror.rs, issue #3).
            emit_warning(
                unsafe { &mut *self.context },
                Code::Code_TableOperations,
                unsafe { (*node).base.location },
                format_args!("{}", msg),
            );
        } else if tty_ref.indexer.is_some()
            && is_string(tty_ref.indexer.as_ref().unwrap().index_type)
        {
            let msg = format!("Using '{}' on a table with string keys is likely a bug", op);
            // Pass `format_args!` straight into the call: a `fmt::Arguments`
            // borrows its operands, so storing it in a `let` and using it in a
            // later statement is the shape that dangles if an operand is ever a
            // temporary (the E0716 fixed in luaur-vm's pusherror.rs, issue #3).
            emit_warning(
                unsafe { &mut *self.context },
                Code::Code_TableOperations,
                unsafe { (*node).base.location },
                format_args!("{}", msg),
            );
        }
    }
}
