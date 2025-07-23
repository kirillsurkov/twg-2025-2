use std::{
    collections::{BinaryHeap, HashSet},
    f32::consts::E,
    ops::Div,
};

use bevy::{color::palettes::css::*, prelude::*};
use bevy_heightmap::HeightMap;
use fast_poisson::Poisson2D;
use imageproc::{
    distance_transform::euclidean_squared_distance_transform,
    drawing::{draw_filled_rect_mut, draw_line_segment_mut},
    filter,
    image::{GrayImage, ImageBuffer, Luma, LumaA, Pixel, Primitive, Rgb, Rgba},
    rect,
};
use kiddo::{KdTree, SquaredEuclidean};
use petgraph::{
    Graph, Undirected,
    algo::min_spanning_tree,
    data::Element,
    visit::{EdgeRef, IntoNodeReferences},
};
use rand::distr::{Distribution, weighted::WeightedIndex};

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

pub enum LevelBiome {
    Red,
    Green,
    Blue,
    Cyan,
    Magenta,
    Yellow,
    Orange,
    Purple,
}

pub struct LevelPart {
    points: Vec<Vec2>,
    edges: Vec<(usize, usize)>,
    bounds: Rect,
    pub radius: f32,
    biome: LevelBiome,
}

pub struct LevelPartBuilder {
    width: f32,
    height: f32,
    count: usize,
    fill_ratio: f32,
    biome: LevelBiome,
}

impl LevelPartBuilder {
    const GAP: Vec2 = Vec2::new(1.0, 1.0);

