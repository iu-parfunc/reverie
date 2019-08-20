use std::io::{Result};

use cc;

fn main() -> Result<()> {
    cc::Build::new()
        .flag("-D_GNU_SOURCE=1")
        .file("src/bpf_ll.c")
        .file("src/bpf-helper.c")
        .file("src/dl_ns.c")
        .compile("my-asm-lib");
    Ok(())
}
