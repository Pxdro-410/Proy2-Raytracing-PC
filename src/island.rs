use std::f32::consts::TAU;

use crate::materials::BlockMaterials;
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

pub fn build_base(materials: &BlockMaterials) -> VoxelWorld {
    let overworld = materials.grass;
    let nether = materials.netherrack;
    let end = materials.end_stone;
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

    build_surface_relief(&mut world, materials);
    build_overworld_preview(&mut world, materials);
    build_nether_preview(&mut world, materials);
    build_end_preview(&mut world, materials);
    build_central_statue_and_bridges(&mut world, materials);

    world
}

/// Eleva la superficie con perfiles distintos para cada dimension. Las
/// plataformas de las construcciones y los puentes permanecen niveladas para
/// que las estructuras no queden enterradas en el terreno.
fn build_surface_relief(world: &mut VoxelWorld, materials: &BlockMaterials) {
    for x in -ISLAND_RADIUS..=ISLAND_RADIUS {
        for z in -ISLAND_RADIUS..=ISLAND_RADIUS {
            if x * x + z * z > ISLAND_RADIUS * ISLAND_RADIUS
                || is_gap(x, z)
                || is_statue_clearance(x, z)
                || is_flattened_area(x, z)
            {
                continue;
            }

            let sector = sector(x, z);
            let height = relief_height(x, z, sector);
            if height == 0 {
                continue;
            }

            for y in 1..=height {
                let material = match sector {
                    // Las colinas del Overworld dejan tierra expuesta debajo
                    // de una capa de cesped.
                    0 if y == height => materials.grass,
                    0 => materials.coarse_dirt,
                    // El Nether usa terrazas irregulares de netherrack.
                    1 => materials.netherrack,
                    // Las mesetas del End son de End stone y contrastan con
                    // las construcciones negras y doradas.
                    _ => materials.end_stone,
                };
                world.place_block(x, y, z, material);
            }
        }
    }
}

/// Altura discreta por sector: suficientemente marcada para leerse desde
/// lejos, sin competir con la torre central ni las construcciones principales.
fn relief_height(x: i32, z: i32, sector: usize) -> i32 {
    let x = x as f32;
    let z = z as f32;
    match sector {
        // Colinas redondeadas de uno a cuatro bloques.
        0 => ((x * 0.38).sin() * 1.2 + (z * 0.31).cos() * 1.0 + 1.65)
            .round()
            .clamp(0.0, 4.0) as i32,
        // Relieve mas abrupto, con picos de hasta cinco bloques.
        1 => ((x * 0.73).sin().abs() * 1.8
            + (z * 0.91).cos().abs() * 1.6
            + ((x - z) * 0.23).sin() * 0.7)
            .round()
            .clamp(0.0, 5.0) as i32,
        // Escalones amplios que forman mesetas quebradas caracteristicas del End.
        _ => ((x * 0.24).sin() + (z * 0.27).cos() + 1.4)
            .floor()
            .clamp(0.0, 3.0) as i32,
    }
}

/// Zonas planas para las construcciones existentes y los tres accesos. El
/// resto de cada sector conserva el relieve natural.
fn is_flattened_area(x: i32, z: i32) -> bool {
    // Overworld: casa en la explanada amplia, al mismo lado del rio.
    if (3..=9).contains(&x) && (23..=29).contains(&z) {
        return true;
    }
    if is_river_corridor(x, z) {
        return true;
    }
    // Nether: fortaleza, portal, lagos de lava y glowstone.
    if (-33..=-18).contains(&x) && (-10..=10).contains(&z) {
        return true;
    }
    // El portal queda fuera del lago, sobre una pequena plataforma plana en
    // su lado izquierdo para que el marco nunca quede enterrado por el relieve.
    if (-36..=-34).contains(&x) && (-3..=4).contains(&z) {
        return true;
    }
    // End: pilares de obsidiana, fuente y santuario.
    if (10..=28).contains(&x) && (-30..=-14).contains(&z) {
        return true;
    }

    is_bridge_corridor(x, z)
}

