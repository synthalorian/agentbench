use ratatui::style::Color;

pub struct SynthwaveTheme;

impl SynthwaveTheme {
    pub const DEEP_PURPLE: Color = Color::Rgb(0x24, 0x00, 0x37);
    pub const ELECTRIC_PURPLE: Color = Color::Rgb(0x8F, 0x00, 0xFF);
    pub const HOT_PINK: Color = Color::Rgb(0xFF, 0x7E, 0xDB);
    pub const MAGENTA: Color = Color::Rgb(0xFF, 0x00, 0xFF);
    pub const NEON_YELLOW: Color = Color::Rgb(0xF3, 0xE7, 0x0F);
    pub const CYAN: Color = Color::Rgb(0x00, 0xFF, 0xFF);
    pub const DARK_BG: Color = Color::Rgb(0x0D, 0x00, 0x1A);
    pub const TEXT: Color = Color::Rgb(0xE0, 0xE0, 0xE0);
    pub const MUTED: Color = Color::Rgb(0x80, 0x80, 0x80);
}
