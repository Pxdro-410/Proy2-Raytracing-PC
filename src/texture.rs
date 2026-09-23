use std::fs;
use std::path::Path;

use crate::color::Color;

/// Identificador compacto de las texturas disponibles. `Solid` conserva el
/// color plano de reserva para materiales que no usan imagen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum TextureId {
    Solid,
    GrassTop,
    GrassSide,
    CoarseDirt,
    Cobblestone,
    MossyCobblestone,
    Stone,
    SmoothStone,
    OakLog,
    OakLogTop,
    CherryLog,
    CherryLogTop,
    CherryLeaves,
    Netherrack,
    NetherBricks,
    Glowstone,
    Lava,
    EndStone,
    BlackTerracotta,
    BlackWool,
    Gold,
    GoldTop,
    Glass,
    MagentaGlass,
    Water,
    Crosshair,
    ItemBar,
    ItemSelected,
}

const TEXTURE_COUNT: usize = TextureId::ItemSelected as usize + 1;

#[derive(Clone)]
struct Texture {
    width: usize,
    height: usize,
    pixels: Vec<Color>,
    alpha: Vec<u8>,
}

impl Texture {
    fn from_ppm(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path)
            .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?;
        let mut cursor = 0;
        let signature = next_token(&bytes, &mut cursor).ok_or("PPM sin firma")?;
        if signature != b"P6" {
            return Err(format!("{} no es un PPM binario P6", path.display()));
        }
        let width = parse_dimension(next_token(&bytes, &mut cursor), path, "ancho")?;
        let height = parse_dimension(next_token(&bytes, &mut cursor), path, "alto")?;
        let max_value = parse_dimension(next_token(&bytes, &mut cursor), path, "rango")?;
        if max_value != 255 {
            return Err(format!("{} debe usar rango 255", path.display()));
        }
        if cursor >= bytes.len() || !bytes[cursor].is_ascii_whitespace() {
            return Err(format!(
                "{} tiene una cabecera PPM incompleta",
                path.display()
            ));
        }
        // El formato P6 deja exactamente un separador entre la cabecera y RGB.
        cursor += 1;
        let expected = width * height * 3;
        if bytes.len().saturating_sub(cursor) < expected {
            return Err(format!("{} no contiene todos sus pixeles", path.display()));
        }
        let pixels = bytes[cursor..cursor + expected]
            .chunks_exact(3)
            .map(|rgb| Color::new(rgb[0], rgb[1], rgb[2]))
            .collect();
        Ok(Self {
            width,
            height,
            pixels,
            alpha: vec![255; width * height],
        })
    }

    fn sample(&self, u: f32, v: f32) -> Color {
        // Cada bloque recibe una copia completa de la imagen: no hay estirado
        // entre bloques vecinos ni animacion de agua/lava.
        self.pixels[self.pixel_index(u, v)]
    }

    fn sample_alpha(&self, u: f32, v: f32) -> f32 {
        self.alpha[self.pixel_index(u, v)] as f32 / 255.0
    }

    fn load_alpha(&mut self, path: &Path) -> Result<(), String> {
        let bytes = fs::read(path)
            .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?;
        let mut cursor = 0;
        let signature = next_token(&bytes, &mut cursor).ok_or("PGM sin firma")?;
        if signature != b"P5" {
            return Err(format!("{} no es un PGM binario P5", path.display()));
        }
        let width = parse_dimension(next_token(&bytes, &mut cursor), path, "ancho")?;
        let height = parse_dimension(next_token(&bytes, &mut cursor), path, "alto")?;
        let max_value = parse_dimension(next_token(&bytes, &mut cursor), path, "rango")?;
        if width != self.width || height != self.height || max_value != 255 {
            return Err(format!("{} no coincide con su textura RGB", path.display()));
        }
        if cursor >= bytes.len() || !bytes[cursor].is_ascii_whitespace() {
            return Err(format!(
                "{} tiene una cabecera PGM incompleta",
                path.display()
            ));
        }
        cursor += 1;
        let expected = width * height;
        if bytes.len().saturating_sub(cursor) < expected {
            return Err(format!("{} no contiene todos sus pixeles", path.display()));
        }
        self.alpha
            .copy_from_slice(&bytes[cursor..cursor + expected]);
        Ok(())
    }

    fn pixel_index(&self, u: f32, v: f32) -> usize {
        // como agua usan coordenadas globales que deben repetirse sin cortes.
        let u = u.rem_euclid(1.0);
        let v = v.rem_euclid(1.0);
        let x = (u * self.width as f32) as usize;
        let y = ((1.0 - v) * self.height as f32) as usize;
        y.min(self.height - 1) * self.width + x.min(self.width - 1)
    }
}

pub struct TextureLibrary {
    textures: Vec<Option<Texture>>,
}

