use crate::color::Color;
use crate::vec3::Vec3;

const STARS: &[(f32, f32, f32, f32)] = &[
    (-0.67, 0.72, -0.18, 0.004),
    (-0.42, 0.88, 0.31, 0.003),
    (-0.12, 0.63, -0.76, 0.005),
    (0.18, 0.91, -0.24, 0.003),
    (0.39, 0.68, 0.58, 0.004),
    (0.61, 0.78, -0.12, 0.003),
    (0.74, 0.55, 0.39, 0.005),
    (-0.81, 0.49, 0.31, 0.003),
    (0.04, 0.82, 0.53, 0.003),
    (-0.32, 0.57, 0.68, 0.004),
];

/// Fondo atmosférico continuo, sin geometría ni caras visibles.
/// proporciona color para los rayos que no golpean la isla.
pub struct Skybox {
    daylight: f32,
    sun_direction: Vec3,
}

impl Skybox {
    pub const fn daytime() -> Self {
        Self {
            daylight: 1.0,
            sun_direction: Vec3::new(-0.55, 0.62, 0.56),
        }
    }

    pub const fn nighttime() -> Self {
        Self {
            daylight: 0.0,
            sun_direction: Vec3::new(-0.55, 0.62, 0.56),
        }
    }

    pub fn set_daylight(&mut self, daylight: f32) {
        self.daylight = daylight.clamp(0.0, 1.0);
    }

    pub fn set_sun_direction(&mut self, direction: Vec3) {
        self.sun_direction = direction.normalize();
    }

    pub fn sample(&self, direction: &Vec3) -> Color {
        let direction = direction.normalize();
        let atmospheric_height = ((direction.y + 0.08) / 1.08).clamp(0.0, 1.0);

        let day = gradient([190, 224, 250], [73, 166, 245], atmospheric_height);
        let night = gradient([42, 66, 111], [7, 20, 57], atmospheric_height);
        let mut color = blend(night, day, self.daylight);

        let sun_direction = self.sun_direction.normalize();
        let sun_height = sun_direction.y;
        let sun_visible = smoothstep(-0.08, 0.07, sun_height);
        let low_sun = 1.0 - smoothstep(0.12, 0.55, sun_height.max(0.0));
        let horizon_weight = (1.0 - atmospheric_height).powi(2);
        let sunset_strength = sun_visible * low_sun * horizon_weight * 0.72;
        color = blend(color, Color::new(255, 135, 76), sunset_strength);

        let moon_direction = -sun_direction;
        let sun = square_disc(direction, sun_direction, 0.028);
        let moon = square_disc(direction, moon_direction, 0.022);

        if sun && sun_visible > 0.0 {
            let disc_color = blend(
                Color::new(255, 174, 88),
                Color::new(255, 247, 190),
                1.0 - low_sun,
            );
            color = blend(color, disc_color, sun_visible);
        }
        if moon {
            color = blend(color, Color::new(221, 231, 255), 1.0 - self.daylight);
        }
        let star_visibility = (1.0 - self.daylight).powi(2);
        if star_visibility > 0.01 && is_star(direction) {
            color = blend(color, Color::new(232, 239, 255), star_visibility);
        }

        color
    }
}

fn is_star(direction: Vec3) -> bool {
    STARS
        .iter()
        .any(|&(x, y, z, size)| square_disc(direction, Vec3::new(x, y, z).normalize(), size))
}

fn square_disc(direction: Vec3, center: Vec3, half_size: f32) -> bool {
    let reference = if center.y.abs() < 0.9 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let right = center.cross(reference).normalize();
    let up = right.cross(center).normalize();
    let facing = direction.dot(center);

    facing > 0.995 && direction.dot(right).abs() < half_size && direction.dot(up).abs() < half_size
}

fn gradient(horizon: [u8; 3], zenith: [u8; 3], amount: f32) -> Color {
    Color::new(
        lerp_channel(horizon[0], zenith[0], amount),
        lerp_channel(horizon[1], zenith[1], amount),
        lerp_channel(horizon[2], zenith[2], amount),
    )
}

fn blend(from: Color, to: Color, amount: f32) -> Color {
    let from = channels(from);
    let to = channels(to);
    Color::new(
        lerp_channel(from[0], to[0], amount),
        lerp_channel(from[1], to[1], amount),
        lerp_channel(from[2], to[2], amount),
    )
}

fn channels(color: Color) -> [u8; 3] {
    let hex = color.to_hex();
    [
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    ]
}

fn lerp_channel(from: u8, to: u8, amount: f32) -> u8 {
    (from as f32 + (to as f32 - from as f32) * amount.clamp(0.0, 1.0)) as u8
}

fn smoothstep(edge_start: f32, edge_end: f32, value: f32) -> f32 {
    let t = ((value - edge_start) / (edge_end - edge_start)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
