use crate::color::Color;
use crate::vec3::Vec3;

pub struct Light {
    pub position: Vec3,
    pub color: Color,
    pub intensity: f32,
    pub ambient: f32,
}

impl Light {
    pub fn new(position: Vec3, color: Color, intensity: f32, ambient: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            ambient,
        }
    }
}