const RIVER_PATH: &[(i32, i32)] = &[
    (12, 11),
    (13, 11),
    (13, 12),
    (14, 12),
    (14, 13),
    (15, 13),
    (15, 14),
    (16, 14),
    (16, 15),
    (17, 15),
    (17, 16),
    (18, 16),
    (18, 17),
    (19, 17),
    (19, 18),
    (20, 18),
    (20, 19),
    (21, 19),
    (21, 20),
    (22, 20),
    (22, 21),
    (23, 21),
    (23, 22),
    (24, 22),
    (24, 23),
    (25, 23),
    (25, 24),
    (25, 25),
    (26, 25),
    (26, 26),
];

/// Identifica el corredor del rio en el Overworld para que las colinas
/// de relieve no rellenen el canal hundido.
fn is_river_corridor(x: i32, z: i32) -> bool {
    RIVER_PATH.iter().any(|&(rx, rz)| {
        let dx = x - rx;
        let dz = z - rz;
        dx * dx + dz * dz <= 5
    })
}

fn is_river_channel(x: i32, z: i32) -> bool {
    RIVER_PATH.iter().any(|&(rx, rz)| (x - rx).abs() <= 1 && (z - rz).abs() <= 1)
}

/// Reserva cuatro bloques de ancho bajo los puentes para que suban desde el
/// pedestal hasta cada isla sin quedar absorbidos por una colina.
fn is_bridge_corridor(x: i32, z: i32) -> bool {
    [(11_i32, 19_i32), (-22, 0), (11, -19)]
        .into_iter()
        .any(|(end_x, end_z)| {
            let steps = end_x.abs().max(end_z.abs());
            (0..=steps).any(|step| {
                let path_x = (end_x as f32 * step as f32 / steps as f32).round() as i32;
                let path_z = (end_z as f32 * step as f32 / steps as f32).round() as i32;
                (x - path_x).abs() <= 3 && (z - path_z).abs() <= 3
            })
        })
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

/// Plaza central vacía: base de la torre monumental y los tres puentes.
fn is_statue_clearance(x: i32, z: i32) -> bool {
    x * x + z * z < STATUE_CLEARANCE_RADIUS * STATUE_CLEARANCE_RADIUS
}

/// Pared interior que sigue el borde circular de la plaza central.
fn is_statue_clearance_border(x: i32, z: i32) -> bool {
    [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)]
        .into_iter()
        .any(|(neighbor_x, neighbor_z)| is_statue_clearance(neighbor_x, neighbor_z))
}

/// Overworld:
/// Casa Skyblock acogedora y mas pequena (5x5) en el lado opuesto del rio.
/// Rio continuo y limpio, hundido 1 bloque bajo el cesped, con cascada al acantilado.
/// Grandes arboles de cerezo (tronco cherry) con mucho volumen formando arcos sobre el agua.
fn build_overworld_preview(world: &mut VoxelWorld, materials: &BlockMaterials) {
    let foundation = materials.cobblestone;
    let wall = materials.oak_log;
    let roof = materials.black_terracotta;
    let glass = materials.glass;

    // Asegurar suelo base de cesped a y = 1 (salvo en el canal del rio)
    for x in 12..=36 {
        for z in 5..=36 {
            if x * x + z * z > ISLAND_RADIUS * ISLAND_RADIUS || is_gap(x, z) || is_statue_clearance(x, z) {
                continue;
            }
            if !is_river_channel(x, z) && world.block_at(x, 1, z).is_none() {
                world.place_block(x, 1, z, materials.grass);
            }
        }
    }

    // Casa Skyblock compacta (5x5) en la explanada libre del mismo lado del
    // rio. Queda separada de las copas de cerezo, de la cascada y del cauce.
    fill_box(world, 4, 1, 24, 8, 1, 28, foundation);
    for y in 2..=4 {
        for x in 4..=8 {
            for z in 24..=28 {
                let is_edge = x == 4 || x == 8 || z == 24 || z == 28;
                if !is_edge {
                    continue;
                }
                // Puerta hacia el camino sur
                if z == 24 && x == 6 && y <= 3 {
                    continue;
                }
                world.place_block(x, y, z, wall);
            }
        }
    }
    // Ventanas de vidrio en la casita
    world.place_block(4, 3, 26, glass);
    world.place_block(8, 3, 26, glass);
    world.place_block(6, 3, 28, glass);
    // Lampara interior: ilumina la casa por las ventanas sin alterar su techo.
    world.place_block(6, 3, 26, materials.glowstone);
    // Techo escalonado
    fill_box(world, 3, 5, 23, 9, 5, 29, roof);
    fill_box(world, 4, 6, 24, 8, 6, 28, roof);

    // Rio limpio, continuo y hundido con su cascada al acantilado
    build_overworld_sunken_river(world, materials);

    // Arboles de cerezo grandes formando arcos con las hojas sobre el rio
    build_cherry_arch_trees(world, materials);

    // Gran cerezo majestuoso con mucho volumen en la explanada sur
    build_grand_cherry_tree(world, 27, 1, 18, materials);

    // Cerezo mirador junto al acantilado de la cascada
    build_waterfall_cherry_tree(world, 22, 1, 28, materials);
}

