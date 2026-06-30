use crate::records::location::Location;
use crate::records::position::Position;
use crate::records::printer::Printer;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

impl<'a> Printer<'a> {
    pub fn write_end(&mut self, loc: &Location) {
        let mut end_pos = loc.end;
        if end_pos.column >= 3 {
            end_pos.column -= 3;
        }
        self.advance(&end_pos);
        self.writer.keyword("end");
    }
}

#[export_name = "luaur_printer_write_end"]
pub extern "C" fn printer_write_end(this: *mut Printer, loc: *const Location) {
    unsafe {
        (*this).write_end(&*loc);
    }
}
