use std::collections::HashSet;

use bevy::{color::palettes::css::GREEN, prelude::*};
use fast_poisson::Poisson2D;
use petgraph::{
    Graph, Undirected,
    algo::min_spanning_tree,
    data::Element,
    visit::{EdgeRef, IntoNodeReferences},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

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

fn graph(points: &Vec<Vec2>, ratio: f32) -> HashSet<UVec2> {
    let mut edges = HashSet::new();

    let delaunay = delaunay(points);
    let gabriel = gabriel(points, &delaunay);

    let mut all_edges = gabriel.edge_references().collect::<Vec<_>>();
    all_edges.sort_by(|e1, e2| e2.weight().partial_cmp(e1.weight()).unwrap());

    edges.extend(min_spanning_tree(&delaunay).filter_map(|e| match e {
        Element::Edge { source, target, .. } => Some(UVec2::new(source as u32, target as u32)),
        _ => None,
    }));

    let target_cnt = edges.len() + (ratio * (all_edges.len() - edges.len()) as f32) as usize;

    for edge in all_edges {
        if edges.len() >= target_cnt {
            break;
        }
        edges.insert(UVec2::new(
            edge.source().index() as u32,
            edge.target().index() as u32,
        ));
    }

    edges
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let points = Poisson2D::new()
        .with_dimensions([20.0, 20.0], 3.5)
        .iter()
        .map(|[x, y]| Vec2 {
            x: x as f32 - 10.0,
            y: y as f32 - 10.0,
        })
        .collect::<Vec<_>>();

    let edges = graph(&points, 0.2);

    for UVec2 { x: start, y: end } in edges {
        let start = points[start as usize];
        let end = points[end as usize];
        let mid = (start + end) * 0.5;
        let distance = start.distance(end);
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(1.0, 0.05)))),
            MeshMaterial3d(materials.add(Color::WHITE)),
            Transform::from_xyz(mid.x as f32, 0.0, mid.y as f32)
                .with_scale(Vec3::new(distance * 0.5, 1.0, 1.0))
                .with_rotation(Quat::from_rotation_y((end - start).angle_to(Vec2::X))),
        ));
    }

    for p in points {
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::ONE * 0.25))),
            MeshMaterial3d(materials.add(Color::WHITE)),
            Transform::from_xyz(p.x as f32, 0.0, p.y as f32),
        ));
    }

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::ONE * 11.0))),
        MeshMaterial3d(materials.add(StandardMaterial::from_color(GREEN))),
        Transform::from_xyz(0.0, -0.01, 0.0),
    ));

    // light
    commands.spawn((
        DirectionalLight {
            illuminance: 1000.0,
            ..Default::default()
        },
        Transform::default().looking_to(-Vec3::Y, Vec3::Y),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 30.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
