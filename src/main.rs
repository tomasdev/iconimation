use std::{fs, path::Path};

use bodymovin::Bodymovin as Lottie;
use iconimation::Template;
use skrifa::raw::FontRef;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let lottie_files: Vec<_> = args
        .iter()
        .map(Path::new)
        .filter(|p| {
            p.extension()
                .map(|ext| ext.eq_ignore_ascii_case("json"))
                .unwrap_or_default()
        })
        .collect();

    let font_file: Vec<_> = args
        .iter()
        .map(Path::new)
        .filter(|p| {
            p.extension()
                .map(|ext| ext.eq_ignore_ascii_case("ttf"))
                .unwrap_or_default()
        })
        .collect();
    if font_file.len() != 1 {
        eprintln!("Must have exactly one font file, got {}", font_file.len());
        return;
    }
    let font_file = font_file[0];
    let font_bytes = fs::read(font_file).unwrap();
    let font = FontRef::new(&font_bytes);

    let codepoints: Vec<_> = args
        .iter()
        .filter_map(|arg| {
            arg.starts_with("0x")
                .then(|| u32::from_str_radix(&arg[2..], 16).unwrap())
        })
        .collect();

    for lottie_file in lottie_files {
        let lottie = Lottie::load(lottie_file).unwrap();
        eprintln!("Parsed {lottie_file:?}");

        for codepoint in &codepoints {
            let lottie = lottie.clone().replace_shape().unwrap();
        }

        eprintln!(
            "Updated\n{}",
            serde_json::to_string_pretty(&lottie).unwrap()
        );
    }
}
