//! pdfplumber's table finder with its default settings (`table.py`): both
//! strategies `lines`, snap, join and intersection tolerances of 3 pt, edges
//! shorter than 1 pt ignored before merging and shorter than 3 pt after.
//!
//! Every step is kept in the order pdfplumber performs it, down to which sort
//! is stable, because cells are identified by coordinates compared for
//! equality: two edges land on the same vertex only if the same arithmetic
//! put them there.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::cluster::cluster_objects;
use crate::model::BBox;
use crate::objects::{Edge, Orientation};

const SNAP_TOLERANCE: f64 = 3.0;
const JOIN_TOLERANCE: f64 = 3.0;
const INTERSECTION_TOLERANCE: f64 = 3.0;
const EDGE_MIN_LENGTH_PREFILTER: f64 = 1.0;
const EDGE_MIN_LENGTH: f64 = 3.0;

#[derive(Debug, Clone)]
pub struct Table {
    pub cells: Vec<BBox>,
    pub bbox: BBox,
}

fn length(edge: &Edge) -> f64 {
    match edge.orientation {
        Orientation::Vertical => edge.height,
        Orientation::Horizontal => edge.width,
    }
}

/// Snap edges within tolerance of one another to their average position.
fn snap(edges: Vec<Edge>) -> Vec<Edge> {
    let (vertical, horizontal): (Vec<Edge>, Vec<Edge>) =
        edges.into_iter().partition(|edge| edge.orientation == Orientation::Vertical);
    let mut snapped = Vec::with_capacity(vertical.len() + horizontal.len());
    for cluster in cluster_objects(&vertical, |edge| edge.x0, SNAP_TOLERANCE) {
        let average = cluster.iter().map(|edge| edge.x0).sum::<f64>() / cluster.len() as f64;
        for mut edge in cluster {
            let shift = average - edge.x0;
            edge.x0 += shift;
            edge.x1 += shift;
            snapped.push(edge);
        }
    }
    for cluster in cluster_objects(&horizontal, |edge| edge.top, SNAP_TOLERANCE) {
        let average = cluster.iter().map(|edge| edge.top).sum::<f64>() / cluster.len() as f64;
        for mut edge in cluster {
            let shift = average - edge.top;
            edge.top += shift;
            edge.bottom += shift;
            snapped.push(edge);
        }
    }
    snapped
}

/// Join edges on the same infinite line that touch or nearly touch.
fn join_group(mut group: Vec<Edge>, orientation: Orientation) -> Vec<Edge> {
    let start = |edge: &Edge| if orientation == Orientation::Horizontal { edge.x0 } else { edge.top };
    let end = |edge: &Edge| if orientation == Orientation::Horizontal { edge.x1 } else { edge.bottom };
    group.sort_by(|a, b| start(a).total_cmp(&start(b)));
    let mut joined: Vec<Edge> = Vec::new();
    for edge in group {
        match joined.last_mut() {
            Some(last) if start(&edge) <= end(last) + JOIN_TOLERANCE => {
                if end(&edge) > end(last) {
                    if orientation == Orientation::Horizontal {
                        last.x1 = edge.x1;
                        last.width = edge.x1 - last.x0;
                    } else {
                        last.height += edge.bottom - last.bottom;
                        last.bottom = edge.bottom;
                    }
                }
            }
            _ => joined.push(edge),
        }
    }
    joined
}

fn merge(edges: Vec<Edge>) -> Vec<Edge> {
    let mut edges = snap(edges);
    let position = |edge: &Edge| if edge.orientation == Orientation::Horizontal { edge.top } else { edge.x0 };
    // ("h", top) sorts before ("v", x0).
    edges.sort_by(|a, b| a.orientation.cmp(&b.orientation).then(position(a).total_cmp(&position(b))));
    let mut merged = Vec::new();
    let mut group: Vec<Edge> = Vec::new();
    for edge in edges {
        if let Some(first) = group.first() {
            if first.orientation != edge.orientation || position(first) != position(&edge) {
                let orientation = first.orientation;
                merged.extend(join_group(std::mem::take(&mut group), orientation));
            }
        }
        group.push(edge);
    }
    if let Some(first) = group.first() {
        let orientation = first.orientation;
        merged.extend(join_group(group, orientation));
    }
    merged
}

type Vertex = (u64, u64);

fn vertex(x: f64, y: f64) -> Vertex {
    ((x + 0.0).to_bits(), (y + 0.0).to_bits())
}

#[derive(Default)]
struct Crossing {
    vertical: HashSet<usize>,
    horizontal: HashSet<usize>,
}

