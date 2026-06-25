use crate::enums::dump_flags::DumpFlags;
use crate::functions::get_base_type_string::get_base_type_string;
use crate::records::bytecode_builder::BytecodeBuilder;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::CStr;
use luaur_common::enums::luau_bytecode_type::{LuauBytecodeType, LBC_TYPE_OPTIONAL_BIT};
use luaur_common::enums::luau_opcode::LuauOpcode;
use luaur_common::functions::format_append::formatAppend;
use luaur_common::functions::get_jump_target::get_jump_target;
use luaur_common::functions::get_op_length::get_op_length;
use luaur_common::macros::luau_assert::LUAU_ASSERT;
use luaur_common::macros::luau_insn_op::LUAU_INSN_OP;

impl BytecodeBuilder {
    pub fn dump_current_function(&self, dumpinstoffs: &mut Vec<i32>) -> String {
        if (self.dump_flags & (DumpFlags::Dump_Code as u32 | DumpFlags::Dump_Constants as u32)) == 0
        {
            return String::new();
        }

        let mut last_line = -1;
        let mut next_remark = 0;
        let mut result = String::new();

        if self.dump_flags & DumpFlags::Dump_Locals as u32 != 0 {
            for i in 0..self.debug_locals.len() {
                let l = &self.debug_locals[i as usize];

                if l.startpc == l.endpc {
                    LUAU_ASSERT!(l.startpc < self.lines.len() as u32);

                    formatAppend(
                        &mut result,
                        format_args!(
                            "local {}: reg {}, start pc {} line {}, no live range\n",
                            i, l.reg, l.startpc, self.lines[l.startpc as usize]
                        ),
                    );
                } else {
                    LUAU_ASSERT!(l.startpc < l.endpc);
                    LUAU_ASSERT!(l.startpc < self.lines.len() as u32);
                    LUAU_ASSERT!(l.endpc <= self.lines.len() as u32);

                    formatAppend(
                        &mut result,
                        format_args!(
                            "local {}: reg {}, start pc {} line {}, end pc {} line {}\n",
                            i,
                            l.reg,
                            l.startpc,
                            self.lines[l.startpc as usize],
                            l.endpc - 1,
                            self.lines[(l.endpc - 1) as usize]
                        ),
                    );
                }
            }
        }

        if self.dump_flags & DumpFlags::Dump_Types as u32 != 0 {
            let typeinfo = &self.functions.last().unwrap().typeinfo;
            let typeinfo_bytes = typeinfo.as_bytes();

            for i in 2..typeinfo_bytes.len() {
                let et = typeinfo_bytes[i];

                // C++ `name = userdata ? userdata : getBaseTypeString(et)`.
                let name = match self.try_get_userdata_type_name(LuauBytecodeType(et as u16)) {
                    Some(s) => alloc::borrow::Cow::Borrowed(s),
                    None => unsafe { CStr::from_ptr(get_base_type_string(et)).to_string_lossy() },
                };
                let optional = if (et as u16 & LBC_TYPE_OPTIONAL_BIT.0) != 0 {
                    "?"
                } else {
                    ""
                };

                formatAppend(
                    &mut result,
                    format_args!("R{}: {}{} [argument]\n", i - 2, name, optional),
                );
            }

            for i in 0..self.typed_upvals.len() {
                let l = &self.typed_upvals[i];

                let name = match self.try_get_userdata_type_name(l.r#type) {
                    Some(s) => alloc::borrow::Cow::Borrowed(s),
                    None => unsafe {
                        CStr::from_ptr(get_base_type_string(l.r#type.0 as u8)).to_string_lossy()
                    },
                };
                let optional = if (l.r#type.0 & LBC_TYPE_OPTIONAL_BIT.0) != 0 {
                    "?"
                } else {
                    ""
                };

                formatAppend(&mut result, format_args!("U{}: {}{}\n", i, name, optional));
            }

            for i in 0..self.typed_locals.len() {
                let l = &self.typed_locals[i];

                let name = match self.try_get_userdata_type_name(l.r#type) {
                    Some(s) => alloc::borrow::Cow::Borrowed(s),
                    None => unsafe {
                        CStr::from_ptr(get_base_type_string(l.r#type.0 as u8)).to_string_lossy()
                    },
                };
                let optional = if (l.r#type.0 & LBC_TYPE_OPTIONAL_BIT.0) != 0 {
                    "?"
                } else {
                    ""
                };

                formatAppend(
                    &mut result,
                    format_args!(
                        "R{}: {}{} from {} to {}\n",
                        l.reg, name, optional, l.startpc, l.endpc
                    ),
                );
            }
        }

        if self.dump_flags & DumpFlags::Dump_Constants as u32 != 0 {
            for i in 0..self.constants.len() {
                formatAppend(&mut result, format_args!("K{}: ", i));
                self.dump_constant(&mut result, i as i32, true);
                formatAppend(&mut result, format_args!("\n"));
            }
        }

        if self.dump_flags & DumpFlags::Dump_Code as u32 != 0 {
            let mut labels = vec![-1; self.insns.len()];

            let mut i = 0;
            while i < self.insns.len() {
                let target = get_jump_target(self.insns[i], i as u32);

                if target >= 0 {
                    LUAU_ASSERT!((target as usize) < self.insns.len());
                    labels[target as usize] = 0;
                }

                let op: LuauOpcode =
                    unsafe { core::mem::transmute(LUAU_INSN_OP(self.insns[i]) as u8) };
                let op_len = get_op_length(op) as usize;
                i += op_len;
                LUAU_ASSERT!(i <= self.insns.len());
            }

            let mut next_label = 0;

            for i in 0..labels.len() {
                if labels[i] == 0 {
                    labels[i] = next_label;
                    next_label += 1;
                }
            }

            dumpinstoffs.resize(self.insns.len() + 1, -1);

            let mut i = 0;
            while i < self.insns.len() {
                let code = &self.insns[i];
                let op = LUAU_INSN_OP(*code) as u8;

                dumpinstoffs[i] = result.len() as i32;

                // C++: `if (op == LOP_PREPVARARGS) { i++; continue; }` — the vararg
                // prologue is a call-dispatch header with no "interesting" info and
                // is never disassembled. (The prior `op == 32` was a mistranslated
                // literal; LOP_PREPVARARGS is 65, so the skip never fired and the
                // header reached `dump_instruction`'s unsupported-opcode assert.)
                if op == LuauOpcode::LOP_PREPVARARGS as u8 {
                    i += 1;
                    continue;
                }

                if self.dump_flags & DumpFlags::Dump_Remarks as u32 != 0 {
                    while next_remark < self.debug_remarks.len()
                        && self.debug_remarks[next_remark].0 == i as u32
                    {
                        let remark_start = self.debug_remarks[next_remark].1 as usize;
                        let remark_end = if next_remark + 1 < self.debug_remarks.len() {
                            self.debug_remarks[next_remark + 1].1 as usize
                        } else {
                            self.debug_remark_buffer.len()
                        };
                        // C++ reads `debugRemarkBuffer.c_str() + offset` — a C-string that stops
                        // at the null terminator. remark_end points at the *next* remark (past this
                        // remark's '\0'), so slice only up to the terminator.
                        let remark_str = self.debug_remark_buffer[remark_start..remark_end]
                            .split('\0')
                            .next()
                            .unwrap_or("");
                        formatAppend(&mut result, format_args!("REMARK {}\n", remark_str));
                        next_remark += 1;
                    }
                }

                if self.dump_flags & DumpFlags::Dump_Source as u32 != 0 {
                    let line = self.lines[i];

                    if line > 0 && line != last_line {
                        LUAU_ASSERT!(((line - 1) as usize) < self.dump_source.len());
                        formatAppend(
                            &mut result,
                            format_args!("{:5}: {}\n", line, self.dump_source[(line - 1) as usize]),
                        );
                        last_line = line;
                    }
                }

                if self.dump_flags & DumpFlags::Dump_Lines as u32 != 0 {
                    formatAppend(&mut result, format_args!("{}: ", self.lines[i]));
                }

                if labels[i] != -1 {
                    formatAppend(&mut result, format_args!("L{}: ", labels[i]));
                }

                let target = get_jump_target(*code, i as u32);
                let target_label = if target >= 0 {
                    labels[target as usize]
                } else {
                    -1
                };

                // Pass the full remaining instruction stream (not just one word):
                // multi-word ops (LOADKX, GETIMPORT, FASTCALL2K, NEWCLASSMEMBER,
                // CMPPROTO, …) read their aux word via `code[1]`, which would be
                // out of bounds on a length-1 slice. C++ passes a bare pointer.
                self.dump_instruction(&self.insns[i..], &mut result, target_label);

                let op: LuauOpcode = unsafe { core::mem::transmute(op) };
                let op_len = get_op_length(op) as usize;
                i += op_len;
                LUAU_ASSERT!(i <= self.insns.len());
            }

            dumpinstoffs[self.insns.len()] = result.len() as i32;
        }

        result
    }
}
