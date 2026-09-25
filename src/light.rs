use crate::color::Color;
use crate::vec3::Vec3;

pub struct Light {
    pub position: Vec3,
    pub color: Color,
    pub intensity: f32,
    pub ambient: f32,
    pub uses_distance_attenuation: bool,
    pub radius: f32,
}

impl Light {
    pub fn point(position: Vec3, color: Color, intensity: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            ambient: 0.0,
            uses_distance_attenuation: true,
            radius: 12.0,
        }
    }

    /// Luz puntual con alcance finito. Evita calcular sombras para fuentes
    /// que no pueden aportar luz visible al punto sombreado.
    pub fn point_with_radius(position: Vec3, color: Color, intensity: f32, radius: f32) -> Self {
        Self {
            radius: radius.max(0.0),
            ..Self::point(position, color, intensity)
        }
    }

    pub fn directional(position: Vec3, color: Color, intensity: f32, ambient: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            ambient,
            uses_distance_attenuation: false,
            radius: f32::INFINITY,
        }
    }
}
