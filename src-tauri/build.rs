fn main() {
    // Moonshine (sherpa-onnx) on Windows: sherpa-onnx-c-api.dll is built with the STATIC CRT
    // (/MT) — confirmed by it having no VCRUNTIME140/MSVCP140 dependency, unlike onnxruntime.dll
    // and whisper.cpp, which use the DYNAMIC CRT (/MD, shared with Rust). sherpa's import lib
    // therefore carries a `/DEFAULTLIB:libcmt` directive that conflicts (LNK4098) with the /MD
    // exe. Two CRTs linked into one process corrupt shared CRT global state and abort at startup
    // with 0x80000003. The DLL is self-contained (its CRT lives inside it), so the exe needs only
    // its import stubs — drop the stray static-CRT directive and keep the exe purely /MD.
    // NB: build scripts aren't compiled with the crate's features, so this reads the
    // CARGO_FEATURE_* env var cargo sets rather than using #[cfg(feature = ...)].
    if std::env::var_os("CARGO_FEATURE_MOONSHINE").is_some() {
        // sherpa's /MT import lib drags the STATIC CRT into the exe, giving it a second heap
        // alongside the DYNAMIC CRT (msvcrt/ucrt) that Rust, whisper.cpp and onnxruntime.dll
        // use — two heaps in one process corrupt CRT globals and abort at startup (0x80000003).
        // The heap lives in libcmt + libucrt, so exclude only those (forcing the single dynamic
        // heap). libvcruntime is left in: it holds no heap, only EH helpers like
        // __CxxFrameHandler3 that Rust's std needs, and excluding it broke the link.
        for lib in ["libcmt", "libcmtd", "libucrt", "libucrtd"] {
            println!("cargo:rustc-link-arg-bins=/NODEFAULTLIB:{lib}");
        }
    }

    tauri_build::build()
}
