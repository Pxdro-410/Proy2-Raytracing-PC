use std::f32::consts::TAU;

use crate::color::Color;
use crate::ray_intersect::Material;
use crate::world::VoxelWorld;

pub const ISLAND_RADIUS: i32 = 20;

const LAYERS: &[(i32, i32)] = &[
    (0, ISLAND_RADIUS),
    (-1, ISLAND_RADIUS - 1),
    (-2, ISLAND_RADIUS - 2),
    (-3, ISLAND_RADIUS - 4),
    (-4, ISLAND_RADIUS - 6),
    (-5, ISLAND_RADIUS - 9),
    (-6, ISLAND_RADIUS - 12),
];

pub fn build_base() -> VoxelWorld {
    let overworld = matte(Color::new(79, 151, 93));
    let nether = matte(Color::new(139, 58, 56));
    let end = matte(Color::new(202, 191, 112));
    let mut world = VoxelWorld::new();

    for (layer_index, &(y, radius)) in LAYERS.iter().enumerate() {
        for x in -radius..=radius {
            for z in -radius..=radius {
                if x * x + z * z > radius * radius {
                    continue;
                }
                if !is_shell_block(layer_index, x, z, radius) {
                    continue;
                }
                if is_gap(x, z) {
                    continue;
                }

                let material = match sector(x, z) {
                    0 => overworld,
                    1 => nether,
                    _ => end,
                };
                world.place_block(x, y, z, material);
            }
        }
    }

    build_overworld_preview(&mut world);
    build_nether_preview(&mut world);
    build_end_preview(&mut world);

    world
}

/// Conserva la piel exterior y una tapa interior bajo la superficie. La tapa
/// bloquea la vista hacia las construcciones cuando se mira desde abajo.
fn is_shell_block(layer_index: usize, x: i32, z: i32, radius: i32) -> bool {
    if layer_index <= 1 || layer_index == LAYERS.len() - 1 {
        return true;
    }

    let inner_radius = radius - 1;
    x * x + z * z > inner_radius * inner_radius
}

/// Construcciones de escala en colores planos. Las texturas y los detalles se
/// aplicarán después de aprobar el tamaño y la composición.
fn build_overworld_preview(world: &mut VoxelWorld) {
    let foundation = matte(Color::new(112, 112, 116));
    let wall = matte(Color::new(168, 122, 74));
    let roof = matte(Color::new(83, 63, 52));
    let glass = matte(Color::new(105, 180, 220));
    let trunk = matte(Color::new(116, 80, 48));
    let leaves = matte(Color::new(63, 125, 70));

    // Casa Skyblock: 7 x 7 bloques, con techo escalonado de dos niveles.
    fill_box(world, 5, 1, 4, 11, 1, 10, foundation);
    for y in 2..=4 {
        for x in 5..=11 {
            for z in 4..=10 {
                if x != 5 && x != 11 && z != 4 && z != 10 {
                    continue;
                }
                if z == 10 && (x == 7 || x == 8) && y <= 3 {
                    continue;
                }
                world.place_block(x, y, z, wall);
            }
        }
    }
    world.place_block(6, 3, 10, glass);
    world.place_block(9, 3, 10, glass);
    world.place_block(5, 3, 7, glass);
    fill_box(world, 4, 5, 3, 12, 5, 11, roof);
    fill_box(world, 5, 6, 4, 11, 6, 10, roof);

    // Árbol compacto junto a la casa para medir su relación de altura.
    fill_box(world, 14, 1, 9, 14, 5, 9, trunk);
    fill_box(world, 12, 6, 7, 16, 7, 11, leaves);
    fill_box(world, 13, 8, 8, 15, 8, 10, leaves);
}

fn build_nether_preview(world: &mut VoxelWorld) {
    let brick = matte(Color::new(122, 52, 56));
    let dark_brick = matte(Color::new(74, 35, 43));

    // Dos torres y una fachada con arco: fortaleza compacta de 9 x 9 bloques.
    build_tower(world, -8, -4, brick, dark_brick);
    build_tower(world, -8, 4, brick, dark_brick);

    for y in 1..=4 {
        for z in -4..=4 {
            let gate_opening = (-1..=1).contains(&z) && y <= 3;
            if !gate_opening {
                world.place_block(-7, y, z, brick);
            }
        }
    }
    fill_box(world, -7, 4, -1, -7, 4, 1, dark_brick);
}

fn build_end_preview(world: &mut VoxelWorld) {
    let pillar = matte(Color::new(40, 43, 51));
    let gold = matte(Color::new(220, 181, 55));
    let crystal = matte(Color::new(211, 88, 189));

    // Santuario: cuatro pilares de 3 x 3 y un altar central.
    for (x, z) in [(7, -13), (13, -13), (7, -7), (13, -7)] {
        fill_box(world, x - 1, 1, z - 1, x + 1, 8, z + 1, pillar);
        fill_box(world, x - 1, 9, z - 1, x + 1, 9, z + 1, gold);
    }
    fill_box(world, 9, 1, -11, 11, 1, -9, gold);
    fill_box(world, 10, 2, -10, 10, 4, -10, crystal);
}

fn build_tower(
    world: &mut VoxelWorld,
    center_x: i32,
    center_z: i32,
    wall: Material,
    crown: Material,
) {
    fill_box(
        world,
        center_x - 1,
        1,
        center_z - 1,
        center_x + 1,
        6,
        center_z + 1,
        wall,
    );
    for x in (center_x - 1)..=(center_x + 1) {
        for z in (center_z - 1)..=(center_z + 1) {
            if x == center_x - 1 || x == center_x + 1 || z == center_z - 1 || z == center_z + 1 {
                world.place_block(x, 7, z, crown);
            }
        }
    }
}

fn fill_box(
    world: &mut VoxelWorld,
    min_x: i32,
    min_y: i32,
    min_z: i32,
    max_x: i32,
    max_y: i32,
    max_z: i32,
    material: Material,
) {
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                world.place_block(x, y, z, material);
            }
        }
    }
}

fn matte(color: Color) -> Material {
    Material::new(color, 0.9, 10.0, 0.0, 0.0, 1.0)
}

fn sector(x: i32, z: i32) -> usize {
    let angle = (z as f32).atan2(x as f32).rem_euclid(TAU);
    (angle / (TAU / 3.0)).floor() as usize
}

/// Deja tres cortes radiales de ancho visible para que los mundos sean islas
fn is_gap(x: i32, z: i32) -> bool {
    let radius = ((x * x + z * z) as f32).sqrt();
    let angle = (z as f32).atan2(x as f32).rem_euclid(TAU);
    let wedge = TAU / 3.0;
    let distance_to_boundary = (angle % wedge).min(wedge - angle % wedge);
    radius * distance_to_boundary.sin().abs() < 1.05
}
