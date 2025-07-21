use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashSet},
    f64::consts::E,
};

use bevy::prelude::*;
use fast_poisson::Poisson2D;
use kiddo::{KdTree, SquaredEuclidean};
use petgraph::{
    Graph, Undirected,
    algo::min_spanning_tree,
    data::Element,
    visit::{EdgeRef, IntoNodeReferences},
};

fn delaunay(points: &Vec<Vec2>) -> Graph<(), f32, Undirected> {
    let mut graph = Graph::new_undirected();

    let triangulation = delaunator::triangulate(
        &points
            .iter()
            .map(|p| delaunator::Point {
                x: p.x as f64,
                y: p.y as f64,
            })
            .collect::<Vec<_>>(),
    );

    let nodes = points
        .iter()
        .map(|_| graph.add_node(()))
        .collect::<Vec<_>>();

    for i in 0..triangulation.halfedges.len() {
        if i <= triangulation.halfedges[i] && triangulation.halfedges[i] != delaunator::EMPTY {
            continue;
        }

        let start_idx = triangulation.triangles[i];
        let end_idx = triangulation.triangles[delaunator::next_halfedge(i)];

        graph.add_edge(
            nodes[start_idx],
            nodes[end_idx],
            points[start_idx].distance(points[end_idx]),
        );
    }

    graph
}

fn gabriel(
    points: &Vec<Vec2>,
    delaunay: &Graph<(), f32, Undirected>,
) -> Graph<(), f32, Undirected> {
    let mut graph = delaunay.clone();
    let mut to_remove = vec![];
    for edge in graph.edge_references() {
        let p1 = points[edge.source().index()];
        let p2 = points[edge.target().index()];
        let mid = (p1 + p2) * 0.5;
        let radius = p1.distance(p2) * 0.5;
        for (node, _) in graph.node_references() {
            if [edge.source(), edge.target()].contains(&node) {
                continue;
            }
            if mid.distance(points[node.index()]) <= radius {
                to_remove.push(edge.id());
                break;
            }
        }
    }
    for index in to_remove.into_iter().rev() {
        graph.remove_edge(index);
    }
    graph
}

fn graph(points: &Vec<Vec2>, ratio: f32) -> Vec<(usize, usize)> {
    let mut edges = HashSet::new();

    let delaunay = delaunay(points);
    let gabriel = gabriel(points, &delaunay);

    let mut all_edges = gabriel.edge_references().collect::<Vec<_>>();
    all_edges.sort_by(|e1, e2| e2.weight().partial_cmp(e1.weight()).unwrap());

    edges.extend(min_spanning_tree(&delaunay).filter_map(|e| match e {
        Element::Edge { source, target, .. } => Some((source, target)),
        _ => None,
    }));

    let target_cnt = edges.len() + (ratio * (all_edges.len() - edges.len()) as f32) as usize;

    for edge in all_edges {
        if edges.len() >= target_cnt {
            break;
        }
        edges.insert((edge.source().index(), edge.target().index()));
    }

    edges.into_iter().collect()
}

pub struct LevelPart {
    points: Vec<Vec2>,
    edges: Vec<(usize, usize)>,
    bounds: Rect,
}

impl LevelPart {}

pub struct LevelPartBuilder {
    width: f64,
    height: f64,
    count: usize,
    fill_ratio: f32,
}

impl LevelPartBuilder {
    const GAP: Vec2 = Vec2::new(1.0, 1.0);

    pub fn new() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            count: 0,
            fill_ratio: 0.0,
        }
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = width as f64;
        self.height = height as f64;
        self
    }

    pub fn with_count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    pub fn with_fill_ratio(mut self, fill_ratio: f32) -> Self {
        self.fill_ratio = fill_ratio;
        self
    }

    fn estimate_radius(&self) -> f64 {
        (2.0 * self.width * self.height / (E * self.count as f64)).sqrt()
    }

    pub fn build(&self) -> LevelPart {
        let points = Poisson2D::new()
            .with_dimensions([self.width, self.height], self.estimate_radius())
            .iter()
            .map(|[x, y]| Vec2 {
                x: x as f32 - 0.5 * self.width as f32,
                y: y as f32 - 0.5 * self.height as f32,
            })
            .collect::<Vec<_>>();

        let edges = graph(&points, self.fill_ratio);

        let bounds = Rect::from_center_size(
            Vec2::ZERO,
            Vec2::new(self.width as f32, self.height as f32) + Self::GAP * 2.0,
        );

        LevelPart {
            points,
            edges,
            bounds,
        }
    }
}

pub enum PartAlign {
    Left,
    Right,
    Up,
    Down,
}

pub struct Level {
    kd: KdTree<f32, 2>,
    bounds: Vec<Rect>,
    points: Vec<Vec2>,
    edges: Vec<(usize, usize)>,
}

impl Level {
    pub fn new() -> Self {
        Self {
            kd: KdTree::new(),
            bounds: vec![],
            points: vec![],
            edges: vec![],
        }
    }

    pub fn add(&mut self, offset: Vec2, part: LevelPart) -> usize {
        let idx_offset = self.points.len();
        let points = part
            .points
            .into_iter()
            .enumerate()
            .map(|(i, p)| (i + idx_offset, p + offset))
            .collect::<Vec<_>>();

        self.edges.extend(
            part.edges
                .into_iter()
                .map(|(start, end)| (start + idx_offset, end + idx_offset)),
        );

        let mut heap = points
            .iter()
            .flat_map(|(i, p)| {
                self.kd
                    .nearest_n::<SquaredEuclidean>(&[p.x, p.y], 2)
                    .into_iter()
                    .map(move |n| Reverse((n, *i)))
            })
            .collect::<BinaryHeap<_>>();

        for _ in 0..2 {
            if let Some(Reverse((neighbour, i))) = heap.pop() {
                self.edges.push((i, neighbour.item as usize))
            }
        }

        for (i, point) in &points {
            self.kd.add(&[point.x, point.y], *i as u64);
        }

        self.points.extend(points.into_iter().map(|(_, p)| p));

        self.bounds.push(Rect {
            min: part.bounds.min + offset,
            max: part.bounds.max + offset,
        });

        self.bounds.len() - 1
    }

    pub fn add_after(&mut self, after: usize, align: PartAlign, part: LevelPart) -> usize {
        let after = self.bounds[after];
        let edge = 0.5 * (after.size() + part.bounds.size());
        let offset = after.center() - part.bounds.center();
        let offset = match align {
            PartAlign::Left => offset - Vec2::new(edge.x, 0.0),
            PartAlign::Right => offset + Vec2::new(edge.x, 0.0),
            PartAlign::Up => offset + Vec2::new(0.0, edge.y),
            PartAlign::Down => offset + Vec2::new(0.0, edge.y),
        };
        self.add(offset, part)
    }

    pub fn points(&self) -> impl Iterator<Item = Vec2> {
        self.points.iter().cloned()
    }

    pub fn edges(&self) -> impl Iterator<Item = (Vec2, Vec2)> {
        self.edges
            .iter()
            .map(|(start, end)| (self.points[*start], self.points[*end]))
    }

    pub fn bounds(&self) -> impl Iterator<Item = Rect> {
        self.bounds.iter().cloned()
    }
}
