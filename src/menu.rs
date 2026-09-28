use std::fs;
use std::path::{Path, PathBuf};

use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::texture::{TextureId, TextureLibrary};

const BUTTON_SOURCE_WIDTH: usize = 200;
const BUTTON_SOURCE_HEIGHT: usize = 20;
const BUTTON_SCALE: usize = 2;
const BUTTON_WIDTH: usize = BUTTON_SOURCE_WIDTH * BUTTON_SCALE;
const BUTTON_HEIGHT: usize = BUTTON_SOURCE_HEIGHT * BUTTON_SCALE;
const BUTTON_X: usize = 280;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuPage {
    Main,
    HowToPlay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    None,
    JoinWorld,
    HowToPlay,
    Back,
    Exit,
}

#[derive(Clone, Copy)]
struct ButtonRect {
    x: usize,
    y: usize,
}

impl ButtonRect {
    fn contains(self, x: usize, y: usize) -> bool {
        (self.x..self.x + BUTTON_WIDTH).contains(&x)
            && (self.y..self.y + BUTTON_HEIGHT).contains(&y)
    }
}

struct Background {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
}

/// Menú principal independiente del raytracer. Su fondo es la última captura
/// PPM producida con P; mientras no exista, usa un degradado oscuro temporal.
pub struct Menu {
    background: Option<Background>,
}

impl Menu {
    pub fn load() -> Self {
        Self {
            background: load_background().ok(),
        }
    }

    pub fn reload_background(&mut self) {
        self.background = load_background().ok();
    }

    pub fn draw(
        &self,
        page: MenuPage,
        framebuffer: &mut Framebuffer,
        textures: &TextureLibrary,
        mouse: Option<(usize, usize)>,
        mouse_down: bool,
    ) {
        self.draw_background(framebuffer);
        match page {
            MenuPage::Main => self.draw_main(framebuffer, textures, mouse, mouse_down),
            MenuPage::HowToPlay => self.draw_how_to_play(framebuffer, textures, mouse, mouse_down),
        }
    }

    pub fn click(&self, page: MenuPage, mouse: Option<(usize, usize)>) -> MenuAction {
        let Some((mouse_x, mouse_y)) = mouse else {
            return MenuAction::None;
        };
        match page {
            MenuPage::Main if main_buttons()[0].contains(mouse_x, mouse_y) => MenuAction::JoinWorld,
            MenuPage::Main if main_buttons()[1].contains(mouse_x, mouse_y) => MenuAction::HowToPlay,
            MenuPage::Main if main_buttons()[2].contains(mouse_x, mouse_y) => MenuAction::Exit,
            MenuPage::HowToPlay if back_button().contains(mouse_x, mouse_y) => MenuAction::Back,
            _ => MenuAction::None,
        }
    }

    fn draw_background(&self, framebuffer: &mut Framebuffer) {
        for y in 0..framebuffer.height {
            for x in 0..framebuffer.width {
                let color = self
                    .background
                    .as_ref()
                    .map(|background| {
                        let source_x = x * background.width / framebuffer.width;
                        let source_y = y * background.height / framebuffer.height;
                        background.pixels[source_y * background.width + source_x]
                    })
                    .unwrap_or_else(|| {
                        fallback_background(x, y, framebuffer.width, framebuffer.height)
                    });
                framebuffer.buffer[y * framebuffer.width + x] = dim(color, 0.48);
            }
        }
    }

    fn draw_main(
        &self,
        framebuffer: &mut Framebuffer,
        textures: &TextureLibrary,
        mouse: Option<(usize, usize)>,
        mouse_down: bool,
    ) {
        draw_text_centered(framebuffer, "SKYBLOCK ISLAND", 92, 4, 0xf5e7bd, 0x271d14);
        draw_text_centered(framebuffer, "RAYTRACER", 140, 2, 0xe3d6b3, 0x271d14);

        for (button, label) in main_buttons()
            .into_iter()
            .zip(["JOIN WORLD", "HOW TO PLAY", "EXIT"])
        {
            draw_button(
                framebuffer,
                textures,
                button,
                label,
                mouse.is_some_and(|position| button.contains(position.0, position.1)) && mouse_down,
            );
        }

        draw_text_centered(
            framebuffer,
            "PRESS P IN WORLD TO SAVE A MENU BACKGROUND",
            620,
            1,
            0xd9e8f2,
            0x18202b,
        );
    }

