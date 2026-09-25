mod camera;
mod color;
mod cube;
mod framebuffer;
mod hud;
mod island;
mod light;
mod materials;
mod ray_intersect;
mod skybox;
mod texture;
mod time_of_day;
mod vec3;
mod world;

use minifb::{Key, KeyRepeat, MouseButton, Window, WindowOptions};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::f32::consts::PI;
use std::time::{Duration, Instant};

use crate::camera::Camera;
use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::hud::{Hotbar, HotbarAction};
use crate::island::build_base;
use crate::light::Light;
use crate::materials::BlockMaterials;
use crate::ray_intersect::{BlockFace, Intersect, Material, RayIntersect};
use crate::skybox::Skybox;
use crate::texture::{TextureId, TextureLibrary};
use crate::time_of_day::TimeOfDay;
use crate::vec3::Vec3;
use crate::world::VoxelWorld;

const WIDTH: usize = 960;
const HEIGHT: usize = 720;
const FOV: f32 = PI / 3.0;
const ROTATION_SPEED: f32 = PI / 60.0;
const SHADOW_BIAS: f32 = 1e-3;
const REFLECTION_BIAS: f32 = 1e-3;
const REFRACTION_EXIT_BIAS: f32 = 1.01;
const MAX_DEPTH: u32 = 3;
const TIME_SPEED: f32 = 0.055;
const ZOOM_SPEED: f32 = 9.0;
const FREE_MOVE_SPEED: f32 = 12.0;

#[derive(Clone, Copy, Debug)]
enum NavigationMode {
    Orbit,
    Free,
}

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

fn texture_uv(intersect: &Intersect) -> (f32, f32) {
    let scale = intersect.material.world_uv_scale;
    if scale <= 0.0 {
        return intersect.uv;
    }

    match intersect.face {
        BlockFace::NegativeY | BlockFace::PositiveY => {
            (intersect.point.x * scale, intersect.point.z * scale)
        }
        BlockFace::NegativeX | BlockFace::PositiveX => {
            (intersect.point.z * scale, intersect.point.y * scale)
        }
        BlockFace::NegativeZ | BlockFace::PositiveZ => {
            (intersect.point.x * scale, intersect.point.y * scale)
        }
    }
}

fn texture_alpha(textures: &TextureLibrary, intersect: &Intersect) -> f32 {
    let (u, v) = texture_uv(intersect);
    textures.sample_alpha(intersect.material.texture_for(intersect.face), u, v)
}

