fn main() {
    // The sherpa-onnx / ONNX Runtime SHARED libraries ship beside the executable on Windows, and
    // inside the app bundle on macOS (Contents/Frameworks) / Linux. sherpa-rs-sys deliberately does
    // NOT add a desktop rpath to the final binary (see its build.rs, ~line 575 — only mobile gets
    // one), so without the link args below the dynamic loader cannot find those libs at runtime off
    // Windows and EVERY model load fails silently. Point the loader at the locations we bundle them.
    //
    // These are no-ops on Windows (DLLs resolve from the exe's own directory), so the proven Windows
    // build is unaffected.
    #[cfg(target_os = "macos")]
    {
        // The binary sits in Contents/MacOS; the dylibs are embedded in Contents/Frameworks by CI.
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
    }
    #[cfg(target_os = "linux")]
    {
        // $ORIGIN = the directory of the running binary; resolve libs placed next to it or one dir up.
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib/AuraScribe");
        println!("cargo:rustc-link-arg=-Wl,-z,origin");
    }
    tauri_build::build()
}
