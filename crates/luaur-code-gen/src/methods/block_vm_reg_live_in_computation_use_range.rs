use crate::records::block_vm_reg_live_in_computation::BlockVmRegLiveInComputation;

#[export_name = "luaur_block_vm_reg_live_in_computation_use_range"]
pub extern "C" fn block_vm_reg_live_in_computation_use_range(
    this: &mut BlockVmRegLiveInComputation<'_>,
    start: i32,
    count: i32,
) {
    if count == -1 {
        this.block_vm_reg_live_in_computation_use_varargs(start as u8);
    } else {
        for i in start..(start + count) {
            let idx = i as usize;
            if (this.def_rs.regs[idx / 64] & (1u64 << (idx % 64))) == 0 {
                this.in_rs.regs[idx / 64] |= 1u64 << (idx % 64);
            }
        }
    }
}
