use crate::records::block_vm_reg_live_in_computation::BlockVmRegLiveInComputation;

#[export_name = "luaur_block_vm_reg_live_in_computation_capture"]
pub extern "C" fn block_vm_reg_live_in_computation_capture(
    this: &mut BlockVmRegLiveInComputation,
    reg: i32,
) {
    this.capture(reg);
}
