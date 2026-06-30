use crate::enums::ir_const_kind::IrConstKind;
use crate::enums::ir_op_kind::IrOpKind;
use crate::records::ir_function::IrFunction;
use crate::records::ir_op::IrOp;

impl IrFunction {
    pub fn as_double_op(&mut self, op: IrOp) -> Option<f64> {
        if op.kind() != IrOpKind::Constant {
            return None;
        }

        let value = self.const_op(op);

        if value.kind != IrConstKind::Double {
            return None;
        }

        unsafe { Some(value.value.value_double) }
    }
}

#[export_name = "luaur_ir_function_as_double_op"]
pub extern "C" fn ir_function_as_double_op() {}

impl IrFunction {
    pub fn const_op(&self, op: IrOp) -> crate::records::ir_const::IrConst {
        self.constants[op.index() as usize]
    }
}
