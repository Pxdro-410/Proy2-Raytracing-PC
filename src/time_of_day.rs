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
        // El cielo conserva un crepúsculo suave incluso cuando el sol acaba
        // de ocultarse, pero la luz directa se calcula por separado.
        smoothstep(-0.18, 0.28, self.sun_direction().y)
    }

    pub fn sun_direction(&self) -> Vec3 {
        let angle = self.phase * TAU - PI / 2.0;
        let horizontal = angle.cos();
        Vec3::new(-0.72 * horizontal, angle.sin(), 0.42 * horizontal).normalize()
    }

    pub fn light(&self) -> Light {
        let daylight = self.daylight();
        let sun_direction = self.sun_direction();
        let direct_sun = smoothstep(-0.04, 0.32, sun_direction.y);
        let high_sun = smoothstep(0.12, 0.58, sun_direction.y.max(0.0));
        let color = blend(
            Color::new(255, 128, 62),
            Color::new(255, 244, 220),
            high_sun,
        );

        // El sol alto queda deliberadamente por debajo de la exposición
        // anterior. Cerca del horizonte su intensidad y el ángulo rasante
        // producen sombras largas y un atardecer legible.
        let intensity = 0.78 * direct_sun;
        // Un relleno ambiental algo mayor conserva legibles el pasto, los
        // arboles y la casa del Overworld sin borrar las sombras del sol.
        let ambient = 0.025 + 0.20 * daylight;
        Light::directional(sun_direction * 100.0, color, intensity, ambient)
    }

    pub fn moon_light(&self) -> Light {
        let sun_height = self.sun_direction().y;
        let night = 1.0 - smoothstep(-0.06, 0.22, sun_height);
        let moon_direction = -self.sun_direction();
        Light::directional(
            moon_direction * 100.0,
            Color::new(156, 186, 255),
            0.22 * night,
            0.0,
        )
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

    #[test]
    fn sunset_is_warmer_and_dimmer_than_midday() {
        let midday = TimeOfDay::midday().light();
        let mut sunset = TimeOfDay::midday();
        sunset.advance(-0.25);
        let sunset = sunset.light();

        assert!(sunset.intensity < midday.intensity);
        assert!(red(sunset.color) > blue(sunset.color));
    }

    #[test]
    fn moon_is_present_at_midnight_and_absent_at_midday() {
        let midday = TimeOfDay::midday().moon_light();
        let mut midnight = TimeOfDay::midday();
        midnight.advance(0.5);
        let midnight = midnight.moon_light();

        assert!(midnight.intensity > 0.2);
        assert!(midday.intensity < 1e-4);
    }

    fn red(color: Color) -> u8 {
        (color.to_hex() >> 16) as u8
    }

    fn blue(color: Color) -> u8 {
        color.to_hex() as u8
    }
}
