//! Painted paths, sorted the way pdfminer sorts them into rectangles, lines
//! and curves (`PDFLayoutAnalyzer.paint_path`), then turned into the edges
//! pdfplumber's table finder starts from (`utils/geometry.py`).

use crate::model::Rgb;
use crate::raw::{Op, Path};

#[derive(Debug, Clone)]
pub struct Rect {
    pub x0: f64,
    pub x1: f64,
    pub top: f64,
    pub bottom: f64,
    pub width: f64,
    pub height: f64,
    /// The non-stroking colour in force, whether or not the rectangle is filled.
    pub color: Option<Rgb>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub x0: f64,
    pub x1: f64,
    pub top: f64,
    pub bottom: f64,
    pub width: f64,
    pub height: f64,
    pub orientation: Orientation,
}

#[derive(Debug, Default)]
pub struct Shapes {
    pub rects: Vec<Rect>,
    /// Line edges, then rectangle edges, then curve edges: `Page.edges`.
    pub edges: Vec<Edge>,
    /// Rectangles, lines and curves, as pdfplumber would count them.
    pub count: usize,
}

type Point = (f64, f64);

fn bounds(points: &[Point]) -> (f64, f64, f64, f64) {
    let mut bound = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &(x, y) in points {
        bound = (bound.0.min(x), bound.1.min(y), bound.2.max(x), bound.3.max(y));
    }
    bound
}

pub fn classify(paths: &[Path], page_height: f64) -> Shapes {
    let mut rects = Vec::new();
    let mut line_edges = Vec::new();
    let mut curve_edges = Vec::new();
    let mut count = 0;

    for path in paths {
        // A path that does not open with a move is invalid and paints nothing.
        if path.ops.first().map(|op| op.0) != Some(Op::Move) {
            continue;
        }
        // One object per subpath. A move with nothing after it is dropped,
        // as pdfminer's `m[^m]+` does.
        let mut start = 0;
        for end in 1..=path.ops.len() {
            if end < path.ops.len() && path.ops[end].0 != Op::Move {
                continue;
            }
            let subpath = &path.ops[start..end];
            start = end;
            if subpath.len() < 2 {
                continue;
            }
            count += 1;
            paint(subpath, path.fill_color, page_height, &mut rects, &mut line_edges, &mut curve_edges);
        }
    }

    let mut edges = line_edges;
    for rect in &rects {
        edges.extend(rect_edges(rect));
    }
    edges.extend(curve_edges);
    Shapes { rects, edges, count }
}

fn paint(
    subpath: &[(Op, Point)],
    color: Option<Rgb>,
    page_height: f64,
    rects: &mut Vec<Rect>,
    line_edges: &mut Vec<Edge>,
    curve_edges: &mut Vec<Edge>,
) {
    let first = subpath[0].1;
    let mut shape: Vec<Op> = subpath.iter().map(|op| op.0).collect();
    // A close has no point of its own: it returns to where the subpath began.
    let mut points: Vec<Point> =
        subpath.iter().map(|&(op, point)| if op == Op::Close { first } else { point }).collect();

    // Drop a redundant line back to the start on a path that is closed anyway.
    let n = shape.len();
    if n > 3 && shape[n - 2] == Op::Line && shape[n - 1] == Op::Close && points[n - 2] == points[0] {
        shape.remove(n - 2);
        points.pop();
    }

    use Op::{Close, Line, Move};
    if shape == [Move, Line, Close] || shape == [Move, Line] {
        let (x0, y0, x1, y1) = bounds(&points[..2]);
        let (top, bottom) = (page_height - y1, page_height - y0);
        line_edges.push(Edge {
            x0,
            x1,
            top,
            bottom,
            width: x1 - x0,
            height: y1 - y0,
            // pdfplumber's `line_to_edge`: anything not exactly level is vertical.
            orientation: if top == bottom { Orientation::Horizontal } else { Orientation::Vertical },
        });
        return;
    }

    if shape == [Move, Line, Line, Line, Close] || shape == [Move, Line, Line, Line, Line] {
        let [(x0, y0), (x1, y1), (x2, y2), (x3, y3), last] = [points[0], points[1], points[2], points[3], points[4]];
        let closed = points[0] == last;
        let square = (x0 == x1 && y1 == y2 && x2 == x3 && y3 == y0) || (y0 == y1 && x1 == x2 && y2 == y3 && x3 == x0);
        if closed && square {
            let (bx0, by0, bx1, by1) = bounds(&[points[0], points[2]]);
            rects.push(Rect {
                x0: bx0,
                x1: bx1,
                top: page_height - by1,
                bottom: page_height - by0,
                width: bx1 - bx0,
                height: by1 - by0,
                color,
            });
            return;
        }
    }

    // Anything else is a curve; each of its level or upright segments is an edge.
    let flipped: Vec<Point> = points.iter().map(|&(x, y)| (x, page_height - y)).collect();
    for pair in flipped.windows(2) {
        let (p0, p1) = (pair[0], pair[1]);
        let orientation = if p0.0 == p1.0 {
            Orientation::Vertical
        } else if p0.1 == p1.1 {
            Orientation::Horizontal
        } else {
            continue;
        };
        curve_edges.push(Edge {
            x0: p0.0.min(p1.0),
            x1: p0.0.max(p1.0),
            top: p0.1.min(p1.1),
            bottom: p0.1.max(p1.1),
            width: (p0.0 - p1.0).abs(),
            height: (p0.1 - p1.1).abs(),
            orientation,
        });
    }
}

