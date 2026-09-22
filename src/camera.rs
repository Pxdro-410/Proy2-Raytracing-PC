use crate::vec3::Vec3;
use std::f32::consts::PI;

const PITCH_LIMIT: f32 = PI / 2.0 - 0.1;

pub struct Camera {
    pub eye: Vec3,
    pub center: Vec3,
    pub up: Vec3,
}

impl Camera {
    pub fn new(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        Self { eye, center, up }
    }

    pub fn basis_change(&self, vector: &Vec3) -> Vec3 {
        let forward = (self.center - self.eye).normalize();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward).normalize();
        (right * vector.x + up * vector.y - forward * vector.z).normalize()
    }

    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        let radius_vector = self.eye - self.center;
        let radius = radius_vector.magnitude();
        let current_yaw = radius_vector.z.atan2(radius_vector.x);
        let radius_xz =
            (radius_vector.x * radius_vector.x + radius_vector.z * radius_vector.z).sqrt();
        let current_pitch = (-radius_vector.y).atan2(radius_xz);
        let yaw = (current_yaw + delta_yaw) % (2.0 * PI);
        let pitch = (current_pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        self.eye = self.center
            + Vec3::new(
                radius * yaw.cos() * pitch.cos(),
                -radius * pitch.sin(),
                radius * yaw.sin() * pitch.cos(),
            );
    }

    pub fn zoom(&mut self, amount: f32) {
        let offset = self.eye - self.center;
        let distance = (offset.magnitude() + amount).clamp(5.0, 110.0);
        self.eye = self.center + offset.normalize() * distance;
    }
}
