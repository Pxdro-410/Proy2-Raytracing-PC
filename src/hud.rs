use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::materials::BlockMaterials;
use crate::ray_intersect::{BlockFace, Material};
use crate::texture::{TextureId, TextureLibrary};

const SLOT_COUNT: usize = 6;
const SLOT_SCALE: usize = 2;
const SLOT_SIZE: usize = 20 * SLOT_SCALE;
const BAR_WIDTH: usize = 120;
const BAR_HEIGHT: usize = 20;
const SELECTOR_SIZE: usize = 22;
const CROSSHAIR_SIZE: usize = 9;

#[derive(Clone, Copy)]
enum HotbarSlot {
    Block(Material),
    Remove,
    Copy(Option<Material>),
}

#[derive(Clone, Copy)]
pub enum HotbarAction {
    Place(Material),
    Remove,
    None,
}

/// Inventario infinito de seis posiciones para la construccion creativa.
pub struct Hotbar {
    slots: [HotbarSlot; SLOT_COUNT],
    selected: usize,
}

impl Hotbar {
    pub fn new(materials: &BlockMaterials) -> Self {
        Self {
            slots: [
                HotbarSlot::Block(materials.coarse_dirt),
                HotbarSlot::Block(materials.oak_log),
                HotbarSlot::Block(materials.gold),
                HotbarSlot::Block(materials.glass),
                HotbarSlot::Remove,
                HotbarSlot::Copy(None),
            ],
            selected: 0,
        }
    }

    pub fn select(&mut self, slot: usize) -> bool {
        if slot < SLOT_COUNT && self.selected != slot {
            self.selected = slot;
            true
        } else {
            false
        }
    }

    pub fn selected_action(&self) -> HotbarAction {
        match self.slots[self.selected] {
            HotbarSlot::Block(material) | HotbarSlot::Copy(Some(material)) => {
                HotbarAction::Place(material)
            }
            HotbarSlot::Remove => HotbarAction::Remove,
            HotbarSlot::Copy(None) => HotbarAction::None,
        }
    }

    pub fn copy_material(&mut self, material: Material) {
        self.slots[5] = HotbarSlot::Copy(Some(material));
        self.selected = 5;
    }
}

pub fn draw(framebuffer: &mut Framebuffer, hotbar: &Hotbar, textures: &TextureLibrary) {
    draw_crosshair(framebuffer, textures);
    draw_hotbar(framebuffer, hotbar, textures);
}

fn draw_crosshair(framebuffer: &mut Framebuffer, textures: &TextureLibrary) {
    let width = CROSSHAIR_SIZE * SLOT_SCALE;
    let height = CROSSHAIR_SIZE * SLOT_SCALE;
    let x = (framebuffer.width.saturating_sub(width)) / 2;
    let y = (framebuffer.height.saturating_sub(height)) / 2;
    draw_texture_sprite(
        framebuffer,
        textures,
        TextureId::Crosshair,
        CROSSHAIR_SIZE,
        CROSSHAIR_SIZE,
        x,
        y,
    );
}

fn draw_hotbar(framebuffer: &mut Framebuffer, hotbar: &Hotbar, textures: &TextureLibrary) {
    let bar_width = BAR_WIDTH * SLOT_SCALE;
    let origin_x = (framebuffer.width.saturating_sub(bar_width)) / 2;
    let origin_y = framebuffer
        .height
        .saturating_sub(BAR_HEIGHT * SLOT_SCALE + 12);

    draw_texture_sprite(
        framebuffer,
        textures,
        TextureId::ItemBar,
        BAR_WIDTH,
        BAR_HEIGHT,
        origin_x,
        origin_y,
    );
    for (index, slot) in hotbar.slots.iter().enumerate() {
        let x = origin_x + index * SLOT_SIZE;
        match slot {
            HotbarSlot::Block(material) | HotbarSlot::Copy(Some(material)) => {
                draw_material_icon(framebuffer, textures, x + 12, origin_y + 12, *material)
            }
            HotbarSlot::Remove => {
                draw_remove_icon(framebuffer, x as i32 + 12, origin_y as i32 + 12)
            }
            HotbarSlot::Copy(None) => {
                draw_copy_icon(framebuffer, x as i32 + 12, origin_y as i32 + 12)
            }
        }
    }
    let selected_x = origin_x + hotbar.selected * SLOT_SIZE - SLOT_SCALE;
    let selected_y = origin_y.saturating_sub(SLOT_SCALE);
    draw_texture_sprite(
        framebuffer,
        textures,
        TextureId::ItemSelected,
        SELECTOR_SIZE,
        SELECTOR_SIZE,
        selected_x,
        selected_y,
    );
}