impl TextureLibrary {
    pub fn load_default() -> Result<Self, String> {
        let mut library = Self {
            textures: vec![None; TEXTURE_COUNT],
        };
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("ppm");
        for (id, name) in [
            (TextureId::GrassTop, "grass_block_top"),
            (TextureId::GrassSide, "grass_block_side"),
            (TextureId::CoarseDirt, "coarse_dirt"),
            (TextureId::Cobblestone, "cobblestone"),
            (TextureId::MossyCobblestone, "mossy_cobblestone"),
            (TextureId::Stone, "stone"),
            (TextureId::SmoothStone, "smooth_stone"),
            (TextureId::OakLog, "oak_log"),
            (TextureId::OakLogTop, "oak_log_top"),
            (TextureId::CherryLog, "cherry_log"),
            (TextureId::CherryLogTop, "cherry_log_top"),
            (TextureId::CherryLeaves, "cherry_leaves"),
            (TextureId::Netherrack, "netherrack"),
            (TextureId::NetherBricks, "red_nether_bricks"),
            (TextureId::Glowstone, "glowstone"),
            (TextureId::Lava, "lava_still"),
            (TextureId::EndStone, "end_stone"),
            (TextureId::BlackTerracotta, "black_terracotta"),
            (TextureId::BlackWool, "black_wool"),
            (TextureId::Gold, "gold_block"),
            (TextureId::GoldTop, "gold_block_top"),
            (TextureId::Glass, "glass"),
            (TextureId::MagentaGlass, "magenta_stained_glass"),
            (TextureId::Water, "water_still"),
            (TextureId::Crosshair, "crosshair"),
            (TextureId::ItemBar, "item_bar"),
            (TextureId::ItemSelected, "item_selected"),
        ] {
            let mut texture = Texture::from_ppm(&root.join(format!("{name}.ppm")))?;
            let alpha_path = root.join(format!("{name}.pgm"));
            if alpha_path.exists() {
                texture.load_alpha(&alpha_path)?;
            }
            library.textures[id as usize] = Some(texture);
        }
        Ok(library)
    }

    pub fn sample(&self, texture: TextureId, u: f32, v: f32, fallback: Color) -> Color {
        self.textures[texture as usize]
            .as_ref()
            .map(|image| image.sample(u, v))
            .unwrap_or(fallback)
    }

    pub fn sample_alpha(&self, texture: TextureId, u: f32, v: f32) -> f32 {
        self.textures[texture as usize]
            .as_ref()
            .map(|image| image.sample_alpha(u, v))
            .unwrap_or(1.0)
    }
}

fn next_token<'a>(bytes: &'a [u8], cursor: &mut usize) -> Option<&'a [u8]> {
    loop {
        while *cursor < bytes.len() && bytes[*cursor].is_ascii_whitespace() {
            *cursor += 1;
        }
        if *cursor < bytes.len() && bytes[*cursor] == b'#' {
            while *cursor < bytes.len() && bytes[*cursor] != b'\n' {
                *cursor += 1;
            }
            continue;
        }
        break;
    }
    let start = *cursor;
    while *cursor < bytes.len() && !bytes[*cursor].is_ascii_whitespace() {
        *cursor += 1;
    }
    (start < *cursor).then_some(&bytes[start..*cursor])
}

fn parse_dimension(token: Option<&[u8]>, path: &Path, field: &str) -> Result<usize, String> {
    let token = token.ok_or_else(|| format!("{} no contiene {field}", path.display()))?;
    std::str::from_utf8(token)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &usize| *value > 0)
        .ok_or_else(|| format!("{} tiene {field} invalido", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_textures_load_and_are_sampled() {
        let library = TextureLibrary::load_default().expect("las texturas PPM deben cargar");
        let fallback = Color::new(1, 2, 3);
        assert_ne!(
            library
                .sample(TextureId::GrassTop, 0.5, 0.5, fallback)
                .to_hex(),
            fallback.to_hex()
        );
    }

    #[test]
    fn cherry_leaves_keep_their_png_cutout_alpha() {
        let library = TextureLibrary::load_default().expect("las texturas PPM deben cargar");
        let leaves = library.textures[TextureId::CherryLeaves as usize]
            .as_ref()
            .expect("las hojas deben existir");

        assert!(leaves.alpha.iter().any(|alpha| *alpha == 0));
        assert!(leaves.alpha.iter().any(|alpha| *alpha == 255));
    }

    #[test]
    fn glass_keeps_its_partial_alpha_mask() {
        let library = TextureLibrary::load_default().expect("las texturas PPM deben cargar");
        let glass = library.textures[TextureId::Glass as usize]
            .as_ref()
            .expect("el vidrio debe existir");

        assert!(glass.alpha.iter().any(|alpha| *alpha < 255));
        assert!(glass.alpha.iter().any(|alpha| *alpha > 0));
    }
}
