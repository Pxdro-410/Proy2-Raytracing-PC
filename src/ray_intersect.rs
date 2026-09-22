use crate::color::Color;
use crate::texture::TextureId;
use crate::vec3::Vec3;

#[derive(Debug, Clone, Copy)]
pub enum BlockFace {
    NegativeX,
    PositiveX,
    NegativeY,
    PositiveY,
    NegativeZ,
    PositiveZ,
}

impl BlockFace {
    pub fn from_normal(normal: Vec3) -> Self {
        if normal.x < -0.5 {
            Self::NegativeX
        } else if normal.x > 0.5 {
            Self::PositiveX
        } else if normal.y < -0.5 {
            Self::NegativeY
        } else if normal.y > 0.5 {
            Self::PositiveY
        } else if normal.z < -0.5 {
            Self::NegativeZ
        } else {
            Self::PositiveZ
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::NegativeX => 0,
            Self::PositiveX => 1,
            Self::NegativeY => 2,
            Self::PositiveY => 3,
            Self::NegativeZ => 4,
            Self::PositiveZ => 5,
        }
    }

    pub fn uv(self, point: Vec3, block_min: Vec3) -> (f32, f32) {
        let local = point - block_min;
        match self {
            Self::NegativeX => (1.0 - local.z, local.y),
            Self::PositiveX => (local.z, local.y),
            Self::NegativeY => (local.x, 1.0 - local.z),
            Self::PositiveY => (local.x, local.z),
            Self::NegativeZ => (local.x, local.y),
            Self::PositiveZ => (1.0 - local.x, local.y),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub diffuse: Color,
    pub albedo: f32,
    pub specular: f32,
    pub specular_strength: f32,
    pub reflectivity: f32,
    pub transparency: f32,
    pub refractive_index: f32,
    pub alpha_cutoff: f32,
    pub uses_texture_alpha: bool,
    pub world_uv_scale: f32,
    pub emission: Color,
    pub emission_strength: f32,
    textures: [TextureId; 6],
}

impl Material {
    pub fn new(
        diffuse: Color,
        albedo: f32,
        specular: f32,
        reflectivity: f32,
        transparency: f32,
        refractive_index: f32,
    ) -> Self {
        Self {
            diffuse,
            albedo,
            specular,
            specular_strength: 0.0,
            reflectivity,
            transparency,
            refractive_index,
            alpha_cutoff: 0.0,
            uses_texture_alpha: false,
            world_uv_scale: 0.0,
            emission: Color::new(0, 0, 0),
            emission_strength: 0.0,
            textures: [TextureId::Solid; 6],
        }
    }

    pub fn textured(
        diffuse: Color,
        albedo: f32,
        specular: f32,
        reflectivity: f32,
        transparency: f32,
        refractive_index: f32,
        textures: [TextureId; 6],
    ) -> Self {
        Self {
            diffuse,
            albedo,
            specular,
            specular_strength: 0.0,
            reflectivity,
            transparency,
            refractive_index,
            alpha_cutoff: 0.0,
            uses_texture_alpha: false,
            world_uv_scale: 0.0,
            emission: Color::new(0, 0, 0),
            emission_strength: 0.0,
            textures,
        }
    }

    pub fn with_emission(mut self, emission: Color, strength: f32) -> Self {
        self.emission = emission;
        self.emission_strength = strength;
        self
    }

    pub fn with_specular_strength(mut self, strength: f32) -> Self {
        self.specular_strength = strength.clamp(0.0, 1.0);
        self
    }

    pub fn with_alpha_cutoff(mut self, cutoff: f32) -> Self {
        self.alpha_cutoff = cutoff.clamp(0.0, 1.0);
        self
    }

    pub fn with_texture_alpha_transparency(mut self) -> Self {
        self.uses_texture_alpha = true;
        self
    }

    /// Proyecta la textura en coordenadas globales para que una superficie de
    /// varios voxeles no muestre una junta por bloque.
    pub fn with_world_uv_scale(mut self, scale: f32) -> Self {
        self.world_uv_scale = scale.max(0.0);
        self
    }

    pub fn texture_for(&self, face: BlockFace) -> TextureId {
        self.textures[face.index()]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Intersect {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    pub material: Material,
    pub face: BlockFace,
    pub uv: (f32, f32),
}

pub trait RayIntersect: Sync {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<Intersect>;
}
