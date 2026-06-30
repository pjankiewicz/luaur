use crate::enums::ir_const_kind::IrConstKind;
use crate::records::ir_function::IrFunction;
use crate::records::ir_op::IrOp;

impl IrFunction {
    pub fn double_op(&self, op: IrOp) -> f64 {
        let value = self.const_op(op);

        assert!(value.kind == IrConstKind::Double);

        unsafe { value.value.value_double }
    }
}

#[export_name = "luaur_ir_function_double_op"]
pub extern "C" fn ir_function_double_op() {}