    pub fn new(biome: LevelBiome) -> Self {
        Self {
            width: 0.0,
            height: 0.0,
            count: 0,
            fill_ratio: 0.0,
            biome,
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

    fn estimate_radius(&self) -> f32 {
        (2.0 * self.width * self.height / (E * self.count as f32)).sqrt()
    }

    pub fn build(self) -> LevelPart {
        let radius = self.estimate_radius();
        let points = Poisson2D::new()
            .with_dimensions([self.width as f64, self.height as f64], radius as f64)
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

pub struct Level {
    kd: KdTree<f32, 2>,
    bounds: Rect,
    parts: Vec<LevelPart>,
    points: Vec<(usize, Vec2)>,
    edges: Vec<(usize, usize)>,
}

impl Level {
    const BLACK: Luma<u8> = Luma([0]);
    const WHITE: Luma<u8> = Luma([255]);

    pub fn new() -> Self {
        Self {
            kd: KdTree::new(),
            bounds: Rect {
                min: Vec2::MAX,
                max: Vec2::MIN,
            },
            parts: vec![],
            points: vec![],
            edges: vec![],
        }
    }

    pub fn add(&mut self, offset: Vec2, mut part: LevelPart) -> usize {
        let idx_offset = self.points.len();
        let points = part
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| (i + idx_offset, p + offset))
            .collect::<Vec<_>>();

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

        self.edges.extend(
            part.edges
                .iter()
                .map(|(start, end)| (start + idx_offset, end + idx_offset)),
        );

        let closest = points
            .iter()
            .flat_map(|(i, p)| {
                self.kd
                    .nearest_n::<SquaredEuclidean>(&[p.x, p.y], 2)
                    .into_iter()
                    .map(move |n| (n, *i))
            })
            .collect::<BinaryHeap<_>>()
            .into_sorted_vec();

        println!("{closest:#?}");

        for (neighbour, i) in closest.into_iter().take(2) {
            self.edges.push((i, neighbour.item as usize));
        }

        for (i, point) in &points {
            self.kd.add(&[point.x, point.y], *i as u64);
        }

        let id = self.parts.len();
        self.points.extend(points.into_iter().map(|(_, p)| (id, p)));
        self.parts.push(part);
        id
    }

    pub fn add_after(&mut self, after: usize, align: PartAlign, part: LevelPart) -> usize {
        let after = &self.parts[after];
        let edge = 0.5 * (after.bounds.size() + part.bounds.size());
        let offset = after.bounds.center() - part.bounds.center();
        let offset = match align {
            PartAlign::Left => offset - Vec2::new(edge.x, 0.0),
            PartAlign::Right => offset + Vec2::new(edge.x, 0.0),
            PartAlign::Up => offset + Vec2::new(0.0, edge.y),
            PartAlign::Down => offset + Vec2::new(0.0, edge.y),
        };
        self.add(offset, part)
    }

    pub fn points(&self) -> impl Iterator<Item = (&LevelPart, Vec2)> {
        self.points
            .iter()
            .map(|(i, p)| (&self.parts[*i], p.clone()))
    }

    pub fn edges(&self) -> impl Iterator<Item = (Vec2, Vec2)> {
        self.edges
            .iter()
            .map(|(start, end)| (self.points[*start].1, self.points[*end].1))
    }

    pub fn parts(&self) -> impl Iterator<Item = &LevelPart> {
        self.parts.iter()
    }

    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    fn height_map(&self, scale: f32) -> ImageBuffer<Luma<f32>, Vec<f32>> {
        let bounds = IRect {
            min: (self.bounds.min * scale).as_ivec2(),
            max: (self.bounds.max * scale).as_ivec2(),
        };

        let UVec2 {
            x: width,
            y: height,
        } = bounds.size().as_uvec2();

        let mut image = GrayImage::from_pixel(width, height, Self::BLACK);

        // for point in &self.points {
        //     let IVec2 { x, y } = (point * scale).as_ivec2() - bounds.min;
        //     image.put_pixel(x as u32, y as u32, Self::WHITE);
        // }

        for (start, end) in self.edges() {
            let start = start * scale - bounds.min.as_vec2();
            let end = end * scale - bounds.min.as_vec2();
            draw_line_segment_mut(&mut image, (start.x, start.y), (end.x, end.y), Self::WHITE);
        }

        let distances = euclidean_squared_distance_transform(&image);

        filter::gaussian_blur_f32(
            &ImageBuffer::from_fn(image.width(), image.height(), |x, y| {
                Luma([distances.get_pixel(x, y).0[0].sqrt() as f32 / scale])
            }),
            2.0,
        )
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
            pixel.0[match part.biome {
                LevelBiome::Red => BiomePixel::RED,
                LevelBiome::Green => BiomePixel::GREEN,
                LevelBiome::Blue => BiomePixel::BLUE,
                LevelBiome::Cyan => BiomePixel::CYAN,
                LevelBiome::Magenta => BiomePixel::MAGENTA,
                LevelBiome::Yellow => BiomePixel::YELLOW,
                LevelBiome::Orange => BiomePixel::ORANGE,
                LevelBiome::Purple => BiomePixel::PURPLE,
            }] = 1.0;

            draw_filled_rect_mut(
                &mut biomes,
                rect::Rect::at(x, y).of_size(width, height),
                pixel,
            );
        }

        filter::gaussian_blur_f32(&biomes, 8.0)
    }

    pub fn terrain(&self, scale: f32) -> Mesh {
        println!("heightmap");
        let height_map = self.height_map(scale);
        println!("biomemap");
        let biome_map = self.biome_map(scale);
        let bounds = (self.bounds.size() * scale).as_uvec2();

        println!("mesh");
        let mut mesh: Mesh = HeightMap {
            size: bounds,
            h: |coords| {
                let coords = ((Vec2::new(coords.x, -coords.y) + 0.5) * bounds.as_vec2())
                    .as_uvec2()
                    .min(bounds - 1);
                let dist = height_map.get_pixel(coords.x, coords.y).0[0];
                let biome = biome_map.get_pixel(coords.x, coords.y).0;

                let radius = biome[BiomePixel::RADIUS];

                let road_width = 0.25 * radius;
                let max_height = 0.5 * radius - road_width;

                3.0 * (dist - road_width).max(0.0).div(max_height).powf(0.75)
            },
        }
        .into();

        mesh.compute_smooth_normals();

        let choices = [RED, GREEN, BLUE, AQUA, MAGENTA, YELLOW, ORANGE, PURPLE];
        let mut rng = rand::rng();
        let mut colors = vec![];
        for y in (0..biome_map.height()).rev() {
            for x in 0..biome_map.width() {
                let pixel = biome_map.get_pixel(x, y);
                let dist = WeightedIndex::new(&pixel.0[1..=8]).unwrap();
                colors.push(choices[dist.sample(&mut rng)].to_f32_array());
            }
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

        mesh
    }
}

#[derive(Clone, Copy)]
struct BiomePixel([f32; Self::CHANNEL_COUNT as usize]);

impl Default for BiomePixel {
    fn default() -> Self {
        let mut data = [0.0; Self::CHANNEL_COUNT as usize];
        data[Self::RADIUS] = 1.0;
        data[Self::RED] = 1.0;
        Self(data)
    }
}

impl BiomePixel {
    const RADIUS: usize = 0;
    const RED: usize = 1;
    const GREEN: usize = 2;
    const BLUE: usize = 3;
    const CYAN: usize = 4;
    const MAGENTA: usize = 5;
    const YELLOW: usize = 6;
    const ORANGE: usize = 7;
    const PURPLE: usize = 8;
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
