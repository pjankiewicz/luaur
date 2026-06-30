use crate::records::block_vm_reg_live_in_computation::BlockVmRegLiveInComputation;

#[export_name = "luaur_block_vm_reg_live_in_computation_def_range"]
pub extern "C" fn block_vm_reg_live_in_computation_def_range(
    this: &mut BlockVmRegLiveInComputation<'_>,
    start: i32,
    count: i32,
) {
    this.def_range(start, count);
}
