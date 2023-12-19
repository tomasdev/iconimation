use std::{fs, path::Path};

use bodymovin::Bodymovin as Lottie;
use iconimation::Template;
use kurbo::Point;
use skrifa::{
    raw::{FontRef, TableProvider},
    MetadataProvider,
};

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

    let codepoints: Vec<_> = args
        .iter()
        .filter_map(|arg| {
            arg.starts_with("0x")
                .then(|| u32::from_str_radix(&arg[2..], 16).unwrap())
        })
        .collect();
    if codepoints.is_empty() {
        eprintln!("No codepoints!");
        return;
    }

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
    let font = FontRef::new(&font_bytes).unwrap();
    let upem = font.head().unwrap().units_per_em() as f64;
    let font_drawbox = (Point::ZERO, Point::new(upem, upem)).into();
    let outline_loader = font.outline_glyphs();

    let glyphs: Vec<_> = codepoints
        .iter()
        .map(|cp| {
            let gid = font
                .charmap()
                .map(*cp)
                .unwrap_or_else(|| panic!("No gid for 0x{cp:04x}"));
            outline_loader
                .get(gid)
                .unwrap_or_else(|| panic!("No outline for 0x{cp:04x} (gid {gid})"))
        })
        .collect();

    for lottie_file in lottie_files {
        let lottie = Lottie::load(lottie_file).unwrap();
        eprintln!("Parsed {lottie_file:?}");

        for (glyph, cp) in glyphs.iter().zip(codepoints.iter()) {
            let mut lottie = lottie.clone();
            lottie.replace_shape(&font_drawbox, &glyph).unwrap();

            let out_file = format!(
                "{}-{cp:04x}.json",
                lottie_file.file_stem().unwrap().to_str().unwrap()
            );
            fs::write(&out_file, serde_json::to_string_pretty(&lottie).unwrap()).unwrap();
            eprintln!("Wrote {out_file:?}");
        }
    }
}
