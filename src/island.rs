use std::f32::consts::TAU;

use crate::color::Color;
use crate::ray_intersect::Material;
use crate::world::VoxelWorld;

pub const ISLAND_RADIUS: i32 = 20;

const LAYERS: &[(i32, i32)] = &[
    (0, ISLAND_RADIUS),
    (-1, ISLAND_RADIUS - 1),
    (-2, ISLAND_RADIUS - 2),
    (-3, ISLAND_RADIUS - 4),
    (-4, ISLAND_RADIUS - 6),
    (-5, ISLAND_RADIUS - 9),
    (-6, ISLAND_RADIUS - 12),
];

pub fn build_base() -> VoxelWorld {
    let overworld = matte(Color::new(79, 151, 93));
    let nether = matte(Color::new(139, 58, 56));
    let end = matte(Color::new(202, 191, 112));
    let mut world = VoxelWorld::new();

    for &(y, radius) in LAYERS {
        for x in -radius..=radius {
            for z in -radius..=radius {
                if x * x + z * z > radius * radius {
                    continue;
                }
                if is_gap(x, z) {
                    continue;
                }

                let material = match sector(x, z) {
                    0 => overworld,
                    1 => nether,
                    _ => end,
                };
                world.place_block(x, y, z, material);
            }
        }
    }

    world
}

fn matte(color: Color) -> Material {
    Material::new(color, 0.9, 10.0, 0.0, 0.0, 1.0)
}

fn sector(x: i32, z: i32) -> usize {
    let angle = (z as f32).atan2(x as f32).rem_euclid(TAU);
    (angle / (TAU / 3.0)).floor() as usize
}

/// Deja tres cortes radiales de ancho visible para que los mundos sean islas
fn is_gap(x: i32, z: i32) -> bool {
    let radius = ((x * x + z * z) as f32).sqrt();
    let angle = (z as f32).atan2(x as f32).rem_euclid(TAU);
    let wedge = TAU / 3.0;
    let distance_to_boundary = (angle % wedge).min(wedge - angle % wedge);
    radius * distance_to_boundary.sin().abs() < 1.05
}