    fn draw_how_to_play(
        &self,
        framebuffer: &mut Framebuffer,
        textures: &TextureLibrary,
        mouse: Option<(usize, usize)>,
        mouse_down: bool,
    ) {
        draw_text_centered(framebuffer, "HOW TO PLAY", 76, 4, 0xf5e7bd, 0x271d14);
        let lines = [
            "ARROWS LOOK AROUND",
            "WASD ORBIT OR MOVE",
            "F TOGGLE FREE FLIGHT",
            "SPACE UP  SHIFT DOWN",
            "Q E CHANGE TIME  PLUS MINUS ZOOM",
            "LEFT CLICK BUILD  RIGHT CLICK COPY",
            "NUMBER 1 TO 6 SELECTS AN ITEM",
            "P SAVES A SCREENSHOT  ESC OPENS MENU",
            "CTRL C EXITS THE GAME",
        ];
        for (index, line) in lines.into_iter().enumerate() {
            draw_text_centered(framebuffer, line, 190 + index * 36, 2, 0xe9eef4, 0x17202b);
        }
        let button = back_button();
        draw_button(
            framebuffer,
            textures,
            button,
            "BACK",
            mouse.is_some_and(|position| button.contains(position.0, position.1)) && mouse_down,
        );
    }
}

/// Guarda cada captura y actualiza la imagen que el menú mostrará al volver a
/// abrirse. PPM evita dependencias adicionales y coincide con el formato del
/// cargador de texturas del proyecto.
pub fn save_screenshot(framebuffer: &Framebuffer) -> Result<PathBuf, String> {
    let directory = screenshots_directory();
    fs::create_dir_all(&directory)
        .map_err(|error| format!("No se pudo crear {}: {error}", directory.display()))?;

    let mut number = 1usize;
    let path = loop {
        let candidate = directory.join(format!("screenshot_{number:03}.ppm"));
        if !candidate.exists() {
            break candidate;
        }
        number += 1;
    };

    let bytes = encode_framebuffer_ppm(framebuffer);
    fs::write(&path, &bytes)
        .map_err(|error| format!("No se pudo guardar {}: {error}", path.display()))?;
    let menu_background = directory.join("menu_background.ppm");
    fs::write(&menu_background, bytes).map_err(|error| {
        format!(
            "La captura se guardó, pero no se pudo actualizar {}: {error}",
            menu_background.display()
        )
    })?;
    Ok(path)
}

fn main_buttons() -> [ButtonRect; 3] {
    [
        ButtonRect {
            x: BUTTON_X,
            y: 300,
        },
        ButtonRect {
            x: BUTTON_X,
            y: 356,
        },
        ButtonRect {
            x: BUTTON_X,
            y: 412,
        },
    ]
}

fn back_button() -> ButtonRect {
    ButtonRect {
        x: BUTTON_X,
        y: 610,
    }
}

fn draw_button(
    framebuffer: &mut Framebuffer,
    textures: &TextureLibrary,
    button: ButtonRect,
    label: &str,
    pressed: bool,
) {
    let source_y_offset = if pressed { BUTTON_SOURCE_HEIGHT } else { 0 };
    for target_y in 0..BUTTON_HEIGHT {
        for target_x in 0..BUTTON_WIDTH {
            let source_x = target_x / BUTTON_SCALE;
            let source_y = source_y_offset + target_y / BUTTON_SCALE;
            let u = (source_x as f32 + 0.5) / BUTTON_SOURCE_WIDTH as f32;
            let v = 1.0 - (source_y as f32 + 0.5) / (BUTTON_SOURCE_HEIGHT * 2) as f32;
            let alpha = textures.sample_alpha(TextureId::MenuButton, u, v);
            if alpha <= 0.0 {
                continue;
            }
            let color = textures
                .sample(TextureId::MenuButton, u, v, Color::new(90, 90, 90))
                .to_hex();
            blend_pixel(
                framebuffer,
                button.x as i32 + target_x as i32,
                button.y as i32 + target_y as i32,
                color,
                alpha,
            );
        }
    }
    draw_text_centered_in_rect(framebuffer, label, button, 2, 0xffffff, 0x171717);
}