/// pdfplumber's `rect_to_edges`: top, bottom, left, right.
fn rect_edges(rect: &Rect) -> [Edge; 4] {
    let base = Edge {
        x0: rect.x0,
        x1: rect.x1,
        top: rect.top,
        bottom: rect.bottom,
        width: rect.width,
        height: rect.height,
        orientation: Orientation::Horizontal,
    };
    [
        Edge { height: 0.0, bottom: rect.top, ..base.clone() },
        Edge { height: 0.0, top: rect.top + rect.height, ..base.clone() },
        Edge { width: 0.0, x1: rect.x0, orientation: Orientation::Vertical, ..base.clone() },
        Edge { width: 0.0, x0: rect.x1, orientation: Orientation::Vertical, ..base },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::Op::{Close, Curve, Line, Move};

    fn path(ops: &[(Op, Point)]) -> Path {
        Path { ops: ops.to_vec(), fill_color: Some([1.0; 3]) }
    }

    #[test]
    fn a_closed_square_path_is_a_rectangle() {
        // What `re` leaves behind: four corners and a close.
        let shapes = classify(
            &[path(&[
                (Move, (10.0, 80.0)),
                (Line, (60.0, 80.0)),
                (Line, (60.0, 90.0)),
                (Line, (10.0, 90.0)),
                (Close, (10.0, 90.0)),
            ])],
            100.0,
        );
        assert_eq!(shapes.rects.len(), 1);
        let rect = &shapes.rects[0];
        assert_eq!((rect.x0, rect.top, rect.x1, rect.bottom), (10.0, 10.0, 60.0, 20.0));
        assert_eq!(shapes.edges.len(), 4);
        assert_eq!(shapes.count, 1);
    }

    #[test]
    fn a_line_back_to_the_start_before_a_close_is_redundant() {
        let shapes = classify(
            &[path(&[
                (Move, (0.0, 0.0)),
                (Line, (5.0, 0.0)),
                (Line, (5.0, 5.0)),
                (Line, (0.0, 5.0)),
                (Line, (0.0, 0.0)),
                (Close, (0.0, 0.0)),
            ])],
            100.0,
        );
        assert_eq!(shapes.rects.len(), 1);
    }

    #[test]
    fn two_points_make_a_line_and_a_slanted_one_counts_as_vertical() {
        let shapes = classify(
            &[path(&[(Move, (0.0, 50.0)), (Line, (30.0, 50.0))]), path(&[(Move, (0.0, 0.0)), (Line, (30.0, 40.0))])],
            100.0,
        );
        assert!(shapes.rects.is_empty());
        assert_eq!(shapes.edges[0].orientation, Orientation::Horizontal);
        assert_eq!(shapes.edges[1].orientation, Orientation::Vertical);
    }

    #[test]
    fn a_curve_gives_an_edge_per_level_or_upright_segment() {
        let triangle = path(&[(Move, (0.0, 0.0)), (Line, (40.0, 0.0)), (Line, (40.0, 30.0)), (Close, (40.0, 30.0))]);
        let shapes = classify(&[triangle], 100.0);
        assert!(shapes.rects.is_empty());
        // The base and the upright side; the slanted closing side is no edge.
        assert_eq!(shapes.edges.len(), 2);
    }

    #[test]
    fn each_subpath_is_its_own_object_and_a_lone_move_is_none() {
        let shapes = classify(
            &[path(&[
                (Move, (0.0, 0.0)),
                (Move, (0.0, 10.0)),
                (Line, (9.0, 10.0)),
                (Move, (0.0, 20.0)),
                (Curve, (9.0, 20.0)),
            ])],
            100.0,
        );
        assert_eq!(shapes.count, 2);
    }
}
