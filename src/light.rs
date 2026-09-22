use crate::color::Color;
use crate::vec3::Vec3;

pub struct Light {
    pub position: Vec3,
    pub color: Color,
    pub intensity: f32,
    pub ambient: f32,
    pub uses_distance_attenuation: bool,
}

impl Light {
    pub fn point(position: Vec3, color: Color, intensity: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            ambient: 0.0,
            uses_distance_attenuation: true,
        }
    }

    pub fn directional(position: Vec3, color: Color, intensity: f32, ambient: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            ambient,
            uses_distance_attenuation: false,
        }
    }
}
