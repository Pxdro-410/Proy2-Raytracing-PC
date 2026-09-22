use crate::color::Color;
use crate::ray_intersect::Material;
use crate::texture::TextureId;

/// Catalogo de bloques reutilizable. Centraliza tanto la apariencia como las
/// propiedades de iluminacion de la isla.
pub struct BlockMaterials {
    pub grass: Material,
    pub coarse_dirt: Material,
    pub cobblestone: Material,
    pub mossy_cobblestone: Material,
    pub stone: Material,
    pub smooth_stone: Material,
    pub oak_log: Material,
    pub cherry_log: Material,
    pub cherry_leaves: Material,
    pub netherrack: Material,
    pub nether_bricks: Material,
    pub glowstone: Material,
    pub lava: Material,
    pub end_stone: Material,
    pub black_terracotta: Material,
    pub black_wool: Material,
    pub gold: Material,
    pub glass: Material,
    pub magenta_glass: Material,
    pub water: Material,
}

impl BlockMaterials {
    pub fn new() -> Self {
        let opaque = |texture| {
            Material::textured(
                Color::new(255, 255, 255),
                0.9,
                10.0,
                0.0,
                0.0,
                1.0,
                [texture; 6],
            )
        };
        Self {
            grass: Material::textured(
                Color::new(255, 255, 255),
                0.9,
                10.0,
                0.0,
                0.0,
                1.0,
                [
                    TextureId::GrassSide,
                    TextureId::GrassSide,
                    TextureId::CoarseDirt,
                    TextureId::GrassTop,
                    TextureId::GrassSide,
                    TextureId::GrassSide,
                ],
            ),
            coarse_dirt: opaque(TextureId::CoarseDirt),
            cobblestone: opaque(TextureId::Cobblestone),
            mossy_cobblestone: opaque(TextureId::MossyCobblestone),
            stone: opaque(TextureId::Stone),
            smooth_stone: opaque(TextureId::SmoothStone),
            oak_log: Material::textured(
                Color::new(255, 255, 255),
                0.9,
                10.0,
                0.0,
                0.0,
                1.0,
                [
                    TextureId::OakLog,
                    TextureId::OakLog,
                    TextureId::OakLogTop,
                    TextureId::OakLogTop,
                    TextureId::OakLog,
                    TextureId::OakLog,
                ],
            ),
            cherry_log: Material::textured(
                Color::new(255, 255, 255),
                0.9,
                10.0,
                0.0,
                0.0,
                1.0,
                [
                    TextureId::CherryLog,
                    TextureId::CherryLog,
                    TextureId::CherryLogTop,
                    TextureId::CherryLogTop,
                    TextureId::CherryLog,
                    TextureId::CherryLog,
                ],
            ),
            cherry_leaves: Material::textured(
                Color::new(255, 255, 255),
                0.9,
                10.0,
                0.0,
                0.0,
                1.0,
                [TextureId::CherryLeaves; 6],
            )
            .with_alpha_cutoff(0.1),
            netherrack: opaque(TextureId::Netherrack),
            nether_bricks: opaque(TextureId::NetherBricks),
            glowstone: Material::textured(
                Color::new(255, 255, 255),
                1.0,
                16.0,
                0.0,
                0.0,
                1.0,
                [TextureId::Glowstone; 6],
            )
            .with_emission(Color::new(255, 202, 104), 0.8),
            lava: Material::textured(
                Color::new(255, 255, 255),
                1.0,
                18.0,
                0.05,
                0.0,
                1.0,
                [TextureId::Lava; 6],
            )
            .with_emission(Color::new(255, 76, 20), 1.15)
            .with_specular_strength(0.05),
            end_stone: opaque(TextureId::EndStone),
            black_terracotta: opaque(TextureId::BlackTerracotta),
            black_wool: opaque(TextureId::BlackWool),
            gold: Material::textured(
                Color::new(255, 255, 255),
                0.85,
                40.0,
                0.15,
                0.0,
                1.0,
                [
                    TextureId::Gold,
                    TextureId::Gold,
                    TextureId::Gold,
                    TextureId::GoldTop,
                    TextureId::Gold,
                    TextureId::Gold,
                ],
            )
            .with_specular_strength(0.65),
            glass: Material::textured(
                Color::new(255, 255, 255),
                0.35,
                64.0,
                0.04,
                0.82,
                1.45,
                [TextureId::Glass; 6],
            )
            .with_specular_strength(0.35)
            .with_texture_alpha_transparency(),
            magenta_glass: Material::textured(
                Color::new(255, 255, 255),
                0.4,
                64.0,
                0.05,
                0.7,
                1.45,
                [TextureId::MagentaGlass; 6],
            )
            .with_emission(Color::new(235, 90, 210), 0.35)
            .with_specular_strength(0.35)
            .with_texture_alpha_transparency(),
            water: Material::textured(
                Color::new(255, 255, 255),
                0.7,
                48.0,
                0.12,
                0.5,
                1.33,
                [TextureId::Water; 6],
            )
            .with_specular_strength(0.6),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn house_glass_uses_high_transparency_and_its_texture_alpha() {
        let materials = BlockMaterials::new();

        assert!(materials.glass.transparency >= 0.8);
        assert!(materials.glass.uses_texture_alpha);
    }
}
