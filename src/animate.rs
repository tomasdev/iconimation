//! Simple algorithmic animations
//!
//! Typically supports both a whole-icon and parts mode where parts animate offset slightly in time.

use bodymovin::properties::{Bezier2d, BezierEase, MultiDimensionalKeyframe, Property};
use bodymovin::properties::{ControlPoint2d, Value};
use bodymovin::shapes::{AnyShape, Fill, Group, SubPath, Transform};
use kurbo::{BezPath, PathEl, Point, Shape, Vec2};
use ordered_float::OrderedFloat;

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
        Ok(group_icon_parts(shapes)
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
        Ok(group_icon_parts(shapes)
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

/// Find a point that is contained within the subpath
///
/// Meant for simplified (assume the answer is the same for the entire subpath) nonzero fill resolution.
pub fn a_contained_point(subpath: &BezPath) -> Option<Point> {
    let Some(PathEl::MoveTo(p)) = subpath.elements().first() else {
        eprintln!("Subpath doesn't start with a move!");
        return None;
    };

    // our shapes are simple, just bet that a nearby point is contained
    let offsets = [0.0, 0.001, -0.001];
    offsets
        .iter()
        .flat_map(|x_off| offsets.iter().map(|y_off| Vec2::new(*x_off, *y_off)))
        .map(|offset| *p + offset)
        .find(|p| subpath.contains(*p))
}

/// Piece-wise animation wants to animate "parts" as the eye perceives them; try to so group
///
/// Most importantly, if we have a shape and hole(s) cut out of it they should be together.
///
/// Make some simplifying assumptions:
///
/// 1. Icons don't typically use one subpath to cut a hole in many other subpaths
/// 1. Icons typically fully contain the holepunch within the ... punchee?
///
/// Since we are using non-zero fill, figure out shape by shape what the winding value is. Initially I thought
/// we could simply look at the direction from [`BezPath::area`] but that ofc isn't enough to know if the final
/// winding is nonzero.
fn group_icon_parts(shapes: Vec<(BezPath, SubPath)>) -> Vec<Vec<(BezPath, SubPath)>> {
    // Figure out what is/isn't filled
    let filled: Vec<_> = shapes
        .iter()
        .map(|(bez, _)| {
            let Some(contained) = a_contained_point(bez) else {
                return false;
            };
            let winding: i32 = shapes.iter().map(|(bez, _)| bez.winding(contained)).sum();
            winding != 0
        })
        .collect();

    // Sort filled ahead of unfilled, smaller before larger (to simplify matching below)
    let mut ordered: Vec<_> = (0..shapes.len()).collect();
    ordered.sort_by_cached_key(|i| {
        (
            -1 * filled[*i] as i32,
            OrderedFloat(-shapes[*i].0.area().abs()),
        )
    });

    // Group cutouts with the smallest containing filled subpath
    // Doesn't generalize but perhaps suffices for icons
    // In each group [0] must exist and is a filled subpath, [1..n] are optional and are unfilled
    let mut groups: Vec<Vec<(BezPath, SubPath)>> = Default::default();
    let mut bboxes = Vec::default(); // the bbox of group[n][0] is bbox[n]
    for i in ordered {
        let (bez, subpath) = shapes.get(i).unwrap().clone();
        let bbox = bez.bounding_box();
        if filled[i] {
            // start a new group for a filled subpath
            groups.push(vec![(bez, subpath)]);
            bboxes.push(bbox);
        } else {
            // add cutout to the smallest (first, courtesy of sort above) containing filled subpath
            if let Some(i) = bboxes
                .iter()
                .position(|group_bbox| group_bbox.intersect(bbox) == bbox)
            {
                groups[i].push((bez, subpath));
            } else {
                eprintln!(
                    "Uh oh, we have an unfilled shape that didn't land anywhere! {}",
                    bez.to_svg()
                );
            }
        }
    }

    groups
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
            value: Value::Fixed(50.0), // handy for debugging overlapping shapes
            ..Default::default()
        },
        color: Default::default(),
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
