use crate::enums::ir_const_kind::IrConstKind;
use crate::records::ir_const::IrConst;
use crate::records::ir_function::IrFunction;
use crate::records::ir_op::IrOp;

macro_rules! CODEGEN_ASSERT {
    ($expr:expr) => {
        assert!($expr);
    };
}

impl IrFunction {
    pub fn tag_op(&self, op: IrOp) -> u8 {
        let value: IrConst = self.const_op(op);

        CODEGEN_ASSERT!(value.kind == IrConstKind::Tag);

        unsafe { value.value.value_tag }
    }
}

#[export_name = "luaur_ir_function_tag_op"]
pub extern "C" fn ir_function_tag_op() {}
