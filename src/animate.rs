//! Simple algorithmic animations
//!
//! Typically supports both a whole-icon and parts mode where parts animate offset slightly in time.

use bodymovin::properties::{Bezier2d, BezierEase, MultiDimensionalKeyframe, Property};
use bodymovin::properties::{ControlPoint2d, Value};
use bodymovin::shapes::{AnyShape, Fill, Group, SubPath, Transform};
use kurbo::{BezPath, Point, Shape};

use crate::Error;

#[derive(Clone, Debug)]
pub enum Animation {
    Still,
    PulseWhole,
    PulseParts,
    TwirlWhole,
    TwirlParts,
}

impl Animation {
    pub fn animator(&self) -> Box<dyn Animator> {
        match self {
            Animation::Still => Box::new(Still),
            Animation::PulseWhole => Box::new(Pulse),
            Animation::PulseParts => Box::new(PulseParts),
            Animation::TwirlWhole => Box::new(Twirl),
            Animation::TwirlParts => Box::new(TwirlParts),
        }
    }
}

pub trait Animator {
    fn animate(
        &self,
        start: f64,
        end: f64,
        shapes: Vec<(BezPath, SubPath)>,
    ) -> Result<Vec<AnyShape>, Error>;
}

pub struct Still;

impl Animator for Still {
    fn animate(
        &self,
        _: f64,
        _: f64,
        shapes: Vec<(BezPath, SubPath)>,
    ) -> Result<Vec<AnyShape>, Error> {
        Ok(shapes
            .into_iter()
            .map(|(_, s)| AnyShape::Shape(s))
            .collect())
    }
}

pub struct Pulse;

impl Animator for Pulse {
    fn animate(
        &self,
        start: f64,
        end: f64,
        shapes: Vec<(BezPath, SubPath)>,
    ) -> Result<Vec<AnyShape>, Error> {
        Ok(vec![pulse(start, end, 0, shapes)])
    }
}

pub struct PulseParts;

impl Animator for PulseParts {
    fn animate(
        &self,
        start: f64,
        end: f64,
        shapes: Vec<(BezPath, SubPath)>,
    ) -> Result<Vec<AnyShape>, Error> {
        Ok(group_per_direction(shapes)
            .into_iter()
            .enumerate()
            .map(|(i, s)| pulse(start, end, i, s))
            .collect())
    }
}

pub struct Twirl;

impl Animator for Twirl {
    fn animate(
        &self,
        start: f64,
        end: f64,
        shapes: Vec<(BezPath, SubPath)>,
    ) -> Result<Vec<AnyShape>, Error> {
        Ok(vec![twirl(start, end, 0, shapes)])
    }
}

pub struct TwirlParts;

impl Animator for TwirlParts {
    fn animate(
        &self,
        start: f64,
        end: f64,
        shapes: Vec<(BezPath, SubPath)>,
    ) -> Result<Vec<AnyShape>, Error> {
        Ok(group_per_direction(shapes)
            .into_iter()
            .enumerate()
            .map(|(i, s)| twirl(start, end, i, s))
            .collect())
    }
}

fn default_ease() -> BezierEase {
    // If https://lottiefiles.github.io/lottie-docs/playground/json_editor/ is to be believed
    // the bezier ease is usually required since we rarely want to hold
    BezierEase::_2D(Bezier2d {
        // the control point incoming to destination
        in_value: ControlPoint2d { x: 0.6, y: 1.0 },
        // the control point outgoing from origin
        out_value: ControlPoint2d { x: 0.4, y: 0.0 },
    })
}

