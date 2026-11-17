//! Build script: compiles the Slint interface into Rust.

fn main() {
    slint_build::compile("../../ui/main.slint").expect("failed to compile ui/main.slint");
}
