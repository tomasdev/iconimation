//! Shove glyphs from a variable font into a Lottie template.

pub mod error;
mod shape_pen;

use bodymovin::{
    layers::AnyLayer,
    properties::{
        Bezier2d, BezierEase, ControlPoint2d, MultiDimensionalKeyframe, MultiDimensionalValue,
        Property, Value,
    },
    shapes::{AnyShape, Fill, Group, Shape, Transform},
    Bodymovin as Lottie,
};
use kurbo::{Affine, BezPath, Rect};
use skrifa::{instance::Size, OutlineGlyph};
use write_fonts::pens::TransformPen;

use crate::{error::Error, shape_pen::ShapePen};

pub trait Template {
    fn replace_shape(&mut self, font_drawbox: &Rect, glyph: &OutlineGlyph) -> Result<(), Error>;
}

impl Template for Lottie {
    fn replace_shape(&mut self, font_drawbox: &Rect, glyph: &OutlineGlyph) -> Result<(), Error> {
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
                        AnyShape::Shape(shape) => Some(bez_for_shape(shape).control_box()),
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
                    let glyph_shapes: Vec<_> = shapes_for_glyph(glyph, *transform)?
                        .into_iter()
                        .enumerate()
                        // TODO: we probably don't *always* want pulse
                        .map(pulse)
                        // .map(|shape| AnyShape::Shape(shape))
                        .collect();
                    eprintln!("Splice {} shapes in", glyph_shapes.len());
                    placeholder.items.splice(*i..(*i + 1), glyph_shapes);
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

fn pulse(args: (usize, Shape)) -> AnyShape {
    let (i, shape) = args;
    // https://lottiefiles.github.io/lottie-docs/breakdown/bouncy_ball/#transform
    // says players like to find a transform at the end of a group so we'll build our pulse as a group
    // of [shape, pulse]
    let mut group = Group::default();
    group.items.push(AnyShape::Shape(shape));
    group.items.push(AnyShape::Fill(Fill {
        opacity: Property {
            value: Value::Fixed(100.0),
            ..Default::default()
        },
        color: Property {
            value: Value::Fixed(vec![1.0, 0.0, 0.0, 1.0]),
            ..Default::default()
        },
        ..Default::default()
    }));
    // If https://lottiefiles.github.io/lottie-docs/playground/json_editor/ is to be believed
    // the bezier ease is fairly required
    let ease = BezierEase::_2D(Bezier2d {
        in_value: Default::default(),
        out_value: Default::default(),
    });
    let i = i as f64;
    let mut transform = Transform::default();
    transform.scale.animated = 1;
    transform.scale.value = Value::Animated(vec![
        MultiDimensionalKeyframe {
            start_time: 5.0 * i,
            start_value: Some(vec![100.0, 100.0]),
            bezier: Some(ease.clone()),
            ..Default::default()
        },
        MultiDimensionalKeyframe {
            start_time: 10.0 + 5.0 * i,
            start_value: Some(vec![150.0, 150.0]),
            bezier: Some(ease.clone()),
            ..Default::default()
        },
        MultiDimensionalKeyframe {
            start_time: 15.0 + 5.0 * i,
            start_value: Some(vec![100.0, 100.0]),
            bezier: Some(ease),
            ..Default::default()
        },
    ]);
    group.items.push(AnyShape::Transform(transform));

    AnyShape::Group(group)
}

/// Simplified version of [Affine2D::rect_to_rect](https://github.com/googlefonts/picosvg/blob/a0bcfade7a60cbd6f47d8bfe65b6d471cee628c0/src/picosvg/svg_transform.py#L216-L263)
fn font_units_to_lottie_units(font_box: &Rect, lottie_box: &Rect) -> Affine {
    // println!("font_box is: {:?}", font_box);
    // println!("lottie_box is: {:?}", lottie_box);

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

fn bez_for_shape(shape: &Shape) -> BezPath {
    let Value::Fixed(shape) = &shape.vertices.value else {
        panic!("what is {shape:?}");
    };

    let mut path = BezPath::new();
    if !shape.vertices.is_empty() {
        path.move_to(shape.vertices[0]);
    }
    for (start_end, (c0, c1)) in shape
        .vertices
        .windows(2)
        .zip(shape.in_point.iter().zip(shape.out_point.iter()))
    {
        let end = start_end[1];
        path.curve_to(*c0, *c1, end);
    }
    path
}

pub fn shapes_for_glyph(
    glyph: &OutlineGlyph,
    font_units_to_lottie_units: Affine,
) -> Result<Vec<Shape>, Error> {
    // Fonts draw Y-up, Lottie Y-down. The transform to transition should be negative determinant.
    // Normally a negative determinant flips curve direction but since we're also moving
    // to a coordinate system with Y flipped it should cancel out.
    assert!(
        font_units_to_lottie_units.determinant() < 0.0,
        "We assume a negative determinant"
    );

    let mut shape_pen = ShapePen::default();
    let mut transform_pen = TransformPen::new(&mut shape_pen, font_units_to_lottie_units);
    glyph
        .draw(Size::unscaled(), &mut transform_pen)
        .map_err(Error::DrawError)?;

    Ok(shape_pen.to_shapes())
}

#[cfg(test)]
mod tests {}
