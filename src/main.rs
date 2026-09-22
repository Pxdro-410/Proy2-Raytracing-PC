mod camera;
mod color;
mod cube;
mod framebuffer;
mod island;
mod light;
mod ray_intersect;
mod skybox;
mod time_of_day;
mod vec3;
mod world;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::f32::consts::PI;
use std::time::Duration;

use crate::camera::Camera;
use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::island::build_base;
use crate::light::Light;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::skybox::Skybox;
use crate::time_of_day::TimeOfDay;
use crate::vec3::Vec3;

const WIDTH: usize = 960;
const HEIGHT: usize = 720;
const FOV: f32 = PI / 3.0;
const ROTATION_SPEED: f32 = PI / 60.0;
const SHADOW_BIAS: f32 = 1e-3;
const REFLECTION_BIAS: f32 = 1e-3;
const MAX_DEPTH: u32 = 3;
const TIME_STEP: f32 = 0.025;

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
    let ambient = intersect.material.diffuse * (intersect.material.albedo * light.ambient);
    let reflect_direction = reflect(&-light_direction, &intersect.normal);
    let specular = light.color
        * (view_direction
            .dot(reflect_direction)
            .max(0.0)
            .powf(intersect.material.specular)
            * light_intensity);
    ambient + diffuse + specular
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

fn pixel_color(
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    objects: &[Box<dyn RayIntersect>],
    camera: &Camera,
    light: &Light,
    skybox: &Skybox,
) -> u32 {
    let aspect_ratio = width as f32 / height as f32;
    let perspective_scale = (FOV / 2.0).tan();
    let screen_x = ((2.0 * x as f32) / width as f32 - 1.0) * aspect_ratio * perspective_scale;
    let screen_y = (-(2.0 * y as f32) / height as f32 + 1.0) * perspective_scale;
    let direction = camera.basis_change(&Vec3::new(screen_x, screen_y, -1.0).normalize());
    cast_ray(&camera.eye, &direction, objects, light, skybox, 0).to_hex()
}

#[cfg(not(feature = "parallel"))]
fn render(
    framebuffer: &mut Framebuffer,
    objects: &[Box<dyn RayIntersect>],
    camera: &Camera,
    light: &Light,
    skybox: &Skybox,
) {
    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let color = pixel_color(
                x,
                y,
                framebuffer.width,
                framebuffer.height,
                objects,
                camera,
                light,
                skybox,
            );
            framebuffer.buffer[y * framebuffer.width + x] = color;
        }
    }
}

#[cfg(feature = "parallel")]
fn render(
    framebuffer: &mut Framebuffer,
    objects: &[Box<dyn RayIntersect>],
    camera: &Camera,
    light: &Light,
    skybox: &Skybox,
) {
    let width = framebuffer.width;
    let height = framebuffer.height;
    framebuffer
        .buffer
        .par_chunks_mut(width)
        .enumerate()
        .for_each(|(y, row)| {
            for (x, pixel) in row.iter_mut().enumerate() {
                *pixel = pixel_color(x, y, width, height, objects, camera, light, skybox);
            }
        });
}

fn update_environment(time: &TimeOfDay, skybox: &mut Skybox, light: &mut Light) {
    skybox.set_daylight(time.daylight());
    skybox.set_sun_direction(time.sun_direction());
    *light = time.light();
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
    let island = build_base();
    println!("Maqueta de isla creada: {} bloques.", island.block_count());
    let objects: Vec<Box<dyn RayIntersect>> = vec![Box::new(island)];
    let mut time = TimeOfDay::midday();
    let mut skybox = Skybox::daytime();
    let mut light = time.light();
    update_environment(&time, &mut skybox, &mut light);
    let mut camera = Camera::new(
        Vec3::new(28.0, 18.0, 28.0),
        Vec3::new(0.0, -2.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let mut camera_moved = true;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for (key, yaw, pitch) in [
            (Key::Left, ROTATION_SPEED, 0.0),
            (Key::Right, -ROTATION_SPEED, 0.0),
            (Key::Up, 0.0, -ROTATION_SPEED),
            (Key::Down, 0.0, ROTATION_SPEED),
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
        if window.is_key_pressed(Key::Q, KeyRepeat::Yes) {
            time.advance(-TIME_STEP);
            update_environment(&time, &mut skybox, &mut light);
            camera_moved = true;
        }
        if window.is_key_pressed(Key::E, KeyRepeat::Yes) {
            time.advance(TIME_STEP);
            update_environment(&time, &mut skybox, &mut light);
            camera_moved = true;
        }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            time.reset_midday();
            update_environment(&time, &mut skybox, &mut light);
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
