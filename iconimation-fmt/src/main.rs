//! Load a lottie file and dump it.
//!
//! Useful to determine if that alone trashes a template.
use std::{fs, path::Path};

use bodymovin::Bodymovin as Lottie;

fn main() {
    for lottie_file in std::env::args().skip(1) {
        let lottie_file = Path::new(&lottie_file);
        let lottie = Lottie::load(lottie_file)
            .unwrap_or_else(|e| panic!("Unable to load {lottie_file:?}: {e}"));
        let out_file = lottie_file.with_file_name(format!(
            "{}-pretty.{}",
            lottie_file.file_stem().unwrap().to_str().unwrap(),
            lottie_file.extension().unwrap().to_str().unwrap()
        ));
        fs::write(&out_file, serde_json::to_string_pretty(&lottie).unwrap()).unwrap();
        eprintln!("Wrote {out_file:?}");
    }
}
