fn main() {
    println!("cargo:rerun-if-env-changed=HARNESSDECK_EDITION");
    let edition = std::env::var("HARNESSDECK_EDITION").unwrap_or_else(|_| "lite".into());
    println!("cargo:rustc-env=HARNESSDECK_EDITION={edition}");
    tauri_build::build();
}
