const SWIFT_RUNTIME: &str = "/usr/lib/swift";

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{SWIFT_RUNTIME}");
    }
    tauri_build::build();
}
