use std::f32::consts::TAU;

use crate::color::Color;
use crate::ray_intersect::Material;
use crate::world::VoxelWorld;

pub const ISLAND_RADIUS: i32 = 37;
pub const STATUE_CLEARANCE_RADIUS: i32 = 16;

const LAYERS: &[(i32, i32)] = &[
    (0, ISLAND_RADIUS),
    (-1, ISLAND_RADIUS - 2),
    (-2, ISLAND_RADIUS - 5),
    (-3, ISLAND_RADIUS - 9),
    (-4, ISLAND_RADIUS - 13),
    (-5, ISLAND_RADIUS - 18),
    (-6, ISLAND_RADIUS - 23),
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
                if is_statue_clearance(x, z) {
                    continue;
                }
                if !is_shell_block(layer_index, x, z, radius)
                    && !is_gap_border(x, z, radius)
                    && !is_statue_clearance_border(x, z)
                {
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
    build_central_statue_and_bridges(&mut world);

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

/// Forma las paredes verticales que cierran los cortes entre sectores. Los
/// huecos se mantienen, pero ya no permiten mirar al interior desde abajo.
fn is_gap_border(x: i32, z: i32, radius: i32) -> bool {
    [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)]
        .into_iter()
        .any(|(neighbor_x, neighbor_z)| {
            neighbor_x * neighbor_x + neighbor_z * neighbor_z <= radius * radius
                && is_gap(neighbor_x, neighbor_z)
        })
}

/// Plaza central vacía: futura base de una estatua con tres puentes.
fn is_statue_clearance(x: i32, z: i32) -> bool {
    x * x + z * z < STATUE_CLEARANCE_RADIUS * STATUE_CLEARANCE_RADIUS
}

/// Pared interior que sigue el borde circular de la plaza central. No llena la
/// plaza, pero evita que los rayos vean el volumen hueco bajo cada isla.
fn is_statue_clearance_border(x: i32, z: i32) -> bool {
    [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)]
        .into_iter()
        .any(|(neighbor_x, neighbor_z)| is_statue_clearance(neighbor_x, neighbor_z))
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
    fill_box(world, 15, 1, 16, 21, 1, 22, foundation);
    for y in 2..=4 {
        for x in 15..=21 {
            for z in 16..=22 {
                if x != 15 && x != 21 && z != 16 && z != 22 {
                    continue;
                }
                if z == 22 && (x == 17 || x == 18) && y <= 3 {
                    continue;
                }
                world.place_block(x, y, z, wall);
            }
        }
    }
    world.place_block(16, 3, 22, glass);
    world.place_block(19, 3, 22, glass);
    world.place_block(15, 3, 19, glass);
    fill_box(world, 14, 5, 15, 22, 5, 23, roof);
    fill_box(world, 15, 6, 16, 21, 6, 22, roof);

    // Árbol compacto junto a la casa para medir su relación de altura.
    fill_box(world, 25, 1, 21, 25, 5, 21, trunk);
    fill_box(world, 23, 6, 19, 27, 7, 23, leaves);
    fill_box(world, 24, 8, 20, 26, 8, 22, leaves);
}

fn build_nether_preview(world: &mut VoxelWorld) {
    let brick = matte(Color::new(122, 52, 56));
    let dark_brick = matte(Color::new(74, 35, 43));

    // Dos torres y una fachada con arco: fortaleza compacta de 9 x 9 bloques.
    build_tower(world, -21, -4, brick, dark_brick);
    build_tower(world, -21, 4, brick, dark_brick);

    for y in 1..=4 {
        for z in -4..=4 {
            let gate_opening = (-1..=1).contains(&z) && y <= 3;
            if !gate_opening {
                world.place_block(-20, y, z, brick);
            }
        }
    }
    fill_box(world, -20, 4, -1, -20, 4, 1, dark_brick);
}

fn build_end_preview(world: &mut VoxelWorld) {
    let pillar = matte(Color::new(40, 43, 51));
    let gold = matte(Color::new(220, 181, 55));
    let crystal = matte(Color::new(211, 88, 189));

    // Santuario: cuatro pilares de 3 x 3 y un altar central.
    for (x, z) in [(13, -26), (19, -26), (13, -20), (19, -20)] {
        fill_box(world, x - 1, 1, z - 1, x + 1, 8, z + 1, pillar);
        fill_box(world, x - 1, 9, z - 1, x + 1, 9, z + 1, gold);
    }
    fill_box(world, 15, 1, -24, 17, 1, -22, gold);
    fill_box(world, 16, 2, -23, 16, 4, -23, crystal);
}