/// se construye el rio
fn build_overworld_sunken_river(world: &mut VoxelWorld, materials: &BlockMaterials) {
    // 1. Excavar y colocar agua continua en cada coordenada del camino
    for &(rx, rz) in RIVER_PATH {
        for dx in -1_i32..=1_i32 {
            for dz in -1_i32..=1_i32 {
                let x = rx + dx;
                let z = rz + dz;
                if x * x + z * z > 37 * 37 || is_statue_clearance(x, z) {
                    continue;
                }
                // Limpiar aire a y = 1 y 2 para abrir el canal hundido
                world.remove_block(x, 2, z);
                world.remove_block(x, 1, z);
                // Agua clara y cristalina a y = 0
                world.place_block(x, 0, z, materials.water);
                // Lecho limpio del rio a y = -1
                world.place_block(x, -1, z, materials.coarse_dirt);
                // Base rocosa a y = -2
                world.place_block(x, -2, z, materials.stone);
            }
        }
    }

    // 2. Asegurar riberas firmes de cesped a y = 1 enmarcando el agua
    for &(rx, rz) in RIVER_PATH {
        for (bx, bz) in [(rx - 2, rz), (rx + 2, rz), (rx, rz - 2), (rx, rz + 2)] {
            if bx * bx + bz * bz <= 36 * 36 && !is_gap(bx, bz) && !is_statue_clearance(bx, bz) {
                if !is_river_channel(bx, bz) {
                    if world.block_at(bx, 1, bz).is_none() {
                        world.place_block(bx, 1, bz, materials.grass);
                    }
                    world.place_block(bx, 0, bz, materials.coarse_dirt);
                }
            }
        }
    }

    // 3. Puente rustico de madera cruzando el rio hundido
    for offset in -1_i32..=1_i32 {
        world.place_block(18 + offset, 1, 16 - offset, materials.oak_log);
    }
    world.place_block(17, 2, 17, materials.cobblestone);
    world.place_block(19, 2, 15, materials.cobblestone);
    world.place_block(17, 3, 17, materials.glowstone);

    // 4. Cascada conectada directamente a la roca del acantilado de la isla en (26, 26)
    for (wx, wz) in [(26, 26), (25, 26), (26, 25), (25, 27), (27, 25)] {
        for y in (-6_i32)..=0_i32 {
            world.place_block(wx, y, wz, materials.water);
        }
        // Acantilado de roca de la isla adosado al agua
        world.place_block(wx - 1, 0, wz - 1, materials.mossy_cobblestone);
        for y in (-6_i32)..=-1_i32 {
            world.place_block(wx - 1, y, wz - 1, materials.stone);
        }
    }
}

