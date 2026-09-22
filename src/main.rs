mod camera;
mod color;
mod cube;
mod framebuffer;
mod island;
mod light;
mod materials;
mod ray_intersect;
mod skybox;
mod texture;
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
use crate::materials::BlockMaterials;
use crate::ray_intersect::{Intersect, Material, RayIntersect};
use crate::skybox::Skybox;
use crate::texture::TextureLibrary;
use crate::time_of_day::TimeOfDay;
use crate::vec3::Vec3;

const WIDTH: usize = 960;
const HEIGHT: usize = 720;
const FOV: f32 = PI / 3.0;
const ROTATION_SPEED: f32 = PI / 60.0;
const SHADOW_BIAS: f32 = 1e-3;
const REFLECTION_BIAS: f32 = 1e-3;
const REFRACTION_EXIT_BIAS: f32 = 1.01;
const MAX_DEPTH: u32 = 3;
const TIME_STEP: f32 = 0.025;

pub fn reflect(incident: &Vec3, normal: &Vec3) -> Vec3 {
    *incident - *normal * (2.0 * incident.dot(*normal))
}

pub fn refract(incident: &Vec3, normal: &Vec3, refractive_index: f32) -> Option<Vec3> {
    let mut normal = *normal;
    let mut eta_from = 1.0;
    let mut eta_to = refractive_index;
    let mut cos_incident = (-incident.dot(normal)).clamp(-1.0, 1.0);

    if cos_incident < 0.0 {
        cos_incident = -cos_incident;
        normal = -normal;
        eta_from = refractive_index;
        eta_to = 1.0;
    }

    let eta = eta_from / eta_to;
    let discriminant = 1.0 - eta * eta * (1.0 - cos_incident * cos_incident);
    (discriminant >= 0.0).then(|| {
        (*incident * eta + normal * (eta * cos_incident - discriminant.sqrt())).normalize()
    })
}

fn fresnel(incident: &Vec3, normal: &Vec3, refractive_index: f32) -> f32 {
    let cos_incident = (-incident.dot(*normal)).abs().clamp(0.0, 1.0);
    let base = ((1.0 - refractive_index) / (1.0 + refractive_index)).powi(2);
    base + (1.0 - base) * (1.0 - cos_incident).powi(5)
}

fn advance_past_voxel(point: Vec3, direction: Vec3) -> Vec3 {
    let largest_axis = direction
        .x
        .abs()
        .max(direction.y.abs())
        .max(direction.z.abs());
    point + direction * (REFRACTION_EXIT_BIAS / largest_axis)
}

fn material_transparency(material: &Material, texture_alpha: f32) -> f32 {
    let base = material.transparency.clamp(0.0, 1.0);
    if material.uses_texture_alpha {
        1.0 - (1.0 - base) * texture_alpha
    } else {
        base
    }
}

fn cast_shadow(
    intersect: &Intersect,
    light_direction: &Vec3,
    light: &Light,
    objects: &[Box<dyn RayIntersect>],
    textures: &TextureLibrary,
) -> bool {
    let origin = intersect.point + intersect.normal * SHADOW_BIAS;
    let light_distance = (light.position - intersect.point).magnitude();
    objects.iter().any(|object| {
        object
            .ray_intersect(&origin, light_direction)
            .is_some_and(|hit| {
                let alpha =
                    textures.sample_alpha(hit.material.texture_for(hit.face), hit.uv.0, hit.uv.1);
                hit.distance < light_distance
                    && material_transparency(&hit.material, alpha) < 0.5
                    && hit.material.emission_strength <= 0.0
                    && (hit.material.alpha_cutoff <= 0.0 || alpha >= hit.material.alpha_cutoff)
            })
    })
}