/// This assumes the order of cw and ccw shapes carries meaning I believe it does not.
/// Still a step forward.
fn group_per_direction(shapes: Vec<(BezPath, SubPath)>) -> Vec<Vec<(BezPath, SubPath)>> {
    let mut result: Vec<Vec<(BezPath, SubPath)>> = vec![];

    for shape in shapes.into_iter() {
        if let Some(vec) = result.last_mut() {
            let last = vec.last().expect("Missing path");
            let dir1 = last.1.direction.expect("Missing previous path direction");
            let dir2 = shape.1.direction.expect("Missing current path direction");
            if dir1 == dir2 {
                // Same direction, new group
                result.push(vec![shape]);
            } else {
                // Different direction, reuse group
                vec.push(shape);
            }
        } else {
            // First item, new group
            result.push(vec![shape]);
        }
    }

    result
}

fn group_with_transform(shapes: Vec<(BezPath, SubPath)>, transform: Transform) -> AnyShape {
    // https://lottiefiles.github.io/lottie-docs/breakdown/bouncy_ball/#transform
    // says players like to find a transform at the end of a group and having a fill before
    // the transform seems fairly ubiquotous so we'll build our pulse as a group
    // of [shapes, fill, animated transform]
    let mut group = Group::default();
    group
        .items
        .extend(shapes.into_iter().map(|(_, s)| AnyShape::Shape(s)));
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
    group.items.push(AnyShape::Transform(transform));
    AnyShape::Group(group)
}

fn center(shapes: &Vec<(BezPath, SubPath)>) -> Point {
    shapes
        .iter()
        .map(|(b, _)| b.bounding_box())
        .reduce(|acc, e| acc.union(e))
        .map(|b| b.center())
        .unwrap_or_default()
}

fn pulse(start: f64, end: f64, shape_idx: usize, shapes: Vec<(BezPath, SubPath)>) -> AnyShape {
    assert!(end > start);

    let i = shape_idx as f64;
    let mut transform = Transform::default();

    // pulse around the center of the shape(s)
    // https://lottiefiles.github.io/lottie-docs/concepts/#transform
    // notes that anchor and position need to match for this
    let center = center(&shapes);
    transform.anchor_point = Property {
        value: Value::Fixed(vec![center.x, center.y]),
        ..Default::default()
    };
    transform.position = transform.anchor_point.clone();

    transform.scale.animated = 1;

    let ease = default_ease();
    transform.scale.value = Value::Animated(vec![
        MultiDimensionalKeyframe {
            start_time: 0.2 * (end - start) * i,
            start_value: Some(vec![100.0, 100.0]),
            bezier: Some(ease.clone()),
            ..Default::default()
        },
        MultiDimensionalKeyframe {
            start_time: 0.2 * (end - start) * (i + 1.0),
            start_value: Some(vec![150.0, 150.0]),
            bezier: Some(ease.clone()),
            ..Default::default()
        },
        MultiDimensionalKeyframe {
            start_time: 0.2 * (end - start) * (i + 2.0),
            start_value: Some(vec![100.0, 100.0]),
            bezier: Some(ease),
            ..Default::default()
        },
    ]);
    group_with_transform(shapes, transform)
}

fn twirl(start: f64, end: f64, shape_idx: usize, shapes: Vec<(BezPath, SubPath)>) -> AnyShape {
    assert!(end > start);

    let i = shape_idx as f64;
    let mut transform = Transform::default();

    // spin around the center of the shape(s)
    // https://lottiefiles.github.io/lottie-docs/concepts/#transform
    // notes that anchor and position need to match for this
    let center = center(&shapes);
    transform.anchor_point = Property {
        value: Value::Fixed(vec![center.x, center.y]),
        ..Default::default()
    };
    transform.position = transform.anchor_point.clone();

    transform.rotation.animated = 1;
    let ease = default_ease();
    transform.rotation.value = Value::Animated(vec![
        MultiDimensionalKeyframe {
            start_time: 0.2 * (end - start) * i,
            start_value: Some(vec![0.0]),
            bezier: Some(ease.clone()),
            ..Default::default()
        },
        MultiDimensionalKeyframe {
            start_time: 0.2 * (end - start) * (i + 2.0),
            start_value: Some(vec![360.0]),
            bezier: Some(ease),
            ..Default::default()
        },
    ]);
    group_with_transform(shapes, transform)
}
