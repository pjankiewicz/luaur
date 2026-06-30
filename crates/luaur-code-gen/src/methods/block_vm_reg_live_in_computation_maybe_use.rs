use crate::enums::ir_op_kind::IrOpKind;
use crate::functions::vm_reg_op::vm_reg_op;
use crate::records::block_vm_reg_live_in_computation::BlockVmRegLiveInComputation;
use crate::records::ir_op::IrOp;

#[export_name = "luaur_block_vm_reg_live_in_computation_maybe_use"]
pub extern "C" fn block_vm_reg_live_in_computation_maybe_use(
    this: &mut BlockVmRegLiveInComputation<'_>,
    op: IrOp,
) {
    if op.kind() == IrOpKind::VmReg {
        let reg_index = vm_reg_op(op) as usize;
        let reg_bit = 1u64 << (reg_index % 64);
        let reg_array_index = reg_index / 64;

        if reg_array_index < 4 {
            if (this.def_rs.regs[reg_array_index] & reg_bit) == 0 {
                this.in_rs.regs[reg_array_index] |= reg_bit;
            }
        }
    }
}
