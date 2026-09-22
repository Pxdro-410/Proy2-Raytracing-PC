use std::collections::HashMap;

use crate::ray_intersect::{BlockFace, Intersect, Material, RayIntersect};
use crate::vec3::Vec3;

const EPSILON: f32 = 1e-4;
type BlockPosition = (i32, i32, i32);

#[derive(Clone, Copy)]
struct Bounds {
    min: BlockPosition,
    max: BlockPosition,
}

/// Mundo de bloques indexado por coordenadas enteras.
/// Un rayo avanza por la cuadrícula con DDA, en vez de comparar cada bloque.
pub struct VoxelWorld {
    blocks: HashMap<BlockPosition, Material>,
    bounds: Option<Bounds>,
}

impl VoxelWorld {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            bounds: None,
        }
    }

    pub fn place_block(&mut self, x: i32, y: i32, z: i32, material: Material) {
        self.blocks.insert((x, y, z), material);
        self.bounds = Some(match self.bounds {
            Some(bounds) => Bounds {
                min: (
                    bounds.min.0.min(x),
                    bounds.min.1.min(y),
                    bounds.min.2.min(z),
                ),
                max: (
                    bounds.max.0.max(x),
                    bounds.max.1.max(y),
                    bounds.max.2.max(z),
                ),
            },
            None => Bounds {
                min: (x, y, z),
                max: (x, y, z),
            },
        });
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
}

impl RayIntersect for VoxelWorld {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<Intersect> {
        let bounds = self.bounds?;
        let (entry, exit, mut normal) = intersect_bounds(*ray_origin, *ray_direction, bounds)?;
        if exit < EPSILON {
            return None;
        }

        let mut distance = entry.max(EPSILON);
        let start = *ray_origin + *ray_direction * (distance + EPSILON);
        let mut cell = (
            start.x.floor() as i32,
            start.y.floor() as i32,
            start.z.floor() as i32,
        );

        if entry < EPSILON {
            normal = -*ray_direction;
        }

        let step_x = ray_direction.x.signum() as i32;
        let step_y = ray_direction.y.signum() as i32;
        let step_z = ray_direction.z.signum() as i32;

        let mut next_x = next_boundary(cell.0, ray_origin.x, ray_direction.x, step_x);
        let mut next_y = next_boundary(cell.1, ray_origin.y, ray_direction.y, step_y);
        let mut next_z = next_boundary(cell.2, ray_origin.z, ray_direction.z, step_z);
        let delta_x = axis_delta(ray_direction.x);
        let delta_y = axis_delta(ray_direction.y);
        let delta_z = axis_delta(ray_direction.z);

        while distance <= exit + EPSILON {
            if let Some(material) = self.blocks.get(&cell) {
                let point = *ray_origin + *ray_direction * distance;
                let face = BlockFace::from_normal(normal);
                return Some(Intersect {
                    point,
                    normal,
                    distance,
                    material: *material,
                    face,
                    uv: face.uv(
                        point,
                        Vec3::new(cell.0 as f32, cell.1 as f32, cell.2 as f32),
                    ),
                });
            }

            if next_x <= next_y && next_x <= next_z {
                distance = next_x;
                next_x += delta_x;
                cell.0 += step_x;
                normal = Vec3::new(-(step_x as f32), 0.0, 0.0);
            } else if next_y <= next_z {
                distance = next_y;
                next_y += delta_y;
                cell.1 += step_y;
                normal = Vec3::new(0.0, -(step_y as f32), 0.0);
            } else {
                distance = next_z;
                next_z += delta_z;
                cell.2 += step_z;
                normal = Vec3::new(0.0, 0.0, -(step_z as f32));
            }
        }

        None
    }
}

fn intersect_bounds(origin: Vec3, direction: Vec3, bounds: Bounds) -> Option<(f32, f32, Vec3)> {
    let min = Vec3::new(
        bounds.min.0 as f32,
        bounds.min.1 as f32,
        bounds.min.2 as f32,
    );
    let max = Vec3::new(
        bounds.max.0 as f32 + 1.0,
        bounds.max.1 as f32 + 1.0,
        bounds.max.2 as f32 + 1.0,
    );

    let mut entry = f32::NEG_INFINITY;
    let mut exit = f32::INFINITY;
    let mut entry_normal = Vec3::new(0.0, 1.0, 0.0);

    for (origin, direction, min, max, min_normal, max_normal) in [
        (
            origin.x,
            direction.x,
            min.x,
            max.x,
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        ),
        (
            origin.y,
            direction.y,
            min.y,
            max.y,
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ),
        (
            origin.z,
            direction.z,
            min.z,
            max.z,
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, 1.0),
        ),
    ] {
        if direction.abs() < EPSILON {
            if origin < min || origin > max {
                return None;
            }
            continue;
        }

        let first = (min - origin) / direction;
        let second = (max - origin) / direction;
        let (axis_entry, axis_exit, axis_normal) = if first < second {
            (first, second, min_normal)
        } else {
            (second, first, max_normal)
        };

        if axis_entry > entry {
            entry = axis_entry;
            entry_normal = axis_normal;
        }
        exit = exit.min(axis_exit);
    }

    if entry > exit {
        None
    } else {
        Some((entry, exit, entry_normal))
    }
}

fn next_boundary(cell: i32, origin: f32, direction: f32, step: i32) -> f32 {
    if step == 0 {
        f32::INFINITY
    } else {
        let boundary = if step > 0 { cell + 1 } else { cell };
        (boundary as f32 - origin) / direction
    }
}

fn axis_delta(direction: f32) -> f32 {
    if direction.abs() < EPSILON {
        f32::INFINITY
    } else {
        1.0 / direction.abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    #[test]
    fn finds_the_first_voxel_and_its_entry_normal() {
        let material = Material::new(Color::new(255, 255, 255), 1.0, 1.0, 0.0, 0.0, 1.0);
        let mut world = VoxelWorld::new();
        world.place_block(0, 0, 0, material);
        world.place_block(2, 0, 0, material);

        let hit = world
            .ray_intersect(&Vec3::new(-2.0, 0.5, 0.5), &Vec3::new(1.0, 0.0, 0.0))
            .expect("the ray should hit the first block");

        assert!((hit.distance - 2.0).abs() < EPSILON);
        assert_eq!(hit.normal, Vec3::new(-1.0, 0.0, 0.0));
    }
}
