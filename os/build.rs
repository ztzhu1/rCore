static TARGET_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/debug/";

fn main() {
    println!("cargo:rerun-if-changed=../user/src/");
    println!("cargo:rerun-if-changed={}", TARGET_PATH);
}
