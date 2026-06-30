use crate::functions::get_fmov_imm_fp_64::get_fmov_imm_fp_64;
use crate::records::assembly_builder_a_64::AssemblyBuilderA64;

impl AssemblyBuilderA64 {
    pub fn is_fmov_supported_fp_64(&mut self, value: f64) -> bool {
        get_fmov_imm_fp_64(value) >= 0
    }
}

#[export_name = "luaur_assembly_builder_a_64_is_fmov_supported_fp_64"]
pub extern "C" fn assembly_builder_a_64_is_fmov_supported_fp_64() {}