fn cast_shadow(
    intersect: &Intersect,
    light_direction: &Vec3,
    light: &Light,
    objects: &[&dyn RayIntersect],
    textures: &TextureLibrary,
) -> bool {
    let origin = intersect.point + intersect.normal * SHADOW_BIAS;
    let light_distance = (light.position - intersect.point).magnitude();
    objects.iter().any(|object| {
        object
            .ray_intersect(&origin, light_direction)
            .is_some_and(|hit| {
                let alpha = texture_alpha(textures, &hit);
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
    objects: &[&dyn RayIntersect],
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
    objects: &[&dyn RayIntersect],
    textures: &TextureLibrary,
) -> Color {
    let (u, v) = texture_uv(intersect);
    let surface_color = textures.sample(
        intersect.material.texture_for(intersect.face),
        u,
        v,
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
            if local_light.intensity <= 0.0 {
                return sum;
            }
            let distance = (local_light.position - intersect.point).magnitude();
            if local_light.uses_distance_attenuation && distance > local_light.radius {
                return sum;
            }
            let attenuation = if local_light.uses_distance_attenuation {
                1.0 / (1.0 + 0.12 * distance + 0.04 * distance * distance)
            } else {
                1.0
            };
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
    objects: &[&dyn RayIntersect],
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
    let alpha = texture_alpha(textures, &intersect);
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
    let refracted_direction = if (intersect.material.refractive_index - 1.0).abs() < f32::EPSILON {
        // Vidrio claro: el rayo atraviesa el voxel en línea recta. Evita que
        // un bloque de grosor unitario se comporte como una lente artificial.
        Some(*ray_direction)
    } else {
        refract(
            ray_direction,
            &intersect.normal,
            intersect.material.refractive_index,
        )
    };
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
    objects: &[&dyn RayIntersect],
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
    objects: &[&dyn RayIntersect],
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
    objects: &[&dyn RayIntersect],
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

/// Reune luces derivadas de los bloques actuales. La glowstone se registra
/// bloque por bloque; la lava se agrupa por celdas de 10x6x10 para cubrir una
/// superficie grande sin convertir cada voxel en una luz costosa.
fn build_effect_lights(world: &VoxelWorld, time: &TimeOfDay) -> Vec<Light> {
    let mut lights = vec![
        // Portal y cristal del End: emisores no representados por glowstone.
        Light::point_with_radius(
            Vec3::new(-25.5, 3.5, 0.5),
            Color::new(215, 80, 240),
            5.5,
            8.0,
        ),
        Light::point_with_radius(
            Vec3::new(22.0, 16.5, -25.0),
            Color::new(235, 90, 210),
            4.5,
            8.0,
        ),
    ];

    lights.extend(
        world
            .block_centers_with_texture(TextureId::Glowstone)
            .into_iter()
            .map(|position| {
                Light::point_with_radius(position, Color::new(255, 204, 120), 2.0, 6.5)
            }),
    );
    lights.extend(build_lava_lights(world));
    lights.push(time.moon_light());
    lights
}

fn build_lava_lights(world: &VoxelWorld) -> Vec<Light> {
    const CELL_WIDTH: i32 = 10;
    const CELL_HEIGHT: i32 = 6;
    let mut cells = BTreeMap::<(i32, i32, i32), (f32, f32, f32, u32)>::new();

    for position in world.block_centers_with_texture(TextureId::Lava) {
        let key = (
            (position.x.floor() as i32).div_euclid(CELL_WIDTH),
            (position.y.floor() as i32).div_euclid(CELL_HEIGHT),
            (position.z.floor() as i32).div_euclid(CELL_WIDTH),
        );
        let cell = cells.entry(key).or_insert((0.0, 0.0, 0.0, 0));
        cell.0 += position.x;
        cell.1 += position.y;
        cell.2 += position.z;
        cell.3 += 1;
    }

    cells
        .into_values()
        .map(|(x, y, z, count)| {
            let count = count as f32;
            Light::point_with_radius(
                Vec3::new(x / count, y / count, z / count),
                Color::new(255, 88, 28),
                4.0,
                8.5,
            )
        })
        .collect()
}

fn center_ray(camera: &Camera) -> Vec3 {
    camera.basis_change(&Vec3::new(0.0, 0.0, -1.0)).normalize()
}

fn targeted_intersection(
    camera: &Camera,
    world: &VoxelWorld,
    textures: &TextureLibrary,
) -> Option<Intersect> {
    let direction = center_ray(camera);
    let mut origin = camera.eye;
    for _ in 0..32 {
        let hit = world.ray_intersect(&origin, &direction)?;
        if texture_alpha(textures, &hit) < hit.material.alpha_cutoff {
            origin = advance_past_voxel(hit.point, direction);
            continue;
        }
        return Some(hit);
    }
    None
}

fn hit_block_position(hit: &Intersect) -> (i32, i32, i32) {
    let inside = hit.point - hit.normal * 1e-3;
    (
        inside.x.floor() as i32,
        inside.y.floor() as i32,
        inside.z.floor() as i32,
    )
}

fn adjacent_block_position(hit: &Intersect) -> (i32, i32, i32) {
    let outside = hit.point + hit.normal * 1e-3;
    (
        outside.x.floor() as i32,
        outside.y.floor() as i32,
        outside.z.floor() as i32,
    )
}

fn apply_selected_action(
    hotbar: &Hotbar,
    camera: &Camera,
    world: &mut VoxelWorld,
    textures: &TextureLibrary,
) -> bool {
    let Some(hit) = targeted_intersection(camera, world, textures) else {
        return false;
    };
    match hotbar.selected_action() {
        HotbarAction::Place(material) => {
            let (x, y, z) = adjacent_block_position(&hit);
            world.place_block(x, y, z, material);
            true
        }
        HotbarAction::Remove => {
            let (x, y, z) = hit_block_position(&hit);
            world.remove_block(x, y, z).is_some()
        }
        HotbarAction::None => false,
    }
}

fn copy_target_block(
    hotbar: &mut Hotbar,
    camera: &Camera,
    world: &VoxelWorld,
    textures: &TextureLibrary,
) -> bool {
    let Some(hit) = targeted_intersection(camera, world, textures) else {
        return false;
    };
    let (x, y, z) = hit_block_position(&hit);
    let Some(material) = world.block_at(x, y, z) else {
        return false;
    };
    hotbar.copy_material(material);
    true
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
    let mut island = build_base(&materials);
    println!("Maqueta de isla creada: {} bloques.", island.block_count());
    let mut hotbar = Hotbar::new(&materials);
    let mut time = TimeOfDay::midday();
    let mut skybox = Skybox::daytime();
    let mut light = time.light();
    update_environment(&time, &mut skybox, &mut light);
    let mut effect_lights = build_effect_lights(&island, &time);
    let mut camera = Camera::new(
        Vec3::new(48.0, 28.0, 48.0),
        Vec3::new(0.0, -3.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let mut camera_moved = true;
    let mut navigation_mode = NavigationMode::Orbit;
    let mut last_frame = Instant::now();
    let mut left_mouse_was_down = false;
    let mut right_mouse_was_down = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let elapsed = last_frame.elapsed().as_secs_f32().min(0.1);
        last_frame = Instant::now();
        for (key, yaw, pitch) in [
            (Key::Left, -ROTATION_SPEED, 0.0),
            (Key::Right, ROTATION_SPEED, 0.0),
            (Key::Up, 0.0, ROTATION_SPEED),
            (Key::Down, 0.0, -ROTATION_SPEED),
        ] {
            if window.is_key_down(key) {
                camera.turn_view(yaw, pitch);
                camera_moved = true;
            }
        }
        match navigation_mode {
            NavigationMode::Orbit => {
                for (key, yaw, pitch) in [
                    (Key::A, ROTATION_SPEED, 0.0),
                    (Key::D, -ROTATION_SPEED, 0.0),
                    (Key::W, 0.0, -ROTATION_SPEED),
                    (Key::S, 0.0, ROTATION_SPEED),
                ] {
                    if window.is_key_down(key) {
                        camera.orbit(yaw, pitch);
                        camera_moved = true;
                    }
                }
            }
            NavigationMode::Free => {
                let forward = if window.is_key_down(Key::W) {
                    FREE_MOVE_SPEED * elapsed
                } else if window.is_key_down(Key::S) {
                    -FREE_MOVE_SPEED * elapsed
                } else {
                    0.0
                };
                let right = if window.is_key_down(Key::D) {
                    FREE_MOVE_SPEED * elapsed
                } else if window.is_key_down(Key::A) {
                    -FREE_MOVE_SPEED * elapsed
                } else {
                    0.0
                };
                let vertical = if window.is_key_down(Key::Space) {
                    FREE_MOVE_SPEED * elapsed
                } else if window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift)
                {
                    -FREE_MOVE_SPEED * elapsed
                } else {
                    0.0
                };
                if forward != 0.0 || right != 0.0 || vertical != 0.0 {
                    camera.move_local(forward, right);
                    camera.move_vertical(vertical);
                    camera_moved = true;
                }
            }
        }
        if window.is_key_pressed(Key::F, KeyRepeat::No) {
            navigation_mode = match navigation_mode {
                NavigationMode::Orbit => NavigationMode::Free,
                NavigationMode::Free => NavigationMode::Orbit,
            };
            println!("Modo de navegacion: {:?}", navigation_mode);
            camera_moved = true;
        }
        if window.is_key_down(Key::Equal) || window.is_key_down(Key::NumPadPlus) {
            camera.zoom(-ZOOM_SPEED * elapsed);
            camera_moved = true;
        }
        if window.is_key_down(Key::Minus) || window.is_key_down(Key::NumPadMinus) {
            camera.zoom(ZOOM_SPEED * elapsed);
            camera_moved = true;
        }
        let time_delta = if window.is_key_down(Key::Q) {
            -TIME_SPEED * elapsed
        } else if window.is_key_down(Key::E) {
            TIME_SPEED * elapsed
        } else {
            0.0
        };
        if time_delta != 0.0 {
            time.advance(time_delta);
            update_environment(&time, &mut skybox, &mut light);
            effect_lights = build_effect_lights(&island, &time);
            camera_moved = true;
        }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            time.reset_midday();
            update_environment(&time, &mut skybox, &mut light);
            effect_lights = build_effect_lights(&island, &time);
            camera_moved = true;
        }
        for (slot, key) in [
            (0, Key::Key1),
            (1, Key::Key2),
            (2, Key::Key3),
            (3, Key::Key4),
            (4, Key::Key5),
            (5, Key::Key6),
        ] {
            if window.is_key_pressed(key, KeyRepeat::No) {
                camera_moved |= hotbar.select(slot);
            }
        }

        let left_mouse_down = window.get_mouse_down(MouseButton::Left);
        if left_mouse_down && !left_mouse_was_down {
            let world_changed = apply_selected_action(&hotbar, &camera, &mut island, &textures);
            if world_changed {
                effect_lights = build_effect_lights(&island, &time);
                camera_moved = true;
            }
        }
        left_mouse_was_down = left_mouse_down;

        let right_mouse_down = window.get_mouse_down(MouseButton::Right);
        if right_mouse_down && !right_mouse_was_down {
            camera_moved |= copy_target_block(&mut hotbar, &camera, &island, &textures);
        }
        right_mouse_was_down = right_mouse_down;

        if camera_moved {
            let objects: [&dyn RayIntersect; 1] = [&island];
            render(
                &mut framebuffer,
                &objects,
                &camera,
                &light,
                &effect_lights,
                &skybox,
                &textures,
            );
            hud::draw(&mut framebuffer, &hotbar, &textures);
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
