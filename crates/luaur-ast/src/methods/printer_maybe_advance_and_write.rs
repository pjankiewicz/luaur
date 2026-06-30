use crate::records::position::Position;
use crate::records::printer::Printer;

impl<'a> Printer<'a> {
    pub fn maybe_advance_and_write(&mut self, pos: &Position, s: &str, always_write: bool) {
        if pos.has_value() {
            self.advance(pos);
            self.writer.write(s);
        } else if always_write {
            self.writer.write(s);
        }
    }
}

#[export_name = "luaur_printer_maybe_advance_and_write"]
pub extern "C" fn printer_maybe_advance_and_write(
    this: *mut Printer,
    pos: *const Position,
    s_ptr: *const core::ffi::c_char,
    s_len: usize,
    always_write: bool,
) {
    unsafe {
        let s =
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(s_ptr as *const u8, s_len));
        (*this).maybe_advance_and_write(&*pos, s, always_write);
    }
}
