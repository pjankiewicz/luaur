use crate::records::ast_stat_block::AstStatBlock;
use crate::records::printer::Printer;

pub trait IntoAstStatBlockMut {
    unsafe fn into_ast_stat_block_mut(self) -> *mut AstStatBlock;
}

impl IntoAstStatBlockMut for *mut AstStatBlock {
    unsafe fn into_ast_stat_block_mut(self) -> *mut AstStatBlock {
        self
    }
}

impl IntoAstStatBlockMut for &mut AstStatBlock {
    unsafe fn into_ast_stat_block_mut(self) -> *mut AstStatBlock {
        self
    }
}

impl<'a> Printer<'a> {
    pub fn visualize_block_ast_stat_block<B: IntoAstStatBlockMut>(&mut self, block: B) {
        let block = unsafe { &mut *block.into_ast_stat_block_mut() };
        for i in 0..block.body.size {
            let stat = unsafe { *block.body.data.add(i) };
            self.visualize_ast_stat(unsafe { &mut *stat });
        }
        self.advance(&block.base.base.location.end);
    }
}

#[export_name = "luaur_printer_visualize_block_ast_stat_block"]
pub extern "C" fn printer_visualize_block_ast_stat_block(
    this: *mut Printer,
    block: *mut AstStatBlock,
) {
    unsafe {
        (*this).visualize_block_ast_stat_block(&mut *block);
    }
}
