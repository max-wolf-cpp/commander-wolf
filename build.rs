use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Linker arguments from a dependency's build script do not reach this package.
    match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("linux") => println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN"),
        Ok("macos") => {
            println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
            println!("cargo:rustc-link-arg=-Wl,-headerpad_max_install_names");
        }
        _ => {}
    }
}
