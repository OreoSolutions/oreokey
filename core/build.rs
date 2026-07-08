fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("apple-darwin") {
        // AX API (HIServices) + IsSecureEventInputEnabled (HIToolbox).
        println!("cargo:rustc-link-lib=framework=ApplicationServices");
        println!("cargo:rustc-link-lib=framework=Carbon");
    }
}