fn direct_light(
    intersect: &Intersect,
    ray_origin: &Vec3,
    surface_color: Color,
    light: &Light,
    intensity: f32,
    objects: &[Box<dyn RayIntersect>],
    textures: &TextureLibrary,
) -> Color {
    let light_direction = (light.position - intersect.point).normalize();
    if cast_shadow(intersect, &light_direction, light, objects, textures) {
        return Color::new(0, 0, 0);
    }

    let view_direction = (*ray_origin - intersect.point).normalize();
    let diffuse = surface_color.modulate(light.color)
        * (intersect.normal.dot(light_direction).max(0.0) * intersect.material.albedo * intensity);
    let reflect_direction = reflect(&-light_direction, &intersect.normal);
    let specular = light.color
        * (view_direction
            .dot(reflect_direction)
            .max(0.0)
            .powf(intersect.material.specular)
            * intensity
            * intersect.material.specular_strength);
    diffuse + specular
}

fn shade(
    intersect: &Intersect,
    ray_origin: &Vec3,
    light: &Light,
    effect_lights: &[Light],
    objects: &[Box<dyn RayIntersect>],
    textures: &TextureLibrary,
) -> Color {
    let surface_color = textures.sample(
        intersect.material.texture_for(intersect.face),
        intersect.uv.0,
        intersect.uv.1,
        intersect.material.diffuse,
    );
    let ambient = surface_color * (intersect.material.albedo * light.ambient);
    let sunlight = direct_light(
        intersect,
        ray_origin,
        surface_color,
        light,
        light.intensity,
        objects,
        textures,
    );
    let local_lighting = effect_lights
        .iter()
        .fold(Color::new(0, 0, 0), |sum, local_light| {
            let distance = (local_light.position - intersect.point).magnitude();
            let attenuation = 1.0 / (1.0 + 0.12 * distance + 0.04 * distance * distance);
            sum + direct_light(
                intersect,
                ray_origin,
                surface_color,
                local_light,
                local_light.intensity * attenuation,
                objects,
                textures,
            )
        });
    let emission = intersect.material.emission * intersect.material.emission_strength;
    ambient + sunlight + local_lighting + emission
}

fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Box<dyn RayIntersect>],
    light: &Light,
    effect_lights: &[Light],
    skybox: &Skybox,
    textures: &TextureLibrary,
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
    let alpha = textures.sample_alpha(
        intersect.material.texture_for(intersect.face),
        intersect.uv.0,
        intersect.uv.1,
    );
    if alpha < intersect.material.alpha_cutoff {
        let origin = advance_past_voxel(intersect.point, *ray_direction);
        return cast_ray(
            &origin,
            ray_direction,
            objects,
            light,
            effect_lights,
            skybox,
            textures,
            depth,
        );
    }
    let local = shade(
        &intersect,
        ray_origin,
        light,
        effect_lights,
        objects,
        textures,
    );
    let transparency = material_transparency(&intersect.material, alpha);
    let mut reflection_weight = intersect.material.reflectivity.clamp(0.0, 1.0);
    if transparency > 0.0 {
        reflection_weight += (1.0 - reflection_weight)
            * fresnel(
                ray_direction,
                &intersect.normal,
                intersect.material.refractive_index,
            );
    }

    let mut transmission_weight = (1.0 - reflection_weight) * transparency;
    let refracted_direction = refract(
        ray_direction,
        &intersect.normal,
        intersect.material.refractive_index,
    );
    if transparency > 0.0 && refracted_direction.is_none() {
        reflection_weight = 1.0;
        transmission_weight = 0.0;
    }
    let local_weight = (1.0 - reflection_weight - transmission_weight).max(0.0);
    let reflected = if reflection_weight > 0.0 {
        let direction = reflect(ray_direction, &intersect.normal).normalize();
        let origin = intersect.point + intersect.normal * REFLECTION_BIAS;
        cast_ray(
            &origin,
            &direction,
            objects,
            light,
            effect_lights,
            skybox,
            textures,
            depth + 1,
        )
    } else {
        Color::new(0, 0, 0)
    };
    let refracted = if transparency > 0.0 {
        if let Some(direction) = refracted_direction {
            let origin = advance_past_voxel(intersect.point, direction);
            cast_ray(
                &origin,
                &direction,
                objects,
                light,
                effect_lights,
                skybox,
                textures,
                depth + 1,
            )
        } else {
            Color::new(0, 0, 0)
        }
    } else {
        Color::new(0, 0, 0)
    };
    local * local_weight + reflected * reflection_weight + refracted * transmission_weight
}

