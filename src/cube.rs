use crate::ray_intersect::{BlockFace, Intersect, Material, RayIntersect};
use crate::vec3::Vec3;

const EPSILON: f32 = 1e-4;

/// Cubo alineado a los ejes: la primitiva de todos los bloques de la isla.
pub struct Cube {
    pub min: Vec3,
    pub max: Vec3,
    pub material: Material,
}

impl Cube {
    pub fn from_block(position: Vec3, material: Material) -> Self {
        Self {
            min: position,
            max: position + Vec3::new(1.0, 1.0, 1.0),
            material,
        }
    }
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<Intersect> {
        let (near_x, far_x) = slab(ray_origin.x, ray_direction.x, self.min.x, self.max.x);
        let (near_y, far_y) = slab(ray_origin.y, ray_direction.y, self.min.y, self.max.y);
        let (near_z, far_z) = slab(ray_origin.z, ray_direction.z, self.min.z, self.max.z);
        let near = near_x.max(near_y).max(near_z);
        let far = far_x.min(far_y).min(far_z);

        if near > far || far < EPSILON {
            return None;
        }

        let distance = if near > EPSILON { near } else { far };
        let point = *ray_origin + *ray_direction * distance;
        let normal = cube_normal(point, self.min, self.max);
        let face = BlockFace::from_normal(normal);
        Some(Intersect {
            point,
            normal,
            distance,
            material: self.material,
            face,
            uv: face.uv(point, self.min),
        })
    }
}

fn slab(origin: f32, direction: f32, min: f32, max: f32) -> (f32, f32) {
    if direction.abs() < EPSILON {
        if origin < min || origin > max {
            (f32::INFINITY, f32::NEG_INFINITY)
        } else {
            (f32::NEG_INFINITY, f32::INFINITY)
        }
    } else {
        let first = (min - origin) / direction;
        let second = (max - origin) / direction;
        (first.min(second), first.max(second))
    }
}

fn cube_normal(point: Vec3, min: Vec3, max: Vec3) -> Vec3 {
    if (point.x - min.x).abs() < EPSILON {
        return Vec3::new(-1.0, 0.0, 0.0);
    }
    if (point.x - max.x).abs() < EPSILON {
        return Vec3::new(1.0, 0.0, 0.0);
    }
    if (point.y - min.y).abs() < EPSILON {
        return Vec3::new(0.0, -1.0, 0.0);
    }
    if (point.y - max.y).abs() < EPSILON {
        return Vec3::new(0.0, 1.0, 0.0);
    }
    if (point.z - min.z).abs() < EPSILON {
        return Vec3::new(0.0, 0.0, -1.0);
    }
    Vec3::new(0.0, 0.0, 1.0)
}
