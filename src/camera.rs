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

    /// Gira la direccion observada sin desplazar el ojo de la camara. Esto
    /// representa mirar a izquierda/derecha desde la posicion del jugador,
    /// a diferencia de `orbit`, que inspecciona la escena desde fuera.
    pub fn turn_view(&mut self, delta_yaw: f32, delta_pitch: f32) {
        let view = self.center - self.eye;
        let distance = view.magnitude();
        if distance <= f32::EPSILON {
            return;
        }
        let horizontal_length = (view.x * view.x + view.z * view.z).sqrt();
        let yaw = view.z.atan2(view.x) + delta_yaw;
        let pitch = view.y.atan2(horizontal_length) + delta_pitch;
        let pitch = pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
        let horizontal_distance = distance * pitch.cos();
        self.center = self.eye
            + Vec3::new(
                horizontal_distance * yaw.cos(),
                distance * pitch.sin(),
                horizontal_distance * yaw.sin(),
            );
    }

    /// Traslada ojo y punto observado en el plano horizontal local. Se usa
    /// para caminar en modo de camara libre sin cambiar la altura del jugador.
    pub fn move_local(&mut self, forward_amount: f32, right_amount: f32) {
        let view = self.center - self.eye;
        let forward = Vec3::new(view.x, 0.0, view.z).normalize();
        if forward.magnitude() <= f32::EPSILON {
            return;
        }
        let right = forward.cross(self.up).normalize();
        let offset = forward * forward_amount + right * right_amount;
        self.eye = self.eye + offset;
        self.center = self.center + offset;
    }

    pub fn zoom(&mut self, amount: f32) {
        let offset = self.eye - self.center;
        let distance = (offset.magnitude() + amount).clamp(5.0, 110.0);
        self.eye = self.center + offset.normalize() * distance;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_view_keeps_the_eye_fixed() {
        let mut camera = Camera::new(
            Vec3::new(5.0, 3.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let eye = camera.eye;
        let distance = (camera.center - camera.eye).magnitude();

        camera.turn_view(PI / 4.0, PI / 8.0);

        assert_eq!(camera.eye, eye);
        assert!(((camera.center - camera.eye).magnitude() - distance).abs() < 1e-5);
    }

    #[test]
    fn local_movement_translates_eye_and_center_together() {
        let mut camera = Camera::new(
            Vec3::new(0.0, 3.0, 5.0),
            Vec3::new(0.0, 3.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        camera.move_local(2.0, 1.0);

        assert_eq!(camera.eye, Vec3::new(1.0, 3.0, 3.0));
        assert_eq!(camera.center, Vec3::new(1.0, 3.0, -2.0));
    }
}
