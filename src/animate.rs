//! Simple algorithmic animations
//!
//! Typically supports both a whole-icon and parts mode where parts animate offset slightly in time.

use bodymovin::properties::{Bezier2d, BezierEase, MultiDimensionalKeyframe, Property};
use bodymovin::properties::{ControlPoint2d, Value};
use bodymovin::shapes::{AnyShape, Fill, Group, SubPath, Transform};
use kurbo::{BezPath, Point, Shape};

use crate::Error;

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

/// Piecewise animation has to keep cw and ccw (hole cutting) paths together if doing so alters rendering.
///
/// This implementation is quick and dirty: group shapes with overlapping bbox and keep groups together
/// if they contain both positive and negative area (e.g. cw and ccw shapes). This will over-group; shapes
/// whoses bounding box overlaps that don't impact one anothers rendering will be grouped.
fn shape_groups_for_piecewise_animation(
    mut shapes: Vec<(BezPath, SubPath)>,
) -> Vec<Vec<(BezPath, SubPath)>> {
    let num_shapes = shapes.len();
    let mut groups: Vec<Vec<(BezPath, SubPath)>> = Vec::new();

    // icons have very few shapes, no need to stress efficiency
    while let Some((bez, subpath)) = shapes.pop() {
        // find every group that contains a shape whose bounding box we intersect
        let indices: Vec<_> = groups
            .iter()
            .enumerate()
            .filter_map(|(i, g)| {
                g.iter()
                    .any(|(bez2, _)| {
                        bez.bounding_box().intersect(bez2.bounding_box()).area() != 0.0
                    })
                    .then(|| i)
            })
            .collect();

        if let Some(merge_into_idx) = indices.first().cloned() {
            // if anything matched, merge those groups and add us
            for idx in indices.into_iter().rev().filter(|i| *i != merge_into_idx) {
                let merge_me = groups.remove(idx);
                groups[merge_into_idx].extend(merge_me);
            }
            groups[merge_into_idx].push((bez, subpath));
        } else {
            // we're a new group
            groups.push(vec![(bez, subpath)]);
        }
    }
    eprintln!("{} groups from {} shapes", groups.len(), num_shapes);
    groups
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

fn group_per_direction(shapes: Vec<(BezPath, Shape)>) -> Vec<Vec<(BezPath, Shape)>> {
    let mut result: Vec<Vec<(BezPath, Shape)>> = vec![];

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

fn group_with_transform(shapes: Vec<(BezPath, Shape)>, transform: Transform) -> AnyShape {
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
