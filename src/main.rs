mod camera;
mod color;
mod cube;
mod framebuffer;
mod light;
mod ray_intersect;
mod skybox;
mod vec3;

use minifb::{Key, Window, WindowOptions};
use std::f32::consts::PI;
use std::time::Duration;

use crate::camera::Camera;
use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::light::Light;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::skybox::Skybox;
use crate::vec3::Vec3;

const WIDTH: usize = 800;
const HEIGHT: usize = 600;
const FOV: f32 = PI / 3.0;
const ROTATION_SPEED: f32 = PI / 60.0;
const SHADOW_BIAS: f32 = 1e-3;
const REFLECTION_BIAS: f32 = 1e-3;
const MAX_DEPTH: u32 = 3;

pub fn reflect(incident: &Vec3, normal: &Vec3) -> Vec3 {
    *incident - *normal * (2.0 * incident.dot(*normal))
}

fn cast_shadow(
    intersect: &Intersect,
    light_direction: &Vec3,
    light: &Light,
    objects: &[Box<dyn RayIntersect>],
) -> bool {
    let origin = intersect.point + intersect.normal * SHADOW_BIAS;
    let light_distance = (light.position - intersect.point).magnitude();
    objects.iter().any(|object| {
        object
            .ray_intersect(&origin, light_direction)
            .is_some_and(|hit| hit.distance < light_distance)
    })
}

fn shade(
    intersect: &Intersect,
    ray_origin: &Vec3,
    light: &Light,
    objects: &[Box<dyn RayIntersect>],
) -> Color {
    let light_direction = (light.position - intersect.point).normalize();
    let view_direction = (*ray_origin - intersect.point).normalize();
    let light_intensity = if cast_shadow(intersect, &light_direction, light, objects) {
        0.0
    } else {
        light.intensity
    };
    let diffuse = intersect.material.diffuse
        * (intersect.normal.dot(light_direction).max(0.0)
            * intersect.material.albedo
            * light_intensity);
    let reflect_direction = reflect(&-light_direction, &intersect.normal);
    let specular = light.color
        * (view_direction
            .dot(reflect_direction)
            .max(0.0)
            .powf(intersect.material.specular)
            * light_intensity);
    diffuse + specular
}

fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Box<dyn RayIntersect>],
    light: &Light,
    skybox: &Skybox,
    depth: u32,
) -> Color {
    if depth > MAX_DEPTH {
        return skybox.sample(ray_direction);
    }
    let closest = objects
        .iter()
        .filter_map(|object| object.ray_intersect(ray_origin, ray_direction))
        .min_by(|a, b| a.distance.total_cmp(&b.distance));
    let Some(intersect) = closest else {
        return skybox.sample(ray_direction);
    };
    let local = shade(&intersect, ray_origin, light, objects);
    if intersect.material.reflectivity <= 0.0 {
        return local;
    }
    let direction = reflect(ray_direction, &intersect.normal).normalize();
    let origin = intersect.point + intersect.normal * REFLECTION_BIAS;
    let reflected = cast_ray(&origin, &direction, objects, light, skybox, depth + 1);
    local * (1.0 - intersect.material.reflectivity) + reflected * intersect.material.reflectivity
}

fn render(
    framebuffer: &mut Framebuffer,
    objects: &[Box<dyn RayIntersect>],
    camera: &Camera,
    light: &Light,
    skybox: &Skybox,
) {
    let aspect_ratio = framebuffer.width as f32 / framebuffer.height as f32;
    let perspective_scale = (FOV / 2.0).tan();
    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let screen_x = ((2.0 * x as f32) / framebuffer.width as f32 - 1.0)
                * aspect_ratio
                * perspective_scale;
            let screen_y =
                (-(2.0 * y as f32) / framebuffer.height as f32 + 1.0) * perspective_scale;
            let direction = camera.basis_change(&Vec3::new(screen_x, screen_y, -1.0).normalize());
            framebuffer.set_current_color(
                cast_ray(&camera.eye, &direction, objects, light, skybox, 0).to_hex(),
            );
            framebuffer.point(x, y);
        }
    }
}

fn main() {
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let mut window = Window::new(
        "Minecraft Island Raytracer",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap();
    let objects: Vec<Box<dyn RayIntersect>> = Vec::new();
    let light = Light::new(Vec3::new(-6.0, 6.0, 8.0), Color::new(255, 255, 255), 1.5);
    let skybox = Skybox::daytime();
    let mut camera = Camera::new(
        Vec3::new(0.0, 0.4, 6.0),
        Vec3::new(0.0, -0.7, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let mut camera_moved = true;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for (key, yaw, pitch) in [
            (Key::Left, -ROTATION_SPEED, 0.0),
            (Key::Right, ROTATION_SPEED, 0.0),
            (Key::Up, 0.0, ROTATION_SPEED),
            (Key::Down, 0.0, -ROTATION_SPEED),
        ] {
            if window.is_key_down(key) {
                camera.orbit(yaw, pitch);
                camera_moved = true;
            }
        }
        if window.is_key_down(Key::W) {
            camera.zoom(-0.2);
            camera_moved = true;
        }
        if window.is_key_down(Key::S) {
            camera.zoom(0.2);
            camera_moved = true;
        }
        if camera_moved {
            render(&mut framebuffer, &objects, &camera, &light, &skybox);
            camera_moved = false;
        }
        window
            .update_with_buffer(&framebuffer.buffer, WIDTH, HEIGHT)
            .unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
}