fn draw_text_centered(
    framebuffer: &mut Framebuffer,
    text: &str,
    y: usize,
    scale: usize,
    color: u32,
    shadow: u32,
) {
    let width = text_width(text, scale);
    let x = framebuffer.width.saturating_sub(width) / 2;
    draw_text(framebuffer, text, x, y, scale, color, shadow);
}

fn draw_text_centered_in_rect(
    framebuffer: &mut Framebuffer,
    text: &str,
    button: ButtonRect,
    scale: usize,
    color: u32,
    shadow: u32,
) {
    let width = text_width(text, scale);
    let x = button.x + BUTTON_WIDTH.saturating_sub(width) / 2;
    let text_height = 7 * scale;
    let y = button.y + BUTTON_HEIGHT.saturating_sub(text_height) / 2;
    draw_text(framebuffer, text, x, y, scale, color, shadow);
}

/// Tipografía bitmap integrada: los títulos a escala 4 evocan Press Start 2P;
/// el cuerpo a escala 2 mantiene una lectura más densa estilo Pixelify Sans.
fn draw_text(
    framebuffer: &mut Framebuffer,
    text: &str,
    x: usize,
    y: usize,
    scale: usize,
    color: u32,
    shadow: u32,
) {
    let mut cursor = x;
    for character in text.chars() {
        let glyph = glyph(character);
        for (row, bits) in glyph.into_iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) == 0 {
                    continue;
                }
                fill_rect(
                    framebuffer,
                    cursor as i32 + column as i32 * scale as i32 + scale as i32,
                    y as i32 + row as i32 * scale as i32 + scale as i32,
                    scale,
                    scale,
                    shadow,
                );
                fill_rect(
                    framebuffer,
                    cursor as i32 + column as i32 * scale as i32,
                    y as i32 + row as i32 * scale as i32,
                    scale,
                    scale,
                    color,
                );
            }
        }
        cursor += 6 * scale;
    }
}

fn text_width(text: &str, scale: usize) -> usize {
    text.chars()
        .count()
        .saturating_mul(6 * scale)
        .saturating_sub(scale)
}

fn glyph(character: char) -> [u8; 7] {
    match character.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10011, 0b10101, 0b10101, 0b10101, 0b11001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        '+' => [0, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        ':' => [0, 0b00100, 0b00100, 0, 0b00100, 0b00100, 0],
        _ => [0; 7],
    }
}

fn fill_rect(
    framebuffer: &mut Framebuffer,
    x: i32,
    y: i32,
    width: usize,
    height: usize,
    color: u32,
) {
    for offset_y in 0..height {
        for offset_x in 0..width {
            let pixel_x = x + offset_x as i32;
            let pixel_y = y + offset_y as i32;
            if pixel_x >= 0
                && pixel_y >= 0
                && (pixel_x as usize) < framebuffer.width
                && (pixel_y as usize) < framebuffer.height
            {
                framebuffer.buffer[pixel_y as usize * framebuffer.width + pixel_x as usize] = color;
            }
        }
    }
}

fn blend_pixel(framebuffer: &mut Framebuffer, x: i32, y: i32, foreground: u32, alpha: f32) {
    if x < 0 || y < 0 || (x as usize) >= framebuffer.width || (y as usize) >= framebuffer.height {
        return;
    }
    let index = y as usize * framebuffer.width + x as usize;
    let background = framebuffer.buffer[index];
    let channel = |shift: u32| {
        let from = ((foreground >> shift) & 0xFF) as f32;
        let to = ((background >> shift) & 0xFF) as f32;
        (from * alpha + to * (1.0 - alpha)) as u32
    };
    framebuffer.buffer[index] = (channel(16) << 16) | (channel(8) << 8) | channel(0);
}