/// Construye dos cerezos monumentales a ambos lados del rio cuyas ramas
/// y copas de hojas se extienden y cruzan por encima del agua, formando un arco floral.
fn build_cherry_arch_trees(world: &mut VoxelWorld, materials: &BlockMaterials) {
    let log = materials.cherry_log;
    let leaves = materials.cherry_leaves;

    // Arbol Norte (x = 15, z = 19): tronco robusto que se inclina hacia el rio
    for y in 1..=7 {
        world.place_block(15, y, 19, log);
        if y <= 4 {
            world.place_block(15, y, 20, log);
        }
    }
    // Rama principal que arquea hacia el sureste sobre el rio
    world.place_block(16, 5, 18, log);
    world.place_block(17, 6, 17, log);
    world.place_block(18, 6, 16, log);
    world.place_block(19, 7, 16, log);

    // Gran copa de hojas en arco colgando sobre el cauce del rio
    build_leaf_cloud(world, 18, 7, 16, 2, leaves);
    build_leaf_cloud(world, 15, 8, 19, 2, leaves);
    world.place_block(18, 9, 16, leaves);

    // Arbol Sur (x = 21, z = 13): tronco en la ribera opuesta que arquea al noroeste
    for y in 1..=7 {
        world.place_block(21, y, 13, log);
        if y <= 4 {
            world.place_block(22, y, 13, log);
        }
    }
    // Rama que se extiende hacia el rio al encuentro del otro arbol
    world.place_block(20, 5, 14, log);
    world.place_block(19, 6, 15, log);
    world.place_block(18, 7, 15, log);

    // Gran copa de hojas formando el cierre del arco superior
    build_leaf_cloud(world, 19, 7, 15, 2, leaves);
    build_leaf_cloud(world, 21, 8, 13, 2, leaves);
    world.place_block(19, 9, 15, leaves);
}

/// Gran arbol de cerezo con tronco masivo (cherry_log) y cuatro grandes
/// nubes de hojas con enorme volumen en la explanada abierta.
fn build_grand_cherry_tree(
    world: &mut VoxelWorld,
    cx: i32,
    base_y: i32,
    cz: i32,
    materials: &BlockMaterials,
) {
    let log = materials.cherry_log;
    let leaves = materials.cherry_leaves;

    // Raices expuestas en la base
    for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1)] {
        world.place_block(cx + dx, base_y + 1, cz + dz, log);
    }

    // Tronco principal masivo de altura 9
    for y in (base_y + 1)..=(base_y + 9) {
        world.place_block(cx, y, cz, log);
        if y <= base_y + 5 {
            world.place_block(cx + 1, y, cz, log);
            world.place_block(cx, y, cz + 1, log);
        }
    }

    // Ramas en las cuatro direcciones
    world.place_block(cx + 1, base_y + 6, cz, log);
    world.place_block(cx + 2, base_y + 7, cz, log);
    world.place_block(cx + 3, base_y + 7, cz, log);

    world.place_block(cx - 1, base_y + 5, cz, log);
    world.place_block(cx - 2, base_y + 6, cz, log);
    world.place_block(cx - 3, base_y + 6, cz, log);

    world.place_block(cx, base_y + 6, cz + 1, log);
    world.place_block(cx, base_y + 7, cz + 2, log);

    world.place_block(cx, base_y + 6, cz - 1, log);
    world.place_block(cx, base_y + 7, cz - 2, log);

    // Grandes nubes de hojas con abundante volumen (clouds)
    build_leaf_cloud(world, cx, base_y + 10, cz, 3, leaves);
    build_leaf_cloud(world, cx + 3, base_y + 8, cz, 2, leaves);
    build_leaf_cloud(world, cx - 3, base_y + 7, cz, 2, leaves);
    build_leaf_cloud(world, cx, base_y + 8, cz + 2, 2, leaves);
    build_leaf_cloud(world, cx, base_y + 8, cz - 2, 2, leaves);
    world.place_block(cx, base_y + 12, cz, leaves);
}

