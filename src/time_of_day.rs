use std::f32::consts::{PI, TAU};

use crate::color::Color;
use crate::light::Light;
use crate::vec3::Vec3;

/// Ciclo manual de 0.0 (medianoche) a 1.0, que vuelve a medianoche.
pub struct TimeOfDay {
    phase: f32,
}

impl TimeOfDay {
    pub const fn midday() -> Self {
        Self { phase: 0.5 }
    }

    pub fn advance(&mut self, amount: f32) {
        self.phase = (self.phase + amount).rem_euclid(1.0);
    }

    pub fn reset_midday(&mut self) {
        self.phase = 0.5;
    }

    pub fn daylight(&self) -> f32 {
        let height = self.sun_direction().y;
        smoothstep(-0.08, 0.42, height)
    }

    pub fn sun_direction(&self) -> Vec3 {
        let angle = self.phase * TAU - PI / 2.0;
        let horizontal = angle.cos();
        Vec3::new(-0.72 * horizontal, angle.sin(), 0.42 * horizontal).normalize()
    }

    pub fn light(&self) -> Light {
        let daylight = self.daylight();
        let sun_direction = self.sun_direction();
        let source_direction = if sun_direction.y >= 0.0 {
            sun_direction
        } else {
            -sun_direction
        };

        let color = blend(
            Color::new(126, 151, 214),
            Color::new(255, 246, 225),
            daylight,
        );
        let intensity = 0.22 + 1.28 * daylight;
        let ambient = 0.10 + 0.24 * daylight;
        Light::new(source_direction * 100.0, color, intensity, ambient)
    }
}

fn smoothstep(edge_start: f32, edge_end: f32, value: f32) -> f32 {
    let t = ((value - edge_start) / (edge_end - edge_start)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn blend(from: Color, to: Color, amount: f32) -> Color {
    let from = channels(from);
    let to = channels(to);
    Color::new(
        lerp(from[0], to[0], amount),
        lerp(from[1], to[1], amount),
        lerp(from[2], to[2], amount),
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

fn lerp(from: u8, to: u8, amount: f32) -> u8 {
    (from as f32 + (to as f32 - from as f32) * amount.clamp(0.0, 1.0)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midday_is_brighter_than_midnight() {
        let midday = TimeOfDay::midday();
        let mut midnight = TimeOfDay::midday();
        midnight.advance(0.5);

        assert!(midday.daylight() > midnight.daylight());
        assert!(midday.light().intensity > midnight.light().intensity);
    }

    #[test]
    fn time_wraps_in_both_directions() {
        let mut time = TimeOfDay::midday();
        time.advance(1.0);
        assert!((time.daylight() - 1.0).abs() < 1e-4);
        time.advance(-0.5);
        assert!(time.daylight() < 1e-4);
    }
}
