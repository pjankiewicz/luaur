//! Structured Luau generator for the `structured` fuzz target — the Rust analog
//! of Luau's `luau.proto` + `protoprint` (proto.cpp). Instead of a protobuf AST
//! mutated by libprotobuf-mutator, we walk a grammar and pull every choice from
//! libFuzzer's byte stream via [`arbitrary::Unstructured`], so the fuzzer's
//! coverage feedback *steers* the generated program toward new code paths while
//! the output stays syntactically valid Luau.

use arbitrary::Unstructured;

const NAMES: &[&str] = &["a", "b", "c", "d", "e", "f", "g", "h", "x", "y", "z"];
const SAFE_GLOBALS: &[&str] = &["type", "tostring", "tonumber", "select", "rawequal"];

pub struct Gen<'a> {
    u: Unstructured<'a>,
    out: String,
    budget: i32,
    loop_depth: u32,
}

impl<'a> Gen<'a> {
    pub fn new(data: &'a [u8]) -> Gen<'a> {
        Gen {
            u: Unstructured::new(data),
            out: String::with_capacity(256),
            budget: 200,
            loop_depth: 0,
        }
    }

    /// Generate one program. Bounded by the size budget and by the available
    /// fuzzer bytes (when they run out, `below` returns 0 → only leaves emit).
    pub fn program(mut self) -> String {
        self.block(6);
        self.out
    }

    fn below(&mut self, n: u32) -> u32 {
        if n <= 1 {
            return 0;
        }
        self.u.int_in_range(0..=n - 1).unwrap_or(0)
    }

    fn push(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn name(&mut self) {
        let i = self.below(NAMES.len() as u32) as usize;
        self.out.push_str(NAMES[i]);
    }

    fn dry(&self) -> bool {
        self.budget <= 0
    }

    fn expr(&mut self) {
        self.budget -= 1;
        if self.dry() {
            return self.atom();
        }
        match self.below(10) {
            0 | 1 => self.atom(),
            2 => self.name(),
            3 => {
                self.expr();
                let ops = [
                    " + ", " - ", " * ", " / ", " % ", " ^ ", " .. ", " == ", " ~= ", " < ",
                    " <= ", " > ", " >= ", " and ", " or ",
                ];
                let op = ops[self.below(ops.len() as u32) as usize];
                self.push(op);
                self.expr();
            }
            4 => {
                let ops = ["-", "not ", "#"];
                let op = ops[self.below(ops.len() as u32) as usize];
                self.push(op);
                self.expr();
            }
            5 => {
                self.push("(");
                self.expr();
                self.push(")");
            }
            6 => self.table(),
            7 => {
                self.name();
                if self.below(2) == 0 {
                    self.push("[");
                    self.expr();
                    self.push("]");
                } else {
                    self.push(".");
                    self.name();
                }
            }
            8 => {
                match self.below(3) {
                    0 => {
                        let g = SAFE_GLOBALS[self.below(SAFE_GLOBALS.len() as u32) as usize];
                        self.push(g);
                    }
                    1 => self.name(),
                    _ => {
                        self.name();
                        self.push(":");
                        self.name();
                    }
                }
                self.args();
            }
            _ => self.func_expr(),
        }
    }

    fn atom(&mut self) {
        match self.below(6) {
            0 => {
                let n = self.below(1000);
                self.out.push_str(&n.to_string());
            }
            1 => {
                let n = self.below(1000);
                let f = self.below(100);
                self.out.push_str(&n.to_string());
                self.push(".");
                self.out.push_str(&f.to_string());
            }
            2 => self.push("true"),
            3 => self.push("false"),
            4 => self.push("nil"),
            _ => {
                self.push("\"");
                let len = self.below(6);
                for _ in 0..len {
                    let cs = b"abcdeABCDE01234";
                    let c = cs[self.below(cs.len() as u32) as usize];
                    self.out.push(c as char);
                }
                self.push("\"");
            }
        }
    }

    fn table(&mut self) {
        self.push("{");
        let n = self.below(4);
        for i in 0..n {
            if i > 0 {
                self.push(", ");
            }
            match self.below(3) {
                0 => self.expr(),
                1 => {
                    self.name();
                    self.push(" = ");
                    self.expr();
                }
                _ => {
                    self.push("[");
                    self.expr();
                    self.push("] = ");
                    self.expr();
                }
            }
        }
        self.push("}");
    }

    fn args(&mut self) {
        self.push("(");
        let n = self.below(3);
        for i in 0..n {
            if i > 0 {
                self.push(", ");
            }
            self.expr();
        }
        self.push(")");
    }

    fn func_expr(&mut self) {
        self.push("function(");
        let n = self.below(3);
        for i in 0..n {
            if i > 0 {
                self.push(", ");
            }
            self.name();
        }
        self.push(") ");
        let saved = self.loop_depth;
        self.loop_depth = 0;
        self.block(2);
        self.loop_depth = saved;
        self.push(" end");
    }

    fn block(&mut self, max_stmts: u32) {
        let n = self.below(max_stmts + 1);
        for _ in 0..n {
            if self.dry() {
                break;
            }
            self.stmt();
            self.push("\n");
        }
        if self.below(4) == 0 {
            self.push("return");
            if self.below(2) == 0 {
                self.push(" ");
                self.expr();
            }
            self.push("\n");
        }
    }

    fn stmt(&mut self) {
        self.budget -= 1;
        if self.dry() {
            self.push("local ");
            self.name();
            self.push(" = ");
            return self.atom();
        }
        match self.below(12) {
            0 => {
                self.push("local ");
                self.name();
                self.push(" = ");
                self.expr();
            }
            1 => {
                self.name();
                let ops = [" = ", " += ", " -= ", " *= ", " /= ", " %= ", " ..= "];
                let op = ops[self.below(ops.len() as u32) as usize];
                self.push(op);
                self.expr();
            }
            2 => {
                self.push("if ");
                self.expr();
                self.push(" then\n");
                self.block(2);
                if self.below(2) == 0 {
                    self.push("else\n");
                    self.block(2);
                }
                self.push("end");
            }
            3 => {
                self.push("while ");
                self.expr();
                self.push(" do\n");
                self.loop_depth += 1;
                self.block(2);
                self.loop_depth -= 1;
                self.push("end");
            }
            4 => {
                self.push("for ");
                self.name();
                self.push(" = ");
                self.expr();
                self.push(", ");
                self.expr();
                if self.below(2) == 0 {
                    self.push(", ");
                    self.expr();
                }
                self.push(" do\n");
                self.loop_depth += 1;
                self.block(2);
                self.loop_depth -= 1;
                self.push("end");
            }
            5 => {
                self.push("repeat\n");
                self.loop_depth += 1;
                self.block(2);
                self.loop_depth -= 1;
                self.push("until ");
                self.expr();
            }
            6 => {
                self.push("do\n");
                self.block(2);
                self.push("end");
            }
            7 => {
                self.push("local function ");
                self.name();
                self.func_tail();
            }
            8 => {
                self.push("function ");
                self.name();
                self.func_tail();
            }
            9 => {
                match self.below(2) {
                    0 => {
                        let g = SAFE_GLOBALS[self.below(SAFE_GLOBALS.len() as u32) as usize];
                        self.push(g);
                    }
                    _ => self.name(),
                }
                self.args();
            }
            10 if self.loop_depth > 0 => self.push("break"),
            11 if self.loop_depth > 0 => self.push("continue"),
            _ => {
                self.name();
                self.push(" = ");
                self.expr();
            }
        }
    }

    fn func_tail(&mut self) {
        self.push("(");
        let n = self.below(3);
        for i in 0..n {
            if i > 0 {
                self.push(", ");
            }
            self.name();
        }
        self.push(")\n");
        let saved = self.loop_depth;
        self.loop_depth = 0;
        self.block(3);
        self.loop_depth = saved;
        self.push("end");
    }
}

/// Generate a syntactically-valid Luau program from raw fuzzer bytes.
pub fn generate(data: &[u8]) -> String {
    Gen::new(data).program()
}
