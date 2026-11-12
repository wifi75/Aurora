//! Con la feature `fldigi-oracle` (solo per le prove, mai nell'installatore) compila il ricevitore RTTY di fldigi preso da
//! `terze-parti/fldigi` (sottomodulo) piu' il nostro `oracle/fldigi_rtty_rx.cpp`. Senza la feature non fa niente.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(feature = "fldigi-oracle")]
    oracle::build();
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
        let ours = manifest.join("oracle/fldigi_rtty_rx.cpp");
        for f in [&ours, &fftfilt] {
            println!("cargo:rerun-if-changed={}", f.display());
        }
        cc::Build::new()
            .cpp(true)
            .file(&ours)
            .file(&fftfilt)
            .include(manifest.join("oracle/stub"))
            .include(fldigi.join("include"))
            .define("_USE_MATH_DEFINES", None)
            .flag_if_supported("/EHsc")
            .flag_if_supported("-std=c++14")
            .warnings(false)
            .compile("aurora_fldigi_rtty");
    }
}
