//! An [OutlinePen] that generates an svg displaying subpaths.

use kurbo::{Affine, BezPath, PathEl, Point, Rect, Shape};
use ordered_float::OrderedFloat;
use skrifa::outline::OutlinePen;

use crate::animate::a_contained_point;

pub struct DebugPen {
    default_viewbox: Rect,
    paths: Vec<BezPath>,
}

enum MarkPoint {
    Contained(Point),
    Control(Point),
    End(Point),
}

impl MarkPoint {
    fn svg(&self) -> String {
        let (opacity, attr, point) = match self {
            MarkPoint::Contained(pt) => (100, " fill=\"red\"", *pt),
            MarkPoint::Control(pt) => (50, " fill=\"blue\"", *pt),
            MarkPoint::End(pt) => (75, "", *pt),
        };
        format!(
            "  <circle opacity=\"{opacity}%\" r=\"3\" cx=\"{}\" cy=\"{}\" {attr}/>\n",
            point.x, point.y
        )
    }
}

impl DebugPen {
    pub fn new(default_viewbox: Rect) -> DebugPen {
        DebugPen {
            default_viewbox,
            paths: Default::default(),
        }
    }

    fn active_subpath(&mut self) -> &mut BezPath {
        if self.paths.is_empty() {
            self.paths.push(BezPath::new());
        }
        return self.paths.last_mut().unwrap();
    }

    pub fn to_svg(self) -> String {
        let viewbox = self
            .paths
            .iter()
            .map(|p| p.control_box())
            .reduce(|acc, e| acc.union(e))
            .unwrap_or_default()
            .union(self.default_viewbox);

        // flip around the middle
        let transform = Affine::IDENTITY
            // Move center-y to be at y=0
            .then_translate((0.0, -viewbox.center().y).into())
            // Do a flip!
            .then_scale_non_uniform(1.0, -1.0)
            // Go back again
            .then_translate((0.0, viewbox.center().y).into());

        let mut svg = format!(
            r#"<svg viewBox="{} {} {} {}""#,
            viewbox.min_x(),
            viewbox.min_y(),
            viewbox.width(),
            viewbox.height()
        );
        svg.push_str(r#" xmlns="http://www.w3.org/2000/svg""#);
        svg.push_str(">\n");

        let mut paths: Vec<_> = self
            .paths
            .into_iter()
            .map(|mut p| {
                p.apply_affine(transform);
                p
            })
            .collect();

        paths.sort_by_cached_key(|p| OrderedFloat(p.area().abs()));

        for path in &paths {
            let contained = a_contained_point(&path);
            let mut filled = 0;
            eprintln!(
                "{} found contained? {} ({:?})",
                path.to_svg(),
                contained.is_some(),
                contained
            );
            if let Some(contained) = contained {
                // work out non-zero fill
                for path in &paths {
                    let wind = path.winding(contained);
                    filled += wind;
                    if contained == (280.0, 479.999).into() {
                        let path = path.to_svg();
                        eprintln!(
                            "  {} contributes {}",
                            &path[0..path.find(" ").unwrap()],
                            wind
                        );
                    }
                }
            }

            let filled = filled != 0; // nonzero winding?!
            eprintln!("  filled? {filled}");

            svg.push_str("  <path opacity=\"33%\" d=\"");
            svg.push_str(&path.to_svg());
            svg.push_str("  \"");
            if !filled {
                svg.push_str("\n        fill=\"none\" stroke=\"red\" stroke-dasharray=\"4\"");
            }
            svg.push_str(" />\n");

            let bbox = path.bounding_box();
            svg.push_str(&format!("  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"black\" />",
                bbox.min_x(), bbox.min_y(), bbox.width(), bbox.height()));

            let first_move = match path.elements().first() {
                Some(PathEl::MoveTo(p)) => format!("{p}"),
                _ => "??".to_string(),
            };

            svg.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\">first {first_move} area {:.2}</text>\n",
                bbox.min_x(),
                bbox.min_y() - 2.0,
                path.area()
            ));

            for el in path.elements() {
                let mut last_move = None;
                let mut curr = None;
                match el {
                    PathEl::MoveTo(p) => {
                        last_move = Some(p.clone());
                        curr = Some(p.clone());
                        svg.push_str(&MarkPoint::End(*p).svg());
                    }
                    PathEl::LineTo(p) => {
                        curr = Some(p.clone());
                        svg.push_str(&MarkPoint::End(*p).svg());
                    }
                    PathEl::QuadTo(c, p) => {
                        curr = Some(p.clone());
                        svg.push_str(&MarkPoint::Control(*c).svg());
                        svg.push_str(&MarkPoint::End(*p).svg());
                    }
                    PathEl::CurveTo(c0, c1, p) => {
                        curr = Some(p.clone());
                        svg.push_str(&MarkPoint::Control(*c0).svg());
                        svg.push_str(&MarkPoint::Control(*c1).svg());
                        svg.push_str(&MarkPoint::End(*p).svg());
                    }
                    PathEl::ClosePath => {
                        curr = last_move;
                        if let Some(last_move) = last_move {
                            svg.push_str(&MarkPoint::End(last_move).svg());
                        }
                    }
                }
            }

            if let Some(contained) = contained {
                svg.push_str(&MarkPoint::Contained(contained).svg());
            }
        }

        svg.push_str("\n</svg>");

        svg
    }
}

impl OutlinePen for DebugPen {
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