/// Arbol de cerezo con copa colgante junto al mirador de la cascada
fn build_waterfall_cherry_tree(
    world: &mut VoxelWorld,
    x: i32,
    base_y: i32,
    z: i32,
    materials: &BlockMaterials,
) {
    let log = materials.cherry_log;
    let leaves = materials.cherry_leaves;

    for y in base_y..=(base_y + 6) {
        world.place_block(x, y, z, log);
    }
    world.place_block(x + 1, base_y + 5, z, log);
    world.place_block(x + 2, base_y + 6, z + 1, log);

    build_leaf_cloud(world, x, base_y + 7, z, 2, leaves);
    build_leaf_cloud(world, x + 2, base_y + 7, z + 1, 2, leaves);
    world.place_block(x, base_y + 9, z, leaves);
}

/// Construye una nube de hojas esferica y organica
fn build_leaf_cloud(
    world: &mut VoxelWorld,
    cx: i32,
    cy: i32,
    cz: i32,
    radius: i32,
    material: Material,
) {
    for dx in -radius..=radius {
        for dy in -1..=1 {
            for dz in -radius..=radius {
                if dx.abs() == radius && dz.abs() == radius {
                    continue;
                }
                world.place_block(cx + dx, cy + dy, cz + dz, material);
            }
        }
    }
}

/// Nether: Fortaleza con almenas, portal clasico de obsidiana con
/// vidrio magenta translúcido/emisivo y lagunas de lava fluida.
fn build_nether_preview(world: &mut VoxelWorld, materials: &BlockMaterials) {
    let brick = materials.nether_bricks;
    let obsidian = materials.obsidian;
    let portal_glass = materials.magenta_glass;

    build_nether_lava_lake(world, materials);
    scatter_nether_floor_lava(world, materials);

    // Pasarela elevada de la fortaleza (x = -22, z de -9 a 9 a y = 3)
    for z in -9..=9 {
        world.place_block(-23, 3, z, brick);
        world.place_block(-22, 3, z, brick);
        world.place_block(-21, 3, z, brick);

        // Almenas alternadas en los barandales
        if z % 2 == 0 {
            world.place_block(-23, 4, z, brick);
            world.place_block(-21, 4, z, brick);
        }
    }

    // Pilares de soporte de la fortaleza bajando hacia el netherrack
    for pz in [-7, 0, 7] {
        fill_box(world, -23, -2, pz - 1, -21, 2, pz + 1, brick);
    }

    // Portal del Nether clasico (4 de ancho, 5 de alto), fuera del lago y a
    // su izquierda. La plataforma de netherrack lo separa del borde de lava.
    fill_box(world, -36, 1, -3, -34, 1, 4, materials.netherrack);
    for z in -1..=2 {
        world.place_block(-35, 1, z, obsidian);
        world.place_block(-35, 5, z, obsidian);
    }
    for y in 2..=4 {
        world.place_block(-35, y, -1, obsidian);
        world.place_block(-35, y, 2, obsidian);
    }
    // Interior del portal con vidrio magenta luminoso y refractante
    for y in 2..=4 {
        for z in 0..=1 {
            world.place_block(-35, y, z, portal_glass);
        }
    }
    // Pasarela de acceso que sube desde el portal hasta la fortaleza y cruza
    // el borde del lago sin alterar su superficie.
    for z in 0..=1 {
        world.place_block(-34, 1, z, brick);
        for x in -33..=-25 {
            world.place_block(x, 2, z, brick);
        }
        world.place_block(-24, 3, z, brick);
    }

}

/// Cubre toda la explanada plana del Nether con un lago de lava. Las
/// construcciones se añaden despues y sobresalen como islas sobre el liquido.
fn build_nether_lava_lake(world: &mut VoxelWorld, materials: &BlockMaterials) {
    for x in -33..=-18 {
        for z in -10..=10 {
            world.place_block(x, 1, z, materials.lava);
        }
    }

    // Seis vertientes de un bloque en el borde exterior del lago. Reemplazan
    // parte de la carcasa inferior y dejan la lava caer hacia el vacio.
    for (lava_x, lava_z) in [(-33, -9), (-33, -5), (-33, 5), (-33, 9), (-28, -10), (-20, -10)] {
        for y in -6..=0 {
            world.place_block(lava_x, y, lava_z, materials.lava);
        }
    }
}

