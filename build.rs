// sdp-rs is a pure-Rust text-protocol crate with no native linkage.
// This build script is intentionally a no-op; it exists only so the
// crate matches the workspace layout convention.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
