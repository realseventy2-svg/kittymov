use ratatui::style::Color;

pub struct Theme;

impl Theme {
    pub const BG: Color = Color::Rgb(16, 17, 19); // #101113 Near-black backdrop
    pub const SURFACE: Color = Color::Rgb(26, 28, 32); // #1A1C20 Panel background
    pub const SURFACE_ALT: Color = Color::Rgb(34, 38, 44); // Elevated container

    pub const BORDER: Color = Color::Rgb(0, 229, 255); // Cyber Cyan glow
    pub const BORDER_MUTED: Color = Color::Rgb(45, 52, 64); // Muted border

    pub const ACCENT_CYAN: Color = Color::Rgb(56, 189, 248); // Sky / Cyan
    pub const ACCENT_PINK: Color = Color::Rgb(244, 63, 94); // Neon Magenta / Rose
    pub const ACCENT_GOLD: Color = Color::Rgb(232, 184, 117); // #E8B875 Warm amber accent
    pub const ACCENT_GREEN: Color = Color::Rgb(16, 185, 129); // Status / Success

    pub const TEXT_TITLE: Color = Color::Rgb(233, 230, 224); // #E9E6E0 Primary text
    pub const TEXT_BODY: Color = Color::Rgb(201, 209, 217); // Light slate
    pub const TEXT_MUTED: Color = Color::Rgb(150, 154, 163); // #969AA3 Secondary / dim text

    pub const SELECTION_BG: Color = Color::Rgb(37, 40, 45); // #25282D Elevated selection surface
}