/// Edges are compared by bounding box, so two edges with the same box are one.
fn identities(edges: &[Edge]) -> Vec<usize> {
    let mut seen: HashMap<[u64; 4], usize> = HashMap::new();
    edges
        .iter()
        .map(|edge| {
            let key = [edge.x0, edge.top, edge.x1, edge.bottom].map(|value| (value + 0.0).to_bits());
            let next = seen.len();
            *seen.entry(key).or_insert(next)
        })
        .collect()
}

pub fn find_tables(page_edges: &[Edge]) -> Vec<Table> {
    let usable = |edge: &&Edge, orientation: Orientation| {
        edge.orientation == orientation && length(edge) >= EDGE_MIN_LENGTH_PREFILTER
    };
    let mut edges: Vec<Edge> = page_edges.iter().filter(|edge| usable(edge, Orientation::Vertical)).cloned().collect();
    edges.extend(page_edges.iter().filter(|edge| usable(edge, Orientation::Horizontal)).cloned());
    let edges: Vec<Edge> = merge(edges).into_iter().filter(|edge| length(edge) >= EDGE_MIN_LENGTH).collect();
    let ids = identities(&edges);

    // Where a vertical and a horizontal edge meet, within tolerance.
    let mut crossings: BTreeMap<(OrderedF64, OrderedF64), Crossing> = BTreeMap::new();
    let vertical: Vec<usize> = (0..edges.len()).filter(|&i| edges[i].orientation == Orientation::Vertical).collect();
    let horizontal: Vec<usize> =
        (0..edges.len()).filter(|&i| edges[i].orientation == Orientation::Horizontal).collect();
    for &v in &vertical {
        let v_edge = &edges[v];
        for &h in &horizontal {
            let h_edge = &edges[h];
            if v_edge.top <= h_edge.top + INTERSECTION_TOLERANCE
                && v_edge.bottom >= h_edge.top - INTERSECTION_TOLERANCE
                && v_edge.x0 >= h_edge.x0 - INTERSECTION_TOLERANCE
                && v_edge.x0 <= h_edge.x1 + INTERSECTION_TOLERANCE
            {
                let crossing =
                    crossings.entry((OrderedF64(v_edge.x0 + 0.0), OrderedF64(h_edge.top + 0.0))).or_default();
                crossing.vertical.insert(ids[v]);
                crossing.horizontal.insert(ids[h]);
            }
        }
    }

    let points: Vec<(f64, f64)> = crossings.keys().map(|key| (key.0 .0, key.1 .0)).collect();
    let at = |point: (f64, f64)| crossings.get(&(OrderedF64(point.0), OrderedF64(point.1)));
    let connected = |p1: (f64, f64), p2: (f64, f64)| -> bool {
        let (Some(a), Some(b)) = (at(p1), at(p2)) else { return false };
        (p1.0 == p2.0 && !a.vertical.is_disjoint(&b.vertical))
            || (p1.1 == p2.1 && !a.horizontal.is_disjoint(&b.horizontal))
    };

    // The smallest cell whose top-left corner is each point.
    let mut cells: Vec<BBox> = Vec::new();
    for (index, &point) in points.iter().enumerate() {
        if index + 1 == points.len() {
            break;
        }
        let rest = &points[index + 1..];
        let below: Vec<(f64, f64)> = rest.iter().copied().filter(|other| other.0 == point.0).collect();
        let right: Vec<(f64, f64)> = rest.iter().copied().filter(|other| other.1 == point.1).collect();
        'search: for &below_point in &below {
            if !connected(point, below_point) {
                continue;
            }
            for &right_point in &right {
                if !connected(point, right_point) {
                    continue;
                }
                let corner = (right_point.0, below_point.1);
                if at(corner).is_some() && connected(corner, right_point) && connected(corner, below_point) {
                    cells.push([point.0, point.1, corner.0, corner.1]);
                    break 'search;
                }
            }
        }
    }

    cells_to_tables(&cells)
}