/// Dibuja una textura del HUD respetando su transparencia real.
fn draw_texture_sprite(
    framebuffer: &mut Framebuffer,
    textures: &TextureLibrary,
    texture: TextureId,
    source_width: usize,
    source_height: usize,
    target_x: usize,
    target_y: usize,
) {
    for source_y in 0..source_height {
        for source_x in 0..source_width {
            let u = (source_x as f32 + 0.5) / source_width as f32;
            let v = 1.0 - (source_y as f32 + 0.5) / source_height as f32;
            let alpha = textures.sample_alpha(texture, u, v);
            if alpha <= 0.0 {
                continue;
            }
            let color = textures
                .sample(texture, u, v, Color::new(40, 32, 18))
                .to_hex();
            for dy in 0..SLOT_SCALE {
                for dx in 0..SLOT_SCALE {
                    blend_pixel(
                        framebuffer,
                        (target_x + source_x * SLOT_SCALE + dx) as i32,
                        (target_y + source_y * SLOT_SCALE + dy) as i32,
                        color,
                        alpha,
                    );
                }
            }
        }
    }
}

fn draw_material_icon(
    framebuffer: &mut Framebuffer,
    textures: &TextureLibrary,
    x: usize,
    y: usize,
    material: Material,
) {
    const ICON_SIZE: usize = 16;
    let texture = material.texture_for(BlockFace::PositiveY);
    for icon_y in 0..ICON_SIZE {
        for icon_x in 0..ICON_SIZE {
            let u = (icon_x as f32 + 0.5) / ICON_SIZE as f32;
            let v = (icon_y as f32 + 0.5) / ICON_SIZE as f32;
            if textures.sample_alpha(texture, u, v) < material.alpha_cutoff {
                continue;
            }
            let color = textures.sample(texture, u, v, material.diffuse).to_hex();
            put_pixel(framebuffer, (x + icon_x) as i32, (y + icon_y) as i32, color);
        }
    }
    outline_rect(
        framebuffer,
        x as i32 - 1,
        y as i32 - 1,
        ICON_SIZE + 2,
        ICON_SIZE + 2,
        0x111111,
    );
}

fn draw_remove_icon(framebuffer: &mut Framebuffer, x: i32, y: i32) {
    for offset in 0..16_i32 {
        put_pixel(framebuffer, x + offset, y + offset, 0xc73b38);
        put_pixel(framebuffer, x + 15 - offset, y + offset, 0xc73b38);
    }
}

fn draw_copy_icon(framebuffer: &mut Framebuffer, x: i32, y: i32) {
    outline_rect(framebuffer, x + 3, y, 12, 12, 0x8eb5d1);
    outline_rect(framebuffer, x, y + 4, 12, 12, 0xd8e5ee);
}

fn outline_rect(
    framebuffer: &mut Framebuffer,
    x: i32,
    y: i32,
    width: usize,
    height: usize,
    color: u32,
) {
    for offset in 0..width {
        put_pixel(framebuffer, x + offset as i32, y, color);
        put_pixel(framebuffer, x + offset as i32, y + height as i32 - 1, color);
    }
    for offset in 0..height {
        put_pixel(framebuffer, x, y + offset as i32, color);
        put_pixel(framebuffer, x + width as i32 - 1, y + offset as i32, color);
    }
}

fn blend_pixel(framebuffer: &mut Framebuffer, x: i32, y: i32, foreground: u32, alpha: f32) {
    if x < 0 || y < 0 || (x as usize) >= framebuffer.width || (y as usize) >= framebuffer.height {
        return;
    }
    let index = y as usize * framebuffer.width + x as usize;
    let background = framebuffer.buffer[index];
    let inverse_alpha = 1.0 - alpha;
    let channel = |shift| {
        (((foreground >> shift) & 0xff_u32) as f32 * alpha
            + ((background >> shift) & 0xff_u32) as f32 * inverse_alpha) as u32
    };
    framebuffer.buffer[index] = (channel(16) << 16) | (channel(8) << 8) | channel(0);
}

fn put_pixel(framebuffer: &mut Framebuffer, x: i32, y: i32, color: u32) {
    if x >= 0 && y >= 0 && (x as usize) < framebuffer.width && (y as usize) < framebuffer.height {
        framebuffer.buffer[y as usize * framebuffer.width + x as usize] = color;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copied_block_replaces_slot_six_and_becomes_selected() {
        let materials = BlockMaterials::new();
        let mut hotbar = Hotbar::new(&materials);

        assert!(matches!(hotbar.selected_action(), HotbarAction::Place(_)));
        assert!(hotbar.select(5));
        assert!(matches!(hotbar.selected_action(), HotbarAction::None));

        hotbar.copy_material(materials.gold);
        assert!(matches!(hotbar.selected_action(), HotbarAction::Place(_)));
    }
}
