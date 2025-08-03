use std::{
    collections::{BinaryHeap, HashMap},
    f32::consts::E,
};

use bevy::prelude::*;
use fast_poisson::Poisson2D;
use imageproc::{
    distance_transform::euclidean_squared_distance_transform,
    drawing::{draw_filled_rect_mut, draw_line_segment_mut},
    filter,
    image::{
        GrayImage, ImageBuffer, Luma, LumaA, Pixel, Primitive, Rgb, Rgba, imageops::sample_bilinear,
    },
    rect,
};
use kiddo::{KdTree, SquaredEuclidean};
use petgraph::{
    Graph, Undirected,
    algo::min_spanning_tree,
    data::Element,
    graph::NodeIndex,
    visit::{EdgeRef, IntoNodeReferences},
};

fn delaunay(points: Vec<Vec2>) -> Graph<Vec2, f32, Undirected> {
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
        .into_iter()
        .map(|p| graph.add_node(p))
        .collect::<Vec<_>>();

    for i in 0..triangulation.halfedges.len() {
        if i <= triangulation.halfedges[i] && triangulation.halfedges[i] != delaunator::EMPTY {
            continue;
        }

        let start_idx = triangulation.triangles[i];
        let end_idx = triangulation.triangles[delaunator::next_halfedge(i)];

        let start_node = nodes[start_idx];
        let end_node = nodes[end_idx];

        let weight = graph
            .node_weight(start_node)
            .unwrap()
            .distance(*graph.node_weight(end_node).unwrap());

        graph.add_edge(start_node, end_node, weight);
    }

    graph
}

