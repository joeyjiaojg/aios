// Build script: compile boot.S and set linker script
use std::process::Command;
use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Compile the boot assembly
    let boot_s = manifest_dir.join("src/kernel/boot.S");
    let boot_obj = out_dir.join("boot.o");

    let status = Command::new("gcc")
        .args([
            "-m64",
            "-c",
            "-nostdlib",
            "-nostdinc",
            "-ffreestanding",
            "-o",
        ])
        .arg(&boot_obj)
        .arg(&boot_s)
        .status()
        .expect("failed to run gcc for boot.S");

    assert!(status.success(), "boot.S assembly failed");

    // Tell cargo to link the boot object
    println!("cargo:rustc-link-arg={}", boot_obj.display());

    // Use our linker script
    let ld_script = manifest_dir.join("linker.ld");
    println!("cargo:rustc-link-arg=-T{}", ld_script.display());
    println!("cargo:rustc-link-arg=-n");

    // Re-run if these files change
    println!("cargo:rerun-if-changed=src/kernel/boot.S");
    println!("cargo:rerun-if-changed=linker.ld");
}