fn fallback_background(x: usize, y: usize, width: usize, height: usize) -> u32 {
    let horizontal = x as f32 / width.max(1) as f32;
    let vertical = y as f32 / height.max(1) as f32;
    let red = (18.0 + 16.0 * (1.0 - vertical)) as u32;
    let green = (37.0 + 20.0 * horizontal) as u32;
    let blue = (57.0 + 34.0 * (1.0 - vertical)) as u32;
    (red << 16) | (green << 8) | blue
}

fn dim(color: u32, amount: f32) -> u32 {
    let channel = |shift: u32| (((color >> shift) & 0xFF) as f32 * amount) as u32;
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

fn screenshots_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("screenshots")
}

fn load_background() -> Result<Background, String> {
    let path = screenshots_directory().join("menu_background.ppm");
    let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut cursor = 0;
    if next_token(&bytes, &mut cursor) != Some(b"P6".as_slice()) {
        return Err("La captura de menú debe ser PPM P6".to_string());
    }
    let width = parse_dimension(next_token(&bytes, &mut cursor), "ancho")?;
    let height = parse_dimension(next_token(&bytes, &mut cursor), "alto")?;
    if parse_dimension(next_token(&bytes, &mut cursor), "rango")? != 255 {
        return Err("La captura de menú debe usar rango RGB 255".to_string());
    }
    if cursor >= bytes.len() || !bytes[cursor].is_ascii_whitespace() {
        return Err("La captura de menú tiene cabecera incompleta".to_string());
    }
    cursor += 1;
    let expected = width * height * 3;
    if bytes.len().saturating_sub(cursor) < expected {
        return Err("La captura de menú no contiene todos sus píxeles".to_string());
    }
    let pixels = bytes[cursor..cursor + expected]
        .chunks_exact(3)
        .map(|rgb| ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | rgb[2] as u32)
        .collect();
    Ok(Background {
        width,
        height,
        pixels,
    })
}

fn encode_framebuffer_ppm(framebuffer: &Framebuffer) -> Vec<u8> {
    let mut bytes = format!("P6\n{} {}\n255\n", framebuffer.width, framebuffer.height).into_bytes();
    bytes.reserve(framebuffer.buffer.len() * 3);
    for color in &framebuffer.buffer {
        bytes.push((color >> 16) as u8);
        bytes.push((color >> 8) as u8);
        bytes.push(*color as u8);
    }
    bytes
}

fn next_token<'a>(bytes: &'a [u8], cursor: &mut usize) -> Option<&'a [u8]> {
    while *cursor < bytes.len() && bytes[*cursor].is_ascii_whitespace() {
        *cursor += 1;
    }
    let start = *cursor;
    while *cursor < bytes.len() && !bytes[*cursor].is_ascii_whitespace() {
        *cursor += 1;
    }
    (start < *cursor).then_some(&bytes[start..*cursor])
}

fn parse_dimension(token: Option<&[u8]>, name: &str) -> Result<usize, String> {
    let token = token.ok_or_else(|| format!("Falta {name} en la captura de menú"))?;
    std::str::from_utf8(token)
        .map_err(|_| format!("{name} inválido en la captura de menú"))?
        .parse::<usize>()
        .map_err(|_| format!("{name} inválido en la captura de menú"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_encoder_keeps_framebuffer_rgb_pixels() {
        let mut framebuffer = Framebuffer::new(2, 1);
        framebuffer.buffer = vec![0x112233, 0xABCDEF];

        let bytes = encode_framebuffer_ppm(&framebuffer);

        assert_eq!(&bytes[..11], b"P6\n2 1\n255\n");
        assert_eq!(&bytes[11..], &[0x11, 0x22, 0x33, 0xAB, 0xCD, 0xEF]);
    }

    #[test]
    fn main_buttons_do_not_overlap() {
        let buttons = main_buttons();
        assert!(buttons[0].y + BUTTON_HEIGHT <= buttons[1].y);
        assert!(buttons[1].y + BUTTON_HEIGHT <= buttons[2].y);
    }
}
