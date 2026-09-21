use crate::color::Color;
use crate::vec3::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub diffuse: Color,
    pub albedo: f32,
    pub specular: f32,
    pub reflectivity: f32,
    pub transparency: f32,
    pub refractive_index: f32,
}

impl Material {
    pub fn new(
        diffuse: Color,
        albedo: f32,
        specular: f32,
        reflectivity: f32,
        transparency: f32,
        refractive_index: f32,
    ) -> Self {
        Self {
            diffuse,
            albedo,
            specular,
            reflectivity,
            transparency,
            refractive_index,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Intersect {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    pub material: Material,
}

pub trait RayIntersect {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<Intersect>;
}
