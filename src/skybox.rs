use crate::color::Color;
use crate::vec3::Vec3;

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
        if let Some((star_intensity, star_color)) = star_sample(direction) {
            color = blend(color, star_color, star_visibility * star_intensity);
        }

        color
    }
}

/// Campo estelar determinista sobre toda la esfera, incluyendo las zonas que
/// se ven bajo la isla.
fn star_sample(direction: Vec3) -> Option<(f32, Color)> {
    const COLUMNS: i32 = 72;
    const ROWS: i32 = 36;

    let u = direction.z.atan2(direction.x) / std::f32::consts::TAU + 0.5;
    let v = direction.y.clamp(-1.0, 1.0).asin() / std::f32::consts::PI + 0.5;
    let cell_x = (u * COLUMNS as f32).floor() as i32;
    let cell_y = (v * ROWS as f32).floor() as i32;

    let mut sample = None;
    for offset_y in -1..=1 {
        let y = cell_y + offset_y;
        if !(0..ROWS).contains(&y) {
            continue;
        }
        for offset_x in -1..=1 {
            let x = (cell_x + offset_x).rem_euclid(COLUMNS);
            if hash(x, y, 0) > 0.34 {
                continue;
            }
            let center_u = (x as f32 + hash(x, y, 1)) / COLUMNS as f32;
            let center_v = (y as f32 + hash(x, y, 2)) / ROWS as f32;
            let delta_u = (u - center_u).abs().min(1.0 - (u - center_u).abs());
            let delta_v = (v - center_v).abs();
            // Estrellas cuadradas, pequeñas y sin el aspecto de pÃ­xel blanco
            // grande: aproximadamente uno a tres pixels en el encuadre.
            let size = 0.000_25 + hash(x, y, 3) * 0.000_45;
            if delta_u < size && delta_v < size {
                let intensity = 0.45 + hash(x, y, 4) * 0.35;
                if sample.map_or(true, |(strongest, _)| intensity > strongest) {
                    sample = Some((intensity, star_color(hash(x, y, 5))));
                }
            }
        }
    }
    sample
}

fn star_color(variant: f32) -> Color {
    if variant < 0.28 {
        Color::new(155, 169, 186) // gris fri­o tenue
    } else if variant < 0.62 {
        Color::new(169, 193, 220) // celeste tenue
    } else if variant < 0.84 {
        Color::new(190, 200, 211) // gris azulado
    } else {
        Color::new(183, 211, 235) // azul fri­o brillante
    }
}

fn hash(x: i32, y: i32, salt: i32) -> f32 {
    let mut value = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        .wrapping_add(salt.wrapping_mul(982_451_653));
    value = (value ^ (value >> 13)).wrapping_mul(1_274_126_177);
    ((value ^ (value >> 16)) & 0x00ff_ffff) as f32 / 0x00ff_ffff as f32
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stars_exist_above_and_below_the_horizon() {
        assert!(has_star_in_rows(0, 18));
        assert!(has_star_in_rows(18, 36));
    }

    fn has_star_in_rows(start_row: i32, end_row: i32) -> bool {
        for y in start_row..end_row {
            for x in 0..72 {
                if hash(x, y, 0) > 0.34 {
                    continue;
                }
                let u = (x as f32 + hash(x, y, 1)) / 72.0;
                let v = (y as f32 + hash(x, y, 2)) / 36.0;
                let longitude = (u - 0.5) * std::f32::consts::TAU;
                let latitude = (v - 0.5) * std::f32::consts::PI;
                let direction = Vec3::new(
                    latitude.cos() * longitude.cos(),
                    latitude.sin(),
                    latitude.cos() * longitude.sin(),
                );
                if star_sample(direction).is_some() {
                    return true;
                }
            }
        }
        false
    }
}
