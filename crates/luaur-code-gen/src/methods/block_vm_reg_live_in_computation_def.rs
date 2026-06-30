use crate::records::block_vm_reg_live_in_computation::BlockVmRegLiveInComputation;
use crate::records::ir_op::IrOp;

#[export_name = "luaur_block_vm_reg_live_in_computation_def"]
pub extern "C" fn block_vm_reg_live_in_computation_def(
    this: &mut BlockVmRegLiveInComputation<'_>,
    op: IrOp,
    offset: i32,
) {
    this.def(op, offset);
}
