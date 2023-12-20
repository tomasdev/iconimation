mod transforms;

use std::{fs, path::Path};

use bodymovin::{layers, properties, shapes, Bodymovin as Lottie};
use iconimation::shapes_for_glyph;
use kurbo::Point;
use kurbo::{Affine, Rect};
use skrifa::{
    raw::{FontRef, TableProvider},
    MetadataProvider,
};

use crate::transforms::scale_transform;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

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
                .map(|ext| ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("otf"))
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
    let font_drawbox: Rect = (Point::ZERO, Point::new(upem, upem)).into();
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

    for (glyph, cp) in glyphs.iter().zip(codepoints.iter()) {
        // Here we should do some sort of `font_units_to_lottie_units` to ensure the animation never overflows the Lottie view box
        let glyph_shapes = shapes_for_glyph(glyph, Affine::FLIP_Y).unwrap();

        let layers: Vec<layers::AnyLayer> = glyph_shapes
            .clone()
            .chunks(2)
            .into_iter()
            .enumerate()
            .map(|(i, ss)| {
                layers::AnyLayer::Shape(layers::Shape {
                    transform: scale_transform(Some(3 * (i as i16))),
                    in_point: 0.0,
                    out_point: 60.0,
                    start_time: 0.0,
                    stretch: 1.0,
                    mixin: layers::ShapeMixin {
                        shapes: ss
                            .into_iter()
                            .map(|s| shapes::AnyShape::Shape(s.clone()))
                            .chain([shapes::AnyShape::Fill(shapes::Fill::default())])
                            .collect(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
            })
            .rev()
            .collect();

        let lottie = Lottie {
            in_point: 0.0,
            out_point: 60.0, // 60fps total animation = 1s
            frame_rate: 60.0,
            width: 960,
            height: 960,
            layers,
            ..Default::default()
        };

        let out_file = format!("custom-{cp:04x}.json");
        fs::write(&out_file, serde_json::to_string_pretty(&lottie).unwrap()).unwrap();
        eprintln!("Wrote {out_file:?}");
    }
}
