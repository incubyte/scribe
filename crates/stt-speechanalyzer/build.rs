fn main() {
    #[cfg(target_os = "macos")]
    link_swift_bridge();
}

#[cfg(target_os = "macos")]
fn link_swift_bridge() {
    const MACOS_MINIMUM: &str = "26.0";
    const SWIFT_RUNTIME: &str = "/usr/lib/swift";
    swift_rs::SwiftLinker::new(MACOS_MINIMUM)
        .with_package("scribe-speech", "./swift-lib/")
        .link();
    println!("cargo:rustc-link-arg=-Wl,-rpath,{SWIFT_RUNTIME}");
    println!("cargo:rerun-if-changed=swift-lib/Sources");
    println!("cargo:rerun-if-changed=swift-lib/Package.swift");
}