fn gabriel(delaunay: &Graph<Vec2, f32, Undirected>) -> Graph<Vec2, f32, Undirected> {
    let mut graph = delaunay.clone();
    let mut to_remove = vec![];
    for edge in graph.edge_references() {
        let p1 = delaunay.node_weight(edge.source()).unwrap();
        let p2 = delaunay.node_weight(edge.target()).unwrap();
        let mid = (p1 + p2) * 0.5;
        let radius = p1.distance(*p2) * 0.5;
        for (node, point) in graph.node_references() {
            if [edge.source(), edge.target()].contains(&node) {
                continue;
            }
            if mid.distance(*point) <= radius {
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

fn graph(points: Vec<Vec2>, ratio: f32) -> Graph<Vec2, f32, Undirected> {
    let mut graph = delaunay(points);
    let gabriel = gabriel(&graph);
    let mst = min_spanning_tree(&graph).collect::<Vec<_>>();
    graph.clear_edges();

    graph.extend_with_edges(mst.into_iter().filter_map(|e| match e {
        Element::Edge {
            source,
            target,
            weight,
        } => Some((NodeIndex::new(source), NodeIndex::new(target), weight)),
        _ => None,
    }));

    let mut all_edges = gabriel.edge_references().collect::<Vec<_>>();
    all_edges.sort_by(|e1, e2| e2.weight().partial_cmp(e1.weight()).unwrap());

    let target_cnt =
        graph.edge_count() + (ratio * (all_edges.len() - graph.edge_count()) as f32) as usize;

    for edge in all_edges {
        if graph.edge_count() >= target_cnt {
            break;
        }
        graph.update_edge(edge.source(), edge.target(), *edge.weight());
    }

    graph
}

pub enum LevelBiome {
    Safe,
    Home,
    Forest,
    Cave,
    Ice,
    Temple,
    Boss,
}

impl LevelBiome {
    fn to_pixel_channel(&self) -> usize {
        match self {
            Self::Safe => BiomePixel::AREA_SAFE,
            Self::Home => BiomePixel::AREA_HOME,
            Self::Forest => BiomePixel::AREA_FOREST,
            Self::Cave => BiomePixel::AREA_CAVE,
            Self::Ice => BiomePixel::AREA_ICE,
            Self::Temple => BiomePixel::AREA_TEMPLE,
            Self::Boss => BiomePixel::AREA_BOSS,
        }
    }
}

pub struct LevelPart {
    graph: Graph<Vec2, f32, Undirected>,
    bounds: Rect,
    radius: f32,
    biome: LevelBiome,
}

pub struct LevelPartBuilder {
    width: f32,
    height: f32,
    count: usize,
    fill_ratio: f32,
    biome: LevelBiome,
    points: Option<Vec<Vec2>>,
}

impl LevelPartBuilder {
    const GAP: Vec2 = Vec2::new(25.0, 25.0);

    pub fn new(biome: LevelBiome) -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            count: 0,
            fill_ratio: 0.0,
            biome,
            points: None,
        }
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
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

    pub fn with_points(mut self, points: Vec<Vec2>) -> Self {
        self.points = Some(points);
        self
    }

    fn estimate_radius(&self) -> f32 {
        (2.0 * self.width * self.height / (E * self.count as f32)).sqrt()
    }

    pub fn build(self) -> LevelPart {
        let radius = self.estimate_radius();
        let points = match self.points {
            Some(points) => points,
            None => Poisson2D::new()
                .with_dimensions([self.width as f64, self.height as f64], radius as f64)
                .iter()
                .map(|[x, y]| Vec2 {
                    x: x as f32 - 0.5 * self.width as f32,
                    y: y as f32 - 0.5 * self.height as f32,
                })
                .collect::<Vec<_>>(),
        };

        let bounds = Rect::from_center_size(
            Vec2::ZERO,
            Vec2::new(self.width as f32, self.height as f32) + Self::GAP * 2.0,
        );

        LevelPart {
            graph: graph(points, self.fill_ratio),
            bounds,
            radius,
            biome: self.biome,
        }
    }
}

pub enum PartAlign {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Resource)]
pub struct Level {
    pub graph: Graph<Vec2, f32, Undirected>,
    kd_terrain: KdTree<f32, 2>,
    kd_creatures: KdTree<f32, 3>,
    bounds: Rect,
    scale: f32,
    biome_map: ImageBuffer<BiomePixel, Vec<f32>>,
    height_map: ImageBuffer<Luma<f32>, Vec<f32>>,
    normal_map: ImageBuffer<Rgb<f32>, Vec<f32>>,
}

impl Level {
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn texture_size(&self) -> Vec2 {
        self.scale * self.bounds.size()
    }

    pub fn pixel_size(&self) -> f32 {
        1.0 / self.scale
    }

    pub fn world_to_uv(&self, world_pos: Vec2) -> Vec2 {
        (world_pos - self.bounds.min).clamp(Vec2::ZERO, self.bounds.size()) / self.bounds.size()
    }

    pub fn world_to_texture(&self, world_pos: Vec2) -> Vec2 {
        ((world_pos - self.bounds.min) * self.scale).clamp(Vec2::ZERO, self.texture_size() - 1.0)
    }

    pub fn biome(&self, world_pos: Vec2) -> BiomePixel {
        let pos = self.world_to_texture(world_pos).as_uvec2();
        *self.biome_map.get_pixel(pos.x, pos.y)
    }

    pub fn height(&self, world_pos: Vec2) -> f32 {
        let pos = self.world_to_uv(world_pos);
        sample_bilinear(&self.height_map, pos.x, pos.y).unwrap().0[0]
    }

    pub fn normal_3d(&self, world_pos: Vec2) -> Vec3 {
        let pos = self.world_to_uv(world_pos);
        if sample_bilinear(&self.height_map, pos.x, pos.y).unwrap().0[0] <= 0.0 {
            Vec3::Y
        } else {
            Vec3::from(sample_bilinear(&self.normal_map, pos.x, pos.y).unwrap().0)
        }
    }

    pub fn normal_2d(&self, world_pos: Vec2) -> Vec2 {
        let pos = self.world_to_uv(world_pos);
        let [x, _, z] = sample_bilinear(&self.normal_map, pos.x, pos.y).unwrap().0;
        Vec2::new(x, z).normalize_or_zero()
    }

    pub fn binary_search(&self, mut min: Vec3, mut max: Vec3, binary_steps: u32) -> Vec3 {
        for _ in 0..binary_steps {
            let mid = (min + max) / 2.0;
            if self.height(mid.xz()).max(0.0) < mid.y {
                min = mid;
            } else {
                max = mid;
            }
        }
        max
    }

    pub fn raycast(
        &self,
        origin: Vec3,
        dir: Dir3,
        step: f32,
        max_dist: f32,
        binary_steps: u32,
    ) -> Vec3 {
        let mut pos = origin;
        while pos.distance_squared(origin) < max_dist * max_dist {
            if self.height(pos.xz()).max(0.0) > pos.y {
                return self.binary_search(pos - dir * step, pos, binary_steps);
            }
            pos += dir * step;
        }
        pos
    }

    pub fn can_walk(&self, mut from: Vec2, to: Vec2, radius: f32) -> bool {
        let Some(dir) = (to - from).try_normalize() else {
            return true;
        };
        loop {
            let max_step = -self.height(from);
            if from.distance(to) <= max_step {
                break true;
            }
            if max_step < radius {
                break false;
            }
            from += dir * max_step;
        }
    }

    pub fn nearest_id_terrain(&self, count: usize, point: Vec2) -> Vec<NodeIndex> {
        self.kd_terrain
            .nearest_n::<SquaredEuclidean>(&point.to_array(), count)
            .into_iter()
            .map(|neighbour| NodeIndex::new(neighbour.item as usize))
            .collect()
    }

    pub fn nearest_terrain(&self, count: usize, point: Vec2) -> Vec<Option<Vec2>> {
        self.nearest_id_terrain(count, point)
            .into_iter()
            .map(|node| self.graph.node_weight(node).cloned())
            .collect()
    }

    pub fn clear_creatures(&mut self) {
        self.kd_creatures = KdTree::new();
    }

    pub fn add_creature(&mut self, creature: Entity, pos: Vec3) {
        self.kd_creatures.add(&pos.to_array(), creature.to_bits());
    }

    pub fn nearest_creatures(&self, count: usize, point: Vec3) -> Vec<(Entity, f32)> {
        self.kd_creatures
            .nearest_n::<SquaredEuclidean>(&point.to_array(), count)
            .into_iter()
            .map(|neighbour| (Entity::from_bits(neighbour.item), neighbour.distance))
            .collect()
    }
}

pub struct LevelBuilder {
    graph: Graph<Vec2, f32, Undirected>,
    kd_terrain: KdTree<f32, 2>,
    bounds: Rect,
    parts: Vec<LevelPart>,
}

impl LevelBuilder {
    const BLACK: Luma<u8> = Luma([0]);
    const WHITE: Luma<u8> = Luma([255]);

    pub fn new() -> Self {
        Self {
            graph: Graph::new_undirected(),
            kd_terrain: KdTree::new(),
            bounds: Rect {
                min: Vec2::MAX,
                max: Vec2::MIN,
            },
            parts: vec![],
        }
    }

    pub fn add(&mut self, offset: Vec2, mut part: LevelPart) -> usize {
        let idx_offset = self.graph.node_count();

        part.bounds = Rect {
            min: part.bounds.min + offset,
            max: part.bounds.max + offset,
        };

        self.bounds = Rect::new(
            self.bounds.min.x.min(part.bounds.min.x),
            self.bounds.min.y.min(part.bounds.min.y),
            self.bounds.max.x.max(part.bounds.max.x),
            self.bounds.max.y.max(part.bounds.max.y),
        );

        for point in part.graph.node_weights_mut() {
            *point += offset;
            self.graph.add_node(*point);
        }

        self.graph
            .extend_with_edges(part.graph.edge_references().map(|e| {
                let source = e.source().index() + idx_offset;
                let target = e.target().index() + idx_offset;
                (NodeIndex::new(source), NodeIndex::new(target), e.weight())
            }));

        let closest = part
            .graph
            .node_references()
            .flat_map(|(node, point)| {
                self.kd_terrain
                    .nearest_n::<SquaredEuclidean>(&[point.x, point.y], 2)
                    .into_iter()
                    .map(move |neighbour| (neighbour, node.index() + idx_offset))
            })
            .collect::<BinaryHeap<_>>()
            .into_sorted_vec();

        for (neighbour, node) in closest.into_iter().take(2) {
            self.graph.add_edge(
                NodeIndex::new(node),
                NodeIndex::new(neighbour.item as usize),
                neighbour.distance.sqrt(),
            );
        }

        for edge in part.graph.edge_references() {
            let source = part.graph.node_weight(edge.source()).unwrap();
            let target = part.graph.node_weight(edge.target()).unwrap();
            let dist2 = source.distance_squared(*target);
            let step = (target - source).normalize_or_zero() * 5.0;
            let mut point = *source;
            loop {
                let dist2_source = point.distance_squared(*source);
                let dist2_target = point.distance_squared(*target);
                if dist2_source >= dist2 {
                    break;
                }
                let node = if dist2_source < dist2_target {
                    edge.source()
                } else {
                    edge.target()
                };
                let node = (node.index() + idx_offset) as u64;
                self.kd_terrain.add(&[point.x, point.y], node);
                point += step;
            }
            self.kd_terrain.add(
                &[target.x, target.y],
                (edge.target().index() + idx_offset) as u64,
            );
        }

        self.parts.push(part);
        self.parts.len() - 1
    }

    pub fn add_after(&mut self, after: usize, align: PartAlign, part: LevelPart) -> usize {
        let after = &self.parts[after];
        let edge = 0.5 * (after.bounds.size() + part.bounds.size());
        let offset = after.bounds.center() - part.bounds.center();
        let offset = match align {
            PartAlign::Left => offset - Vec2::new(edge.x, 0.0),
            PartAlign::Right => offset + Vec2::new(edge.x, 0.0),
            PartAlign::Up => offset + Vec2::new(0.0, edge.y),
            PartAlign::Down => offset - Vec2::new(0.0, edge.y),
        };
        self.add(offset, part)
    }

    fn biome_map(&self, scale: f32) -> ImageBuffer<BiomePixel, Vec<f32>> {
        let bounds = IRect {
            min: (self.bounds.min * scale).as_ivec2(),
            max: (self.bounds.max * scale).as_ivec2(),
        };

        let UVec2 {
            x: width,
            y: height,
        } = bounds.size().as_uvec2();

        let mut biomes =
            ImageBuffer::<BiomePixel, Vec<f32>>::from_pixel(width, height, BiomePixel::default());

        for part in &self.parts {
            let IVec2 { x, y } = (part.bounds.min * scale).as_ivec2() - bounds.min;
            let UVec2 {
                x: width,
                y: height,
            } = (part.bounds.size() * scale).as_uvec2();

            let mut pixel = BiomePixel([0.0; BiomePixel::CHANNEL_COUNT as usize]);
            pixel.0[BiomePixel::RADIUS] = part.radius;
            pixel.0[part.biome.to_pixel_channel()] = 1.0;

            draw_filled_rect_mut(
                &mut biomes,
                rect::Rect::at(x, y).of_size(width, height),
                pixel,
            );
        }

        filter::gaussian_blur_f32(&biomes, 8.0)
    }

    fn height_map(
        &self,
        scale: f32,
        biome_map: &ImageBuffer<BiomePixel, Vec<f32>>,
    ) -> ImageBuffer<Luma<f32>, Vec<f32>> {
        let bounds = IRect {
            min: (self.bounds.min * scale).as_ivec2(),
            max: (self.bounds.max * scale).as_ivec2(),
        };

        let UVec2 {
            x: width,
            y: height,
        } = bounds.size().as_uvec2();

        let mut image = GrayImage::from_pixel(width, height, Self::BLACK);

        for edge in self.graph.edge_references() {
            let source = self.graph.node_weight(edge.source()).unwrap();
            let source = source * scale - bounds.min.as_vec2();

            let target = self.graph.node_weight(edge.target()).unwrap();
            let target = target * scale - bounds.min.as_vec2();

            draw_line_segment_mut(
                &mut image,
                (source.x, source.y),
                (target.x, target.y),
                Self::WHITE,
            );
        }

        let distances = euclidean_squared_distance_transform(&image);

        ImageBuffer::from_fn(image.width(), image.height(), |x, y| {
            let dist = distances.get_pixel(x, y).0[0].sqrt() as f32 / scale;
            let biome = biome_map.get_pixel(x, y).0;

            let radius = biome[BiomePixel::RADIUS];

            let road_width = 0.25 * radius;
            let max_height = 0.5 * radius - road_width;

            Luma([if dist < road_width {
                dist - road_width
            } else {
                3.0 * (dist - road_width) / max_height.powf(0.75)
            }])
        })
    }

    fn normal_map(
        &self,
        scale: f32,
        height_map: &ImageBuffer<Luma<f32>, Vec<f32>>,
    ) -> ImageBuffer<Rgb<f32>, Vec<f32>> {
        let (width, height) = height_map.dimensions();
        let max_pos = UVec2::new(width, height).as_ivec2() - 1;
        // let height_map = filter::gaussian_blur_f32(&height_map, 2.0);
        ImageBuffer::from_fn(width, height, |x, y| {
            let pos = UVec2::new(x, y).as_ivec2();

            let pos_r = IVec2::new(pos.x + 1, pos.y).min(max_pos).as_uvec2();
            let pos_l = IVec2::new(pos.x - 1, pos.y).max(IVec2::ZERO).as_uvec2();
            let pos_t = IVec2::new(pos.x, pos.y + 1).min(max_pos).as_uvec2();
            let pos_b = IVec2::new(pos.x, pos.y - 1).max(IVec2::ZERO).as_uvec2();

            let h_r = height_map.get_pixel(pos_r.x, pos_r.y).0[0];
            let h_l = height_map.get_pixel(pos_l.x, pos_l.y).0[0];
            let h_t = height_map.get_pixel(pos_t.x, pos_t.y).0[0];
            let h_b = height_map.get_pixel(pos_b.x, pos_b.y).0[0];

            let dh_dx = (h_r - h_l) * scale * 0.5;
            let dh_dy = (h_t - h_b) * scale * 0.5;

            Rgb(Vec3::new(-dh_dx, 1.0, -dh_dy)
                .normalize_or_zero()
                .to_array())
        })
    }

    pub fn build(self, scale: f32) -> Level {
        let biome_map = self.biome_map(scale);
        let height_map = self.height_map(scale, &biome_map);
        let normal_map = self.normal_map(scale, &height_map);
        Level {
            graph: self.graph,
            kd_terrain: self.kd_terrain,
            kd_creatures: KdTree::new(),
            bounds: self.bounds,
            scale,
            height_map,
            biome_map,
            normal_map,
        }
    }
}

#[derive(Clone, Copy)]
pub struct BiomePixel(pub [f32; Self::CHANNEL_COUNT as usize]);

impl Default for BiomePixel {
    fn default() -> Self {
        let mut data = [0.0; Self::CHANNEL_COUNT as usize];
        data[Self::RADIUS] = 1.0;
        data[Self::AREA_FOREST] = 1.0;
        Self(data)
    }
}

impl BiomePixel {
    pub const RADIUS: usize = 0;

    pub const START_BIOME: usize = 1;
    pub const AREA_SAFE: usize = 1;
    pub const AREA_HOME: usize = 2;
    pub const AREA_FOREST: usize = 3;
    pub const AREA_CAVE: usize = 4;
    pub const AREA_ICE: usize = 5;
    pub const AREA_TEMPLE: usize = 6;
    pub const AREA_BOSS: usize = 7;
    pub const END_BIOME: usize = 8;
}

impl Pixel for BiomePixel {
    type Subpixel = f32;

    const CHANNEL_COUNT: u8 = 10;
    const COLOR_MODEL: &'static str = "BIOME";

    fn channels(&self) -> &[Self::Subpixel] {
        &self.0
    }

    fn channels_mut(&mut self) -> &mut [Self::Subpixel] {
        &mut self.0
    }

    fn channels4(
        &self,
    ) -> (
        Self::Subpixel,
        Self::Subpixel,
        Self::Subpixel,
        Self::Subpixel,
    ) {
        let mut channels = [Self::Subpixel::DEFAULT_MAX_VALUE; 4];
        channels[0..Self::CHANNEL_COUNT as usize].copy_from_slice(&self.0);
        (channels[0], channels[1], channels[2], channels[3])
    }

    fn from_channels(
        a: Self::Subpixel,
        b: Self::Subpixel,
        c: Self::Subpixel,
        d: Self::Subpixel,
    ) -> Self {
        *<Self as Pixel>::from_slice(&[a, b, c, d][..Self::CHANNEL_COUNT as usize])
    }

    fn from_slice(slice: &[Self::Subpixel]) -> &Self {
        assert_eq!(slice.len(), Self::CHANNEL_COUNT as usize);
        unsafe { &*(slice.as_ptr() as *const Self) }
    }

    fn from_slice_mut(slice: &mut [Self::Subpixel]) -> &mut Self {
        assert_eq!(slice.len(), Self::CHANNEL_COUNT as usize);
        unsafe { &mut *(slice.as_mut_ptr() as *mut Self) }
    }

    fn to_rgb(&self) -> Rgb<Self::Subpixel> {
        Rgb([0.0, 0.0, 0.0])
    }

    fn to_rgba(&self) -> Rgba<Self::Subpixel> {
        Rgba([0.0, 0.0, 0.0, 0.0])
    }

    fn to_luma(&self) -> Luma<Self::Subpixel> {
        Luma([0.0])
    }

    fn to_luma_alpha(&self) -> LumaA<Self::Subpixel> {
        LumaA([0.0, 0.0])
    }

    fn map<F>(&self, f: F) -> Self
    where
        F: FnMut(Self::Subpixel) -> Self::Subpixel,
    {
        let mut this = (*self).clone();
        this.apply(f);
        this
    }

    fn apply<F>(&mut self, mut f: F)
    where
        F: FnMut(Self::Subpixel) -> Self::Subpixel,
    {
        for v in &mut self.0 {
            *v = f(*v)
        }
    }

    fn map_with_alpha<F, G>(&self, f: F, g: G) -> Self
    where
        F: FnMut(Self::Subpixel) -> Self::Subpixel,
        G: FnMut(Self::Subpixel) -> Self::Subpixel,
    {
        let mut this = (*self).clone();
        this.apply_with_alpha(f, g);
        this
    }

    fn apply_with_alpha<F, G>(&mut self, mut f: F, mut g: G)
    where
        F: FnMut(Self::Subpixel) -> Self::Subpixel,
        G: FnMut(Self::Subpixel) -> Self::Subpixel,
    {
        let alpha = Self::CHANNEL_COUNT as usize;
        for v in self.0[..alpha].iter_mut() {
            *v = f(*v)
        }
        if let Some(v) = self.0.get_mut(alpha) {
            *v = g(*v)
        }
    }

    fn map2<F>(&self, other: &Self, f: F) -> Self
    where
        F: FnMut(Self::Subpixel, Self::Subpixel) -> Self::Subpixel,
    {
        let mut this = (*self).clone();
        this.apply2(other, f);
        this
    }

    fn apply2<F>(&mut self, other: &Self, mut f: F)
    where
        F: FnMut(Self::Subpixel, Self::Subpixel) -> Self::Subpixel,
    {
        for (a, &b) in self.0.iter_mut().zip(other.0.iter()) {
            *a = f(*a, b)
        }
    }

    fn invert(&mut self) {}

    fn blend(&mut self, other: &Self) {
        *self = *other;
    }
}
