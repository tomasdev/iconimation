//! Simple algorithmic animations
//!
//! Typically supports both a whole-icon and parts mode where parts animate offset slightly in time.

use bodymovin::properties::Value;
use bodymovin::properties::{Bezier2d, BezierEase, MultiDimensionalKeyframe, Property};
use bodymovin::shapes::{AnyShape, Fill, Group, Shape, Transform};
use kurbo::{BezPath, Point, Shape as KShape};

use crate::Error;

pub trait Animator {
    fn animate(
        &self,
        start: f64,
        end: f64,
        shapes: Vec<(BezPath, Shape)>,
    ) -> Result<Vec<AnyShape>, Error>;
}

pub struct Still;

impl Animator for Still {
    fn animate(
        &self,
        start: f64,
        end: f64,
        shapes: Vec<(BezPath, Shape)>,
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
        shapes: Vec<(BezPath, Shape)>,
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
        shapes: Vec<(BezPath, Shape)>,
    ) -> Result<Vec<AnyShape>, Error> {
        Ok(shapes
            .into_iter()
            .enumerate()
            .map(|(i, s)| pulse(start, end, i, vec![s]))
            .collect())
    }
}

pub struct Twirl;

impl Animator for Twirl {
    fn animate(
        &self,
        start: f64,
        end: f64,
        shapes: Vec<(BezPath, Shape)>,
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
        shapes: Vec<(BezPath, Shape)>,
    ) -> Result<Vec<AnyShape>, Error> {
        Ok(shapes
            .into_iter()
            .enumerate()
            .map(|(i, s)| twirl(start, end, i, vec![s]))
            .collect())
    }
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

fn center(shapes: &Vec<(BezPath, Shape)>) -> Point {
    shapes
        .iter()
        .map(|(b, _)| b.bounding_box())
        .reduce(|acc, e| acc.union(e))
        .map(|b| b.center())
        .unwrap_or_default()
}

fn pulse(start: f64, end: f64, shape_idx: usize, shapes: Vec<(BezPath, Shape)>) -> AnyShape {
    assert!(end > start);

    // If https://lottiefiles.github.io/lottie-docs/playground/json_editor/ is to be believed
    // the bezier ease is fairly required
    let ease = BezierEase::_2D(Bezier2d {
        in_value: Default::default(),
        out_value: Default::default(),
    });
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

fn twirl(start: f64, end: f64, shape_idx: usize, shapes: Vec<(BezPath, Shape)>) -> AnyShape {
    assert!(end > start);

    // If https://lottiefiles.github.io/lottie-docs/playground/json_editor/ is to be believed
    // the bezier ease is fairly required
    let ease = BezierEase::_2D(Bezier2d {
        in_value: Default::default(),
        out_value: Default::default(),
    });
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
