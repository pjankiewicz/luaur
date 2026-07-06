fn main() {
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    if target_env == "msvc" {
        // `cargo install` does not include the workspace's `.cargo/config.toml`.
        // These libraries provide exported MSVC/UCRT stdio symbols used by the
        // VM's C-ABI debug/profiling helpers.
        println!("cargo:rustc-link-lib=legacy_stdio_definitions");
        println!("cargo:rustc-link-lib=ucrt");
    }
}
