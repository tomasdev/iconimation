//! Shove glyphs from a variable font into a Lottie template.

pub mod animate;
pub mod debug_pen;
pub mod error;
mod shape_pen;

use animate::Animation;
use bodymovin::{
    layers::{AnyLayer, ShapeMixin},
    properties::{Property, Value},
    shapes::{AnyShape, Group, SubPath},
    Bodymovin as Lottie,
};
use kurbo::{Affine, BezPath, Point, Rect};
use skrifa::{
    instance::Size,
    raw::{FontRef, TableProvider},
    OutlineGlyph,
};
use write_fonts::pens::TransformPen;

use crate::{animate::Animator, error::Error, shape_pen::SubPathPen};

pub fn default_template(font_drawbox: &Rect) -> Lottie {
    Lottie {
        in_point: 0.0,
        out_point: 60.0, // 60fps total animation = 1s
        frame_rate: 60.0,
        width: font_drawbox.width() as i64,
        height: font_drawbox.height() as i64,
        layers: vec![AnyLayer::Shape(bodymovin::layers::Shape {
            in_point: 0.0,
            out_point: 60.0, // 60fps total animation = 1s
            mixin: ShapeMixin {
                shapes: vec![AnyShape::Group(Group {
                    name: Some("placeholder".into()),
                    items: vec![
                        // de facto standard is shape(s), fill, transform
                        AnyShape::Rect(bodymovin::shapes::Rect {
                            position: Property {
                                value: Value::Fixed(vec![0.0, 0.0]),
                                ..Default::default()
                            },
                            size: Property {
                                value: Value::Fixed(vec![
                                    font_drawbox.width(),
                                    font_drawbox.height(),
                                ]),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                        AnyShape::Fill(Default::default()),
                        AnyShape::Transform(Default::default()),
                    ],
                    ..Default::default()
                })],
                ..Default::default()
            },
            ..Default::default()
        })],
        ..Default::default()
    }
}

pub trait Template {
    fn replace_shape(
        &mut self,
        font_drawbox: &Rect,
        glyph: &OutlineGlyph,
        animator: Box<dyn Animator>,
    ) -> Result<(), Error>;
}

impl Template for Lottie {
    fn replace_shape(
        &mut self,
        font_drawbox: &Rect,
        glyph: &OutlineGlyph,
        animator: Box<dyn Animator>,
    ) -> Result<(), Error> {
        for layer in self.layers.iter_mut() {
            let AnyLayer::Shape(layer) = layer else {
                continue;
            };
            let mut shapes_updated = 0;
            let placeholders: Vec<_> = layer
                .mixin
                .shapes
                .iter_mut()
                .filter_map(|any| match any {
                    AnyShape::Group(group) if group.name.as_deref() == Some("placeholder") => {
                        Some(group)
                    }
                    _ => None,
                })
                .collect();

            let mut insert_at = Vec::with_capacity(1);
            for placeholder in placeholders {
                insert_at.clear();
                for (i, item) in placeholder.items.iter_mut().enumerate() {
                    let lottie_box = match item {
                        AnyShape::Shape(shape) => Some(bez_for_subpath(shape).control_box()),
                        AnyShape::Rect(rect) => {
                            let Value::Fixed(pos) = &rect.position.value else {
                                panic!("Unable to process {rect:#?} position, must be fixed");
                            };
                            let Value::Fixed(size) = &rect.size.value else {
                                panic!("Unable to process {rect:#?} size, must be fixed");
                            };
                            assert_eq!(2, pos.len());
                            assert_eq!(2, size.len());
                            Some(Rect {
                                x0: pos[0],
                                y0: pos[1],
                                x1: size[0],
                                y1: size[1],
                            })
                        }
                        _ => None,
                    };
                    let Some(lottie_box) = lottie_box else {
                        continue;
                    };
                    let font_to_lottie = font_units_to_lottie_units(font_drawbox, &lottie_box);
                    insert_at.push((i, font_to_lottie));
                }
                // reverse because replacing 1:n shifts indices past our own
                for (i, transform) in insert_at.iter().rev() {
                    let mut glyph_shapes: Vec<_> = subpaths_for_glyph(glyph, *transform)?;
                    glyph_shapes.sort_by_cached_key(|(b, _)| {
                        let bbox = b.control_box();
                        (
                            (bbox.min_y() * 1000.0) as i64,
                            (bbox.min_x() * 1000.0) as i64,
                        )
                    });
                    eprintln!("Animating {} glyph shapes", glyph_shapes.len());
                    let animated_shapes =
                        animator.animate(layer.in_point, layer.out_point, glyph_shapes)?;
                    placeholder.items.splice(*i..(*i + 1), animated_shapes);
                }
                shapes_updated += insert_at.len();
            }

            if shapes_updated == 0 {
                panic!("No placeholders replaced!!");
            }
        }
        Ok(())
    }
}

/// Simplified version of [Affine2D::rect_to_rect](https://github.com/googlefonts/picosvg/blob/a0bcfade7a60cbd6f47d8bfe65b6d471cee628c0/src/picosvg/svg_transform.py#L216-L263)
fn font_units_to_lottie_units(font_box: &Rect, lottie_box: &Rect) -> Affine {
    assert!(font_box.width() > 0.0);
    assert!(font_box.height() > 0.0);
    assert!(lottie_box.width() > 0.0);
    assert!(lottie_box.height() > 0.0);

    let (sx, sy) = (
        lottie_box.width() / font_box.width(),
        lottie_box.height() / font_box.height(),
    );
    let transform = Affine::IDENTITY
        // Move the font box to touch the origin
        .then_translate((-font_box.min_x(), -font_box.min_y()).into())
        // Do a flip!
        .then_scale_non_uniform(1.0, -1.0)
        // Scale to match the target box
        .then_scale_non_uniform(sx, sy);

    // Line up
    let adjusted_font_box = transform.transform_rect_bbox(*font_box);
    transform.then_translate(
        (
            lottie_box.min_x() - adjusted_font_box.min_x(),
            lottie_box.min_y() - adjusted_font_box.min_y(),
        )
            .into(),
    )
}

fn bez_for_subpath(subpath: &SubPath) -> BezPath {
    let Value::Fixed(value) = &subpath.vertices.value else {
        panic!("what is {subpath:?}");
    };

    let mut path = BezPath::new();
    if !value.vertices.is_empty() {
        path.move_to(value.vertices[0]);
    }
    for (start_end, (c0, c1)) in value
        .vertices
        .windows(2)
        .zip(value.in_point.iter().zip(value.out_point.iter()))
    {
        let end = start_end[1];
        path.curve_to(*c0, *c1, end);
    }
    path
}

/// Returns a [Shape] and [BezPath] in Lottie units for each subpath of a glyph
fn subpaths_for_glyph(
    glyph: &OutlineGlyph,
    font_units_to_lottie_units: Affine,
) -> Result<Vec<(BezPath, SubPath)>, Error> {
    // Fonts draw Y-up, Lottie Y-down. The transform to transition should be negative determinant.
    // Normally a negative determinant flips curve direction but since we're also moving
    // to a coordinate system with Y flipped it should cancel out.
    assert!(
        font_units_to_lottie_units.determinant() < 0.0,
        "We assume a negative determinant"
    );

    let mut subpath_pen = SubPathPen::default();
    let mut transform_pen = TransformPen::new(&mut subpath_pen, font_units_to_lottie_units);
    glyph
        .draw(Size::unscaled(), &mut transform_pen)
        .map_err(Error::DrawError)?;

    Ok(subpath_pen.to_shapes())
}

pub fn lottie_for_glyph(
    font: FontRef<'_>,
    glyph: OutlineGlyph<'_>,
    animation: Animation,
) -> String {
    let upem = font.head().unwrap().units_per_em() as f64;
    let font_drawbox: Rect = (Point::ZERO, Point::new(upem, upem)).into();

    let mut lottie = default_template(&font_drawbox);
    lottie
        .replace_shape(&font_drawbox, &glyph, animation.animator())
        .unwrap();

    serde_json::to_string_pretty(&lottie).unwrap()
}

#[cfg(test)]
mod tests {}