fn pixel_color(
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    objects: &[Box<dyn RayIntersect>],
    camera: &Camera,
    light: &Light,
    effect_lights: &[Light],
    skybox: &Skybox,
    textures: &TextureLibrary,
) -> u32 {
    let aspect_ratio = width as f32 / height as f32;
    let perspective_scale = (FOV / 2.0).tan();
    let screen_x = ((2.0 * x as f32) / width as f32 - 1.0) * aspect_ratio * perspective_scale;
    let screen_y = (-(2.0 * y as f32) / height as f32 + 1.0) * perspective_scale;
    let direction = camera.basis_change(&Vec3::new(screen_x, screen_y, -1.0).normalize());
    cast_ray(
        &camera.eye,
        &direction,
        objects,
        light,
        effect_lights,
        skybox,
        textures,
        0,
    )
    .to_hex()
}

#[cfg(not(feature = "parallel"))]
fn render(
    framebuffer: &mut Framebuffer,
    objects: &[Box<dyn RayIntersect>],
    camera: &Camera,
    light: &Light,
    effect_lights: &[Light],
    skybox: &Skybox,
    textures: &TextureLibrary,
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
                effect_lights,
                skybox,
                textures,
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
    effect_lights: &[Light],
    skybox: &Skybox,
    textures: &TextureLibrary,
) {
    let width = framebuffer.width;
    let height = framebuffer.height;
    framebuffer
        .buffer
        .par_chunks_mut(width)
        .enumerate()
        .for_each(|(y, row)| {
            for (x, pixel) in row.iter_mut().enumerate() {
                *pixel = pixel_color(
                    x,
                    y,
                    width,
                    height,
                    objects,
                    camera,
                    light,
                    effect_lights,
                    skybox,
                    textures,
                );
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
    let textures = TextureLibrary::load_default().expect("No se pudieron cargar las texturas PPM");
    let materials = BlockMaterials::new();
    let island = build_base(&materials);
    println!("Maqueta de isla creada: {} bloques.", island.block_count());
    let objects: Vec<Box<dyn RayIntersect>> = vec![Box::new(island)];
    let mut time = TimeOfDay::midday();
    let mut skybox = Skybox::daytime();
    let mut light = time.light();
    let effect_lights = [
        Light::new(
            Vec3::new(-28.5, 3.0, -6.5),
            Color::new(255, 74, 20),
            7.0,
            0.0,
        ),
        Light::new(
            Vec3::new(-25.0, 10.0, 1.0),
            Color::new(255, 204, 110),
            5.0,
            0.0,
        ),
        Light::new(
            Vec3::new(16.5, 4.5, -23.0),
            Color::new(235, 90, 210),
            3.5,
            0.0,
        ),
    ];
    update_environment(&time, &mut skybox, &mut light);
    let mut camera = Camera::new(
        Vec3::new(48.0, 28.0, 48.0),
        Vec3::new(0.0, -3.0, 0.0),
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
            render(
                &mut framebuffer,
                &objects,
                &camera,
                &light,
                &effect_lights,
                &skybox,
                &textures,
            );
            camera_moved = false;
        }
        window
            .update_with_buffer(&framebuffer.buffer, WIDTH, HEIGHT)
            .unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refraction_bends_an_air_to_water_ray_toward_the_normal() {
        let incident = Vec3::new(0.5, -0.866_025_4, 0.0);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let refracted = refract(&incident, &normal, 1.33).expect("debe refractar");

        assert!(refracted.y < 0.0);
        assert!(refracted.x.abs() < incident.x.abs());
    }

    #[test]
    fn fresnel_reflection_increases_at_grazing_angles() {
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let frontal = fresnel(&Vec3::new(0.0, -1.0, 0.0), &normal, 1.33);
        let grazing = fresnel(&Vec3::new(0.99, -0.1, 0.0).normalize(), &normal, 1.33);

        assert!(grazing > frontal);
    }
}
