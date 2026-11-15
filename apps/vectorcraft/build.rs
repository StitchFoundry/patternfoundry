//! Windows only: embed the app icon and version info (VERSIONINFO) into `patternfoundry.exe`.
//!
//! On every other target this does nothing. A missing resource compiler is a warning, so a
//! cross-compile from macOS or Linux still links, unless `VECTORCRAFT_REQUIRE_WINRES=1` turns it into
//! an error (for release builds).

fn main() -> Result<(), String> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/app-icon/patternfoundry.ico");
    println!("cargo:rerun-if-env-changed=VECTORCRAFT_REQUIRE_WINRES");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/app-icon/patternfoundry.ico")
        .set("ProductName", "PatternFoundry")
        .set("FileDescription", "PatternFoundry sewing-pattern design software")
        .set("CompanyName", "StitchFoundry")
        .set("LegalCopyright", "Copyright (c) Bruce Wattendorf (SiteSentrix) and the PatternFoundry authors. MIT OR Apache-2.0.")
        .set("OriginalFilename", "patternfoundry.exe")
        .set("InternalName", "patternfoundry");
    if let Err(e) = res.compile() {
        if std::env::var_os("VECTORCRAFT_REQUIRE_WINRES").is_some() {
            return Err(format!("embedding Windows resources failed: {e}"));
        }
        println!("cargo:warning=patternfoundry.exe built without icon/version resources: {e}");
    }
    Ok(())
}
