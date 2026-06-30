use crate::enums::ir_const_kind::IrConstKind;
use crate::records::ir_function::IrFunction;
use crate::records::ir_op::IrOp;

impl IrFunction {
    pub fn int_op(&self, op: IrOp) -> i32 {
        let value = self.const_op(op);

        assert!(value.kind == IrConstKind::Int);

        unsafe { value.value.value_int }
    }
}

#[export_name = "luaur_ir_function_int_op"]
pub extern "C" fn ir_function_int_op() {}
