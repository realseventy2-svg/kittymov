use ratatui::style::Color;

pub struct Theme;

impl Theme {
    pub const BG: Color = Color::Rgb(11, 14, 20);           // Deep space obsidian
    pub const SURFACE: Color = Color::Rgb(15, 20, 28);      // Panel background
    pub const SURFACE_ALT: Color = Color::Rgb(22, 27, 34);  // Elevated container
    
    pub const BORDER: Color = Color::Rgb(0, 229, 255);       // Cyber Cyan glow
    pub const BORDER_MUTED: Color = Color::Rgb(33, 45, 60); // Muted border
    
    pub const ACCENT_CYAN: Color = Color::Rgb(56, 189, 248);// Sky / Cyan
    pub const ACCENT_PINK: Color = Color::Rgb(244, 63, 94); // Neon Magenta / Rose
    pub const ACCENT_GOLD: Color = Color::Rgb(251, 191, 36);// Stars / Rating
    pub const ACCENT_GREEN: Color = Color::Rgb(16, 185, 129);// Status / Episodes
    
    pub const TEXT_TITLE: Color = Color::Rgb(240, 246, 252);// Crisp white
    pub const TEXT_BODY: Color = Color::Rgb(201, 209, 217); // Light slate
    pub const TEXT_MUTED: Color = Color::Rgb(113, 123, 137);// Dim gray
    
    pub const SELECTION_BG: Color = Color::Rgb(31, 41, 61); // Selection cursor
}
