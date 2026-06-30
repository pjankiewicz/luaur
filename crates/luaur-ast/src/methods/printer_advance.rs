use crate::records::position::Position;
use crate::records::printer::Printer;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

pub trait IntoPosition {
    fn into_position(self) -> Position;
}

impl IntoPosition for Position {
    fn into_position(self) -> Position {
        self
    }
}

impl IntoPosition for &Position {
    fn into_position(self) -> Position {
        *self
    }
}

impl<'a> Printer<'a> {
    pub fn advance<P: IntoPosition>(&mut self, new_pos: P) {
        let new_pos = new_pos.into_position();
        LUAU_ASSERT!(new_pos.has_value());
        self.writer.advance(&new_pos);
    }
}

#[export_name = "luaur_printer_advance"]
pub extern "C" fn printer_advance(this: *mut Printer, new_pos: *const Position) {
    unsafe {
        (*this).advance(&*new_pos);
    }
}
