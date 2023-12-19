//! An [OutlinePen] that generates [Shape]'s for subpaths.
//!
//! Based on <https://github.com/rsheeter/read-fonts-wasm>

use bodymovin::{
    properties::{Property, ShapeValue, Value},
    shapes::Shape,
};
use kurbo::{BezPath, Point};
use skrifa::outline::OutlinePen;

#[derive(Default)]
pub struct ShapePen {
    paths: Vec<BezPath>,
}

impl ShapePen {
    fn active_subpath(&mut self) -> &mut BezPath {
        if self.paths.is_empty() {
            self.paths.push(BezPath::new());
        }
        return self.paths.last_mut().unwrap();
    }

    pub fn to_shapes(self) -> Vec<Shape> {
        self.paths.iter().map(bez_to_shape).collect()
    }
}

impl OutlinePen for ShapePen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.paths.push(BezPath::new());
        self.active_subpath().move_to((x as f64, y as f64));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.active_subpath().line_to((x as f64, y as f64));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.active_subpath()
            .quad_to((cx0 as f64, cy0 as f64), (x as f64, y as f64));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.active_subpath().curve_to(
            (cx0 as f64, cy0 as f64),
            (cx1 as f64, cy1 as f64),
            (x as f64, y as f64),
        );
    }

    fn close(&mut self) {
        self.active_subpath().close_path();
    }
}

trait Thirds {
    fn one_third(&self) -> Self;
    fn two_thirds(&self) -> Self;
}

impl Thirds for Point {
    fn one_third(&self) -> Self {
        (self.x / 3.0, self.y / 3.0).into()
    }

    fn two_thirds(&self) -> Self {
        (self.x * 2.0 / 3.0, self.y * 2.0 / 3.0).into()
    }
}

fn bez_to_shape(path: &BezPath) -> Shape {
    eprintln!("bez to shape, cbox {:?}", path.control_box());
    // Shape is a cubic B-Spline
    //  vertices are oncurve
    //  out_point is the first control point
    //  in_point is the second control point
    // Cubic[i] is formed vertices[i], outgoing[i], incoming[i], vertices[i + 1]
    // If closed 1 past the end of vertices is vertices[0]
    let mut vertices = ShapeValue::default();
    for el in path.iter() {
        let last_on: Point = vertices.vertices.last().cloned().unwrap_or_default().into();
        match el {
            kurbo::PathEl::MoveTo(p) => {
                vertices.vertices.push(p.into());

                vertices.out_point.push(p.into());
                vertices.in_point.push(p.into());
            }
            kurbo::PathEl::LineTo(p) => {
                vertices.out_point.push(last_on.into());
                vertices.in_point.push(p.into());
                vertices.vertices.push(p.into());
            }
            kurbo::PathEl::QuadTo(control, end) => {
                // https://pomax.github.io/bezierinfo/#reordering
                let c0 = last_on.one_third() + control.two_thirds().to_vec2();
                let c1 = control.two_thirds() + end.one_third().to_vec2();
                vertices.out_point.push(c0.into());
                vertices.in_point.push(c1.into());
                vertices.vertices.push(end.into());
            }
            kurbo::PathEl::CurveTo(c0, c1, end) => {
                vertices.out_point.push(c0.into());
                vertices.in_point.push(c1.into());
                vertices.vertices.push(end.into());
            }
            kurbo::PathEl::ClosePath => {
                vertices.closed = Some(true);
            }
        }
    }
    if vertices.closed.is_none() {
        vertices.closed = Some(
            vertices.vertices.first().cloned().unwrap_or_default()
                == vertices.vertices.last().cloned().unwrap_or_default(),
        );
    }
    Shape {
        closed: vertices.closed.unwrap_or_default(),
        vertices: Property {
            value: Value::Fixed(vertices),
            ..Default::default()
        },
        ..Default::default()
    }
}
