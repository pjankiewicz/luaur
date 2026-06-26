#[cfg(test)]
#[test]
fn compiler_loop_continue_respects_explicit_constant() {
    use alloc::string::String;
    use luaur_bytecode::records::bytecode_builder::BytecodeBuilder;
    use luaur_compiler::functions::compile_or_throw_compiler_alt_b::compile_or_throw_bytecode_builder_string_compile_options_parse_options;
    use luaur_compiler::records::compile_error::CompileError;

    let mut bcb = BytecodeBuilder::new(None);

    let source = String::from("\nrepeat\n    do continue end\n\n    local c = true\nuntil c\n");
    let options = luaur_compiler::records::compile_options::CompileOptions::default();
    let parse_options = luaur_ast::records::parse_options::ParseOptions::default();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        compile_or_throw_bytecode_builder_string_compile_options_parse_options(
            &mut bcb,
            &source,
            &options,
            &parse_options,
        );
    }));

    assert!(result.is_err(), "Expected CompileError");

    let err = result.unwrap_err();
    let err_str = err
        .downcast_ref::<CompileError>()
        .expect("panic payload is not a CompileError");

    let loc = err_str.get_location();
    assert_eq!(loc.begin.line + 1, 6);

    let msg = unsafe {
        core::ffi::CStr::from_ptr(err_str.what())
            .to_string_lossy()
            .to_string()
    };
    let expected_msg =
        "Local c used in the repeat..until condition is undefined because continue statement on line 3 jumps over it";
    assert_eq!(msg, expected_msg);

    // Deterministic guard for issue #3's follow-up bug: a Rust `String` is not
    // NUL-terminated, so `what()` must hand out a terminated buffer or
    // `CStr::from_ptr` over-reads past the message into adjacent memory (which
    // failed flakily on Windows). The byte at `message.len()` must be the NUL.
    unsafe {
        assert_eq!(
            *err_str.what().add(expected_msg.len()),
            0,
            "CompileError::what() must be NUL-terminated"
        );
    }
}