/// Agrega charcos en huecos de un bloque del terreno irregular del Nether. El
/// patron es determinista: siempre luce organico, pero la escena se conserva
/// identica entre ejecuciones y no necesita una dependencia de aleatoriedad.
fn scatter_nether_floor_lava(world: &mut VoxelWorld, materials: &BlockMaterials) {
    for x in -ISLAND_RADIUS..=ISLAND_RADIUS {
        for z in -ISLAND_RADIUS..=ISLAND_RADIUS {
            if !is_nether_floor_lava_cell(x, z) {
                continue;
            }

            let hash = (x * 37 + z * 61 + x * z * 11).rem_euclid(29);
            if !matches!(hash, 0 | 7) {
                continue;
            }

            let surface_y = relief_height(x, z, 1);
            // Retira el bloque superior y baja la lava una unidad. Asi los
            // charcos quedan encajados en el suelo, en vez de parecer cubos
            // luminosos colocados sobre el netherrack.
            world.remove_block(x, surface_y, z);
            world.place_block(x, surface_y - 1, z, materials.lava);
        }
    }
}

/// Las zonas protegidas no reciben salpicaduras: ni el lago principal, ni el
/// portal, ni su pasarela, ni el corredor de cualquiera de los tres puentes.
fn is_nether_floor_lava_cell(x: i32, z: i32) -> bool {
    let inside_island = x * x + z * z <= ISLAND_RADIUS * ISLAND_RADIUS;
    let in_lake = (-33..=-18).contains(&x) && (-10..=10).contains(&z);
    let portal_or_access = (-36..=-24).contains(&x) && (-3..=4).contains(&z);

    inside_island
        && sector(x, z) == 1
        && !is_gap(x, z)
        && !is_statue_clearance(x, z)
        && !is_bridge_corridor(x, z)
        && !in_lake
        && !portal_or_access
}

/// End: 3 pilares de obsidiana de alturas asimetricas coronados con
/// cristales del End (protegidos con jaulas de vidrio), fuente central
/// y una pequena isla flotante en el vacio.
fn build_end_preview(world: &mut VoxelWorld, materials: &BlockMaterials) {
    let obsidian = materials.obsidian;
    let gold = materials.gold;
    let crystal = materials.magenta_glass;
    let glass = materials.glass;

    // Pilar 1 (Alto, altura y = 14) con cristal protegido en jaula de vidrio
    fill_box(world, 21, 1, -26, 23, 14, -24, obsidian);
    world.place_block(22, 15, -25, obsidian);
    world.place_block(22, 16, -25, materials.glowstone);
    world.place_block(22, 17, -25, crystal);
    // Jaula de vidrio alrededor del cristal
    for y in 15..=17 {
        for x in 21..=23 {
            for z in -26..=-24 {
                if x == 22 && z == -25 {
                    continue;
                }
                world.place_block(x, y, z, glass);
            }
        }
    }
    world.place_block(22, 18, -25, glass);

    // Pilar 2 (Medio, altura y = 10) con cristal abierto
    fill_box(world, 13, 1, -28, 15, 10, -26, obsidian);
    fill_box(world, 13, 10, -28, 15, 10, -26, gold);
    world.place_block(14, 11, -27, obsidian);
    world.place_block(14, 12, -27, crystal);
    world.place_block(14, 13, -27, materials.glowstone);

    // Pilar 3 (Bajo, altura y = 7) con cristal abierto
    fill_box(world, 25, 1, -17, 27, 7, -15, obsidian);
    fill_box(world, 25, 7, -17, 27, 7, -15, gold);
    world.place_block(26, 8, -16, obsidian);
    world.place_block(26, 9, -16, crystal);

    // Fuente / Altar del Portal de salida del End (Bedrock / Obsidian fountain)
    fill_disc(world, 1, 3, obsidian); // situado cerca de (16, -20)
    for x in 14..=18 {
        for z in -22..=-18 {
            if (x == 14 || x == 18) && (z == -22 || z == -18) {
                continue;
            }
            world.place_block(x, 1, z, materials.smooth_stone);
        }
    }
    world.place_block(16, 2, -20, obsidian);
    world.place_block(16, 3, -20, obsidian);
    world.place_block(16, 4, -20, materials.glowstone);

    // Pequeño asteroide / islote de End Stone flotando en el vacio
    fill_box(world, 31, 3, -25, 33, 4, -23, materials.end_stone);
    world.place_block(32, 2, -24, materials.end_stone);
    world.place_block(32, 5, -24, gold);
}