/// Group cells that share a corner into tables, top table first.
fn cells_to_tables(cells: &[BBox]) -> Vec<Table> {
    let corners = |cell: &BBox| {
        [vertex(cell[0], cell[1]), vertex(cell[0], cell[3]), vertex(cell[2], cell[1]), vertex(cell[2], cell[3])]
    };
    let mut remaining: Vec<BBox> = cells.to_vec();
    let mut current: Vec<BBox> = Vec::new();
    let mut current_corners: HashSet<Vertex> = HashSet::new();
    let mut groups: Vec<Vec<BBox>> = Vec::new();
    while !remaining.is_empty() {
        let before = current.len();
        let mut left_over = Vec::with_capacity(remaining.len());
        for cell in remaining {
            let cell_corners = corners(&cell);
            if current.is_empty() || cell_corners.iter().any(|corner| current_corners.contains(corner)) {
                current_corners.extend(cell_corners);
                current.push(cell);
            } else {
                left_over.push(cell);
            }
        }
        remaining = left_over;
        if current.len() == before {
            groups.push(std::mem::take(&mut current));
            current_corners.clear();
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }

    let first_corner = |group: &Vec<BBox>| {
        group
            .iter()
            .map(|cell| (cell[1], cell[0]))
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)))
            .unwrap_or((0.0, 0.0))
    };
    groups.sort_by(|a, b| {
        let (ka, kb) = (first_corner(a), first_corner(b));
        ka.0.total_cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
    });
    groups
        .into_iter()
        .filter(|group| group.len() > 1)
        .map(|cells| {
            let bbox =
                cells.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |acc, cell| {
                    [acc[0].min(cell[0]), acc[1].min(cell[1]), acc[2].max(cell[2]), acc[3].max(cell[3])]
                });
            Table { cells, bbox }
        })
        .collect()
}

/// A float usable as a map key: coordinates here are never NaN.
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedF64(f64);

impl Eq for OrderedF64 {}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn horizontal(x0: f64, x1: f64, y: f64) -> Edge {
        Edge { x0, x1, top: y, bottom: y, width: x1 - x0, height: 0.0, orientation: Orientation::Horizontal }
    }

    fn vertical(x: f64, top: f64, bottom: f64) -> Edge {
        Edge { x0: x, x1: x, top, bottom, width: 0.0, height: bottom - top, orientation: Orientation::Vertical }
    }

    fn grid(xs: &[f64], ys: &[f64]) -> Vec<Edge> {
        let mut edges: Vec<Edge> = ys.iter().map(|&y| horizontal(xs[0], xs[xs.len() - 1], y)).collect();
        edges.extend(xs.iter().map(|&x| vertical(x, ys[0], ys[ys.len() - 1])));
        edges
    }

    #[test]
    fn a_ruled_grid_is_one_table_of_its_cells() {
        let tables = find_tables(&grid(&[10.0, 60.0, 110.0], &[20.0, 40.0, 60.0]));
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].bbox, [10.0, 20.0, 110.0, 60.0]);
        let mut cells = tables[0].cells.clone();
        cells.sort_by(|a, b| a[1].total_cmp(&b[1]).then(a[0].total_cmp(&b[0])));
        assert_eq!(
            cells,
            vec![
                [10.0, 20.0, 60.0, 40.0],
                [60.0, 20.0, 110.0, 40.0],
                [10.0, 40.0, 60.0, 60.0],
                [60.0, 40.0, 110.0, 60.0]
            ]
        );
    }

    #[test]
    fn edges_a_hair_apart_are_snapped_to_their_average() {
        let mut edges = grid(&[10.0, 110.0], &[20.0, 60.0]);
        // A doubled middle rule, 0.48 pt thick: both sides of a filled border.
        edges.push(horizontal(10.0, 110.0, 40.0));
        edges.push(horizontal(10.0, 110.0, 40.48));
        let tables = find_tables(&edges);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].cells.len(), 2);
        assert!(tables[0].cells.iter().any(|cell| cell[3] == (40.0 + 40.48) / 2.0));
    }

    #[test]
    fn pieces_of_one_rule_are_joined() {
        let mut edges = vec![vertical(10.0, 20.0, 60.0), vertical(110.0, 20.0, 60.0)];
        for y in [20.0, 60.0] {
            edges.push(horizontal(10.0, 55.0, y));
            edges.push(horizontal(57.0, 110.0, y)); // a 2 pt gap in the rule
        }
        let tables = find_tables(&edges);
        // One cell is not a table; the joined rules make exactly one cell.
        assert!(tables.is_empty());
    }

    #[test]
    fn a_lone_cell_and_stray_marks_are_not_tables() {
        assert!(find_tables(&grid(&[10.0, 60.0], &[20.0, 40.0])).is_empty());
        assert!(find_tables(&[horizontal(0.0, 2.0, 5.0), vertical(3.0, 0.0, 0.5)]).is_empty());
    }

    #[test]
    fn grids_that_do_not_touch_are_separate_tables_top_first() {
        let mut edges = grid(&[10.0, 60.0, 110.0], &[300.0, 320.0]);
        edges.extend(grid(&[10.0, 60.0, 110.0], &[20.0, 40.0]));
        let tables = find_tables(&edges);
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].bbox[1], 20.0);
    }
}
