fn main() {
    // On Windows/GNU the cdylib crate-type causes a PE export-table overflow
    // (>65535 symbols) when soroban-sdk testutils are linked in.
    // --exclude-all-symbols tells the linker not to populate the DLL export
    // table, which avoids the overflow.  This flag is only meaningful on
    // Windows/GNU and is silently ignored elsewhere.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu")
    {
        println!("cargo:rustc-link-arg=-Wl,--exclude-all-symbols");
    }
}
