//! Embed the vendored Luau conformance scripts into the binary so the AST
//! splicer (`luaur_fuzz::generate_spliced`) can recombine real, hand-written
//! programs without needing the corpus on disk at runtime. Generates
//! `$OUT_DIR/splice_corpus.rs` with `pub static SPLICE_CORPUS: &[&str]`.

use std::{env, fs, path::Path};

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let corpus_dir = Path::new(&manifest).join("../crates/luaur-conformance/conformance");
    let out_dir = env::var("OUT_DIR").unwrap();

    let mut files: Vec<_> = fs::read_dir(&corpus_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("luau"))
        // A few conformance scripts embed intentionally non-UTF-8 bytes (lexer
        // tests). `include_str!` rejects those, so skip them — the splicer only
        // needs valid-UTF-8 program text.
        .filter(|p| {
            fs::read(p)
                .map(|b| String::from_utf8(b).is_ok())
                .unwrap_or(false)
        })
        .collect();
    files.sort();

    let mut code = String::from("pub static SPLICE_CORPUS: &[&str] = &[\n");
    for p in &files {
        // `include_str!` requires valid UTF-8; conformance scripts are UTF-8.
        code.push_str(&format!("    include_str!(r\"{}\"),\n", p.display()));
        println!("cargo:rerun-if-changed={}", p.display());
    }
    code.push_str("];\n");

    fs::write(Path::new(&out_dir).join("splice_corpus.rs"), code).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", corpus_dir.display());
}
