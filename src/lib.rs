//! Shove glyphs from a variable font into a Lottie template.

pub mod error;
mod shape_pen;

use bodymovin::{
    layers::AnyLayer,
    properties::Value,
    shapes::{AnyShape, Shape},
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
            for potential_placeholder in layer.mixin.shapes.iter_mut() {
                let AnyShape::Group(group) = potential_placeholder else {
                    continue;
                };
                if Some("placeholder") != group.name.as_deref() {
                    continue;
                }
                
                // We have all the best nesting
                let mut frontier = Vec::new();
                let mut insert_at = Vec::with_capacity(1);
                frontier.push(group);
                while let Some(group) = frontier.pop() {
                    insert_at.clear();
                    for (i, item) in group.items.iter_mut().enumerate() {

                        let lottie_box = match item {
                            AnyShape::Shape(shape) => {
                                // println!("shape");
                                Some(bez_for_shape(shape).control_box())
                            },
                            AnyShape::Rect(rect) => {
                                // println!("rect");
                                let Value::Fixed(pos) = &rect.position.value else {
                                    panic!("Unable to process {rect:#?} position, must be fixed");
                                };
                                let Value::Fixed(size) = &rect.size.value else {
                                    panic!("Unable to process {rect:#?} size, must be fixed");
                                };
                                assert_eq!(2, pos.len());
                                assert_eq!(2, size.len());
                                Some(Rect { x0: pos[0], y0: pos[1], x1: size[0], y1: size[1] })
                            }
                            _ => None,
                        };
                        let Some(lottie_box) = lottie_box else {
                            continue;
                        };
                        let font_to_lottie =
                            font_units_to_lottie_units(font_drawbox, &lottie_box);
                        insert_at.push((i, font_to_lottie));
                    }
                    // reverse because replacing 1:n shifts indices past our own
                    for (i, transform) in insert_at.iter().rev() {
                        let glyph_shapes = shapes_for_glyph(glyph, *transform)?;
                        group.items.splice(
                            *i..(*i + 1),
                            glyph_shapes.clone().into_iter().map(|s| AnyShape::Shape(s)),
                        );
                    }
                    shapes_updated += insert_at.len();

                    for item in group.items.iter_mut() {
                        match item {
                            AnyShape::Group(g) => frontier.push(g),
                            _ => (),
                        }
                    }
                }
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
