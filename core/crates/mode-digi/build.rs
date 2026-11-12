//! Con la feature `fldigi-oracle` (solo per le prove, mai nell'installatore) compila i ricevitori RTTY, PSK e CW di fldigi presi da
//! `terze-parti/fldigi` (sottomodulo) piu' i nostri `oracle/fldigi_rtty_rx.cpp`, `fldigi_psk_rx.cpp` e `fldigi_cw_rx.cpp`. Con `mmtty-oracle`
//! compila il ricevitore RTTY di MMTTY 1.70L (`oracle/mmtty_rtty_rx.cpp` con le classi copiate in `oracle/mmtty/mmtty_dsp.inc`, LGPL).
//! Senza le feature non fa niente.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(feature = "fldigi-oracle")]
    oracle::build();
    #[cfg(feature = "mmtty-oracle")]
    mmtty::build();
}

#[cfg(feature = "mmtty-oracle")]
mod mmtty {
    use std::path::PathBuf;

    pub fn build() {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
        let main = manifest.join("oracle/mmtty_rtty_rx.cpp");
        let inc = manifest.join("oracle/mmtty/mmtty_dsp.inc");
        for f in [&main, &inc] {
            println!("cargo:rerun-if-changed={}", f.display());
        }
        cc::Build::new()
            .cpp(true)
            .file(&main)
            .include(manifest.join("oracle"))
            .define("_USE_MATH_DEFINES", None)
            .define("_CRT_SECURE_NO_WARNINGS", None)
            .flag_if_supported("/EHsc")
            .flag_if_supported("/utf-8")
            .flag_if_supported("-std=c++14")
            .warnings(false)
            .compile("aurora_mmtty");
    }
}

#[cfg(feature = "fldigi-oracle")]
mod oracle {
    use std::path::PathBuf;

    pub fn build() {
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
        let fldigi = manifest.join("../../../terze-parti/fldigi/src");
        let fftfilt = fldigi.join("filters/fftfilt.cxx");
        if !fftfilt.is_file() {
            panic!(
                "feature fldigi-oracle: manca il sorgente di fldigi ({}). Lanciare dalla radice del repository: git submodule update --init terze-parti/fldigi",
                fftfilt.display()
            );
        }
        let ours = [manifest.join("oracle/fldigi_rtty_rx.cpp"), manifest.join("oracle/fldigi_psk_rx.cpp"), manifest.join("oracle/fldigi_cw_rx.cpp")];
        // dai sorgenti di fldigi, compilati cosi' come sono: filtro FFT (RTTY), filtro FIR e media mobile (PSK, CW), varicode (PSK)
        let theirs = [fftfilt, fldigi.join("filters/filters.cxx"), fldigi.join("psk/pskvaricode.cxx")];
        for f in ours.iter().chain(theirs.iter()) {
            println!("cargo:rerun-if-changed={}", f.display());
        }
        let mut build = cc::Build::new();
        // filters.h di fldigi usa `__restrict__` di gcc: MSVC lo chiama `__restrict`
        if std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|e| e == "msvc") {
            build.define("__restrict__", "__restrict");
        }
        build
            .cpp(true)
            .files(ours.iter().chain(theirs.iter()))
            .include(manifest.join("oracle/stub"))
            .include(fldigi.join("include"))
            .define("_USE_MATH_DEFINES", None)
            .flag_if_supported("/EHsc")
            .flag_if_supported("-std=c++14")
            .warnings(false)
            .compile("aurora_fldigi");
    }
}