/// Torre central monumental tripode y tres puentes aéreos arqueados
/// que conectan de forma imponente las tres dimensiones a 120°.
fn build_central_statue_and_bridges(world: &mut VoxelWorld, materials: &BlockMaterials) {
    build_monumental_tower(world, materials);
    build_tripartite_arched_bridges(world, materials);
}

/// Torre inspirada en la Torre Eiffel y obeliscos escalonados:
/// base ancha calada, balcon mirador, cuerpo piramidal y aguja luminosa.
fn build_monumental_tower(world: &mut VoxelWorld, materials: &BlockMaterials) {
    let stone = materials.stone;
    let smooth = materials.smooth_stone;
    let dark = materials.black_terracotta;
    let gold = materials.gold;
    let glass = materials.glass;

    // Nivel 0-3: Plaza circular central escalonada
    fill_disc(world, -1, 6, dark);
    fill_disc(world, 0, 6, smooth);
    fill_disc(world, 1, 5, smooth);
    fill_disc(world, 2, 5, stone);
    fill_disc(world, 3, 4, smooth);

    // Nivel 4-11: Base de la torre con 3 grandes portales arqueados
    // para caminar libremente hacia los tres puentes
    for y in 4..=11 {
        for x in -4_i32..=4_i32 {
            for z in -4_i32..=4_i32 {
                let dist_sq = x * x + z * z;
                if dist_sq > 18 {
                    continue;
                }
                // Dejar columnas y esquinas sólidas
                let is_corner = (x.abs() == 4 && z.abs() >= 2) || (z.abs() == 4 && x.abs() >= 2);
                let is_pillar = (x.abs() == 3 && z.abs() == 3) || (x.abs() == 2 && z.abs() == 2);
                if is_corner || is_pillar || dist_sq <= 2 {
                    let mat = if is_corner { dark } else { stone };
                    world.place_block(x, y, z, mat);
                }
            }
        }
    }

    // Nivel 12-17: Balcón mirador intermedio y cuerpo medio
    fill_box(world, -4, 12, -4, 4, 12, 4, smooth);
    for x in -4_i32..=4_i32 {
        for z in -4_i32..=4_i32 {
            if x.abs() == 4 || z.abs() == 4 {
                world.place_block(x, 13, z, dark); // Barandilla del balcon
            }
        }
    }
    // Paredes del cuerpo medio con ventanas de vidrio
    for y in 13..=17 {
        for x in -2_i32..=2_i32 {
            for z in -2_i32..=2_i32 {
                if x.abs() == 2 || z.abs() == 2 {
                    if (x.abs() + z.abs() == 2) && (y == 14 || y == 15) {
                        world.place_block(x, y, z, glass);
                    } else {
                        world.place_block(x, y, z, stone);
                    }
                }
            }
        }
    }
    // Lampara del mirador. La glowstone queda dentro del cuerpo de la torre y
    // proyecta luz calida hacia sus cuatro ventanas de vidrio.
    world.place_block(0, 14, 0, materials.glowstone);

    // Nivel 18-25: Cuerpo superior escalonado piramidal
    fill_box(world, -3, 18, -3, 3, 18, 3, smooth);
    fill_box(world, -2, 19, -2, 2, 22, 2, stone);
    fill_box(world, -2, 23, -2, 2, 23, 2, gold);
    fill_box(world, -1, 24, -1, 1, 26, 1, smooth);

    // Nivel 27-34: Aguja estilizada y faro de la cuspide
    fill_box(world, -1, 27, -1, 1, 27, 1, gold);
    for y in 28..=31 {
        world.place_block(0, y, 0, gold);
    }
    world.place_block(0, 32, 0, materials.glowstone);
    world.place_block(0, 33, 0, materials.magenta_glass);
    world.place_block(0, 34, 0, materials.gold);
}