/// Previsualización central: estatua de piedra y tres puentes que alcanzan el
/// centro de cada isla. Se sustituirá por materiales texturizados más adelante.
fn build_central_statue_and_bridges(world: &mut VoxelWorld) {
    let base_dark = matte(Color::new(57, 65, 80));
    let pedestal = matte(Color::new(104, 114, 130));
    let shadow = matte(Color::new(45, 51, 64));
    let accent = matte(Color::new(213, 178, 67));
    let bridge = matte(Color::new(124, 113, 103));
    let rail = matte(Color::new(72, 77, 89));

    // Pedestal macizo, ancho y escalonado: alcanza siete niveles sobre el vacío.
    fill_disc(world, -2, 6, base_dark);
    fill_disc(world, -1, 7, base_dark);
    fill_disc(world, 0, 8, pedestal);
    fill_disc(world, 1, 8, pedestal);
    fill_disc(world, 2, 7, pedestal);
    fill_disc(world, 3, 6, pedestal);
    fill_disc(world, 4, 5, pedestal);
    fill_disc(world, 5, 5, accent);

    build_watchtower(world, pedestal, shadow, accent);

    // Puentes hacia Overworld, Nether y End, respectivamente.
    build_bridge(world, 10, 18, bridge, rail);
    build_bridge(world, -21, 0, bridge, rail);
    build_bridge(world, 10, -18, bridge, rail);
}

/// Torre-mirador alta sobre el pedestal central, con ventanas y plataforma.
fn build_watchtower(world: &mut VoxelWorld, stone: Material, dark: Material, accent: Material) {
    // Piso y muros del cuerpo principal, vacío en el interior.
    fill_box(world, -3, 6, -3, 3, 6, 3, stone);
    for y in 7..=17 {
        for x in -3_i32..=3 {
            for z in -3_i32..=3 {
                if x.abs() == 3 || z.abs() == 3 {
                    world.place_block(x, y, z, stone);
                }
            }
        }
    }

    // Columnas de esquina, ventanas oscuras y dos pisos interiores visibles.
    for (x, z) in [(-3, -3), (-3, 3), (3, -3), (3, 3)] {
        fill_box(world, x, 7, z, x, 17, z, dark);
    }
    for y in [10, 14] {
        fill_box(world, -1, y, 3, 1, y + 1, 3, dark);
        fill_box(world, -1, y, -3, 1, y + 1, -3, dark);
        fill_box(world, 3, y, -1, 3, y + 1, 1, dark);
        fill_box(world, -3, y, -1, -3, y + 1, 1, dark);
    }
    fill_box(world, -2, 12, -2, 2, 12, 2, stone);
    fill_box(world, -2, 17, -2, 2, 17, 2, stone);

    // Mirador ancho, barandales de dos bloques y techo escalonado.
    fill_box(world, -5, 18, -5, 5, 18, 5, stone);
    for x in -5_i32..=5 {
        for z in -5_i32..=5 {
            if x.abs() == 5 || z.abs() == 5 {
                fill_box(world, x, 19, z, x, 20, z, dark);
            }
        }
    }
    fill_box(world, -4, 21, -4, 4, 21, 4, dark);
    fill_box(world, -3, 22, -3, 3, 22, 3, dark);
    fill_box(world, -2, 23, -2, 2, 23, 2, accent);
    fill_box(world, -1, 24, -1, 1, 25, 1, dark);
}

fn fill_disc(world: &mut VoxelWorld, y: i32, radius: i32, material: Material) {
    for x in -radius..=radius {
        for z in -radius..=radius {
            if x * x + z * z <= radius * radius {
                world.place_block(x, y, z, material);
            }
        }
    }
}

fn build_bridge(world: &mut VoxelWorld, end_x: i32, end_z: i32, deck: Material, rail: Material) {
    let steps = end_x.abs().max(end_z.abs());
    let side = if end_x.abs() >= end_z.abs() {
        (0, 1)
    } else {
        (1, 0)
    };

    for step in 0..=steps {
        let x = (end_x as f32 * step as f32 / steps as f32).round() as i32;
        let z = (end_z as f32 * step as f32 / steps as f32).round() as i32;
        let distance = ((x * x + z * z) as f32).sqrt();
        // Arranca sobre el borde del pedestal para que el tablero no flote.
        if distance < 5.0 {
            continue;
        }

        let end_distance = ((end_x * end_x + end_z * end_z) as f32).sqrt();
        let climb = ((distance - 5.0) / (end_distance - 5.0)).clamp(0.0, 1.0);
        let deck_y = (5.0 - 4.0 * climb).round() as i32;

        for width in -2..=1 {
            let bridge_x = x + side.0 * width;
            let bridge_z = z + side.1 * width;
            for thickness in 0..4 {
                world.place_block(bridge_x, deck_y - thickness, bridge_z, deck);
            }
            if width == -2 || width == 1 {
                world.place_block(bridge_x, deck_y + 1, bridge_z, rail);
                world.place_block(bridge_x, deck_y + 2, bridge_z, rail);
            }
        }
    }
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
    radius * distance_to_boundary.sin().abs() < 3.5
}