/// Construye tres puentes volados con arco parabolico hacia
/// los centros respectivos de Overworld (60°), Nether (180°) y End (300°).
fn build_tripartite_arched_bridges(world: &mut VoxelWorld, materials: &BlockMaterials) {
    let destinations = [
        (11_i32, 19_i32), // Overworld (60°)
        (-22, 0),         // Nether (180°)
        (11, -19),        // End (300°)
    ];

    for (end_x, end_z) in destinations {
        build_single_arched_bridge(world, end_x, end_z, materials);
    }
}

/// Construye un puente individual que cruza el abismo radial en un arco continuo.
fn build_single_arched_bridge(
    world: &mut VoxelWorld,
    end_x: i32,
    end_z: i32,
    materials: &BlockMaterials,
) {
    let total_steps = 22;
    let stone = materials.stone;
    let smooth = materials.smooth_stone;
    let rail_mat = materials.black_terracotta;

    // Vector perpendicular en el plano horizontal para dar anchura al puente
    let side_x = if end_x.abs() >= end_z.abs() { 0 } else { 1 };
    let side_z = if end_x.abs() >= end_z.abs() { 1 } else { 0 };

    for step in 4..=total_steps {
        let t = step as f32 / total_steps as f32;
        let cx = (end_x as f32 * t).round() as i32;
        let cz = (end_z as f32 * t).round() as i32;
        let dist = ((cx * cx + cz * cz) as f32).sqrt();

        // Arco parabolico: se eleva sobre el vacio a y=6 y desciende a y=2
        let arch_lift = (t * std::f32::consts::PI).sin() * 2.8;
        let deck_y = (4.2 + arch_lift - 2.2 * t).round() as i32;

        // Construir el ancho del puente (3 bloques de ancho)
        for offset in -1..=1 {
            let bx = cx + side_x * offset;
            let bz = cz + side_z * offset;

            // Tablero transitable
            world.place_block(bx, deck_y, bz, smooth);
            world.place_block(bx, deck_y - 1, bz, stone);

            // En los extremos del puente, las bases tocan tierra
            if dist >= 18.0 || dist <= 5.5 {
                world.place_block(bx, deck_y - 2, bz, stone);
            }

            // Barandales en los bordes laterales
            if offset != 0 {
                world.place_block(bx, deck_y + 1, bz, rail_mat);
            }
        }

        // Faroles luminosos en las entradas y llegadas del puente
        if step == 5 || step == 21 {
            for offset in [-1, 1] {
                let lx = cx + side_x * offset;
                let lz = cz + side_z * offset;
                world.place_block(lx, deck_y + 1, lz, stone);
                world.place_block(lx, deck_y + 2, lz, materials.glowstone);
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_dimension_has_a_non_flat_relief_profile() {
        for expected_sector in 0..3 {
            let heights: Vec<_> = (-ISLAND_RADIUS..=ISLAND_RADIUS)
                .flat_map(|x| (-ISLAND_RADIUS..=ISLAND_RADIUS).map(move |z| (x, z)))
                .filter(|(x, z)| {
                    x * x + z * z <= ISLAND_RADIUS * ISLAND_RADIUS
                        && sector(*x, *z) == expected_sector
                })
                .map(|(x, z)| relief_height(x, z, expected_sector))
                .collect();

            assert!(heights.iter().any(|height| *height > 0));
            assert!(heights.iter().min() < heights.iter().max());
        }
    }

    #[test]
    fn landmarks_and_bridge_accesses_stay_level() {
        assert!(is_flattened_area(18, 18));
        assert!(is_flattened_area(-25, 0));
        assert!(is_flattened_area(16, -23));
        assert!(is_flattened_area(8, 15));
    }
}
