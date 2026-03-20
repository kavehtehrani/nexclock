use std::time::Duration;

// Tick / refresh rates
pub const DEFAULT_TICK_RATE: Duration = Duration::from_millis(250);
pub const MIN_TICK_RATE_MS: u64 = 50;

// Clock display
pub const DEFAULT_TIME_FORMAT: &str = "24h";

// Status bar
pub const STATUS_BAR_HEIGHT: u16 = 3;

// Minimum terminal size before showing a "too small" message
pub const MIN_TERMINAL_WIDTH: u16 = 40;
pub const MIN_TERMINAL_HEIGHT: u16 = 10;

// Network defaults
pub const DEFAULT_IP_REFRESH_MINUTES: u64 = 10;

// Weather defaults
pub const DEFAULT_LATITUDE: f64 = 35.6892;
pub const DEFAULT_LONGITUDE: f64 = 51.3890;
pub const DEFAULT_TEMP_UNIT: &str = "celsius";
pub const DEFAULT_WEATHER_REFRESH_MINUTES: u64 = 30;

// System stats defaults
pub const DEFAULT_STATS_REFRESH_SECONDS: u64 = 5;

// Font
pub const DEFAULT_FONT_STYLE: &str = "Block";

// Date format
pub const DEFAULT_DATE_FORMAT: &str = "%A, %B %d, %Y";
pub const DEFAULT_SECONDARY_DATE_FORMAT: &str = "%a, %b %d";
pub const DATE_FORMAT_PRESETS: &[&str] = &[
    "%A, %B %d, %Y", // Wednesday, March 19, 2026
    "%Y-%m-%d",       // 2026-03-19
    "%d/%m/%Y",       // 19/03/2026
    "%B %d, %Y",      // March 19, 2026
    "%d %b %Y",       // 19 Mar 2026
];

// Color gradient presets: (label, &[color_strings])
// Empty slice means "reset to theme default"
pub const COLOR_PRESETS: &[(&str, &[&str])] = &[
    // Reset
    ("Theme default", &[]),
    // Solid colors
    ("Cyan", &["cyan"]),
    ("Green", &["green"]),
    ("Yellow", &["yellow"]),
    ("Magenta", &["magenta"]),
    ("Red", &["red"]),
    ("White", &["white"]),
    // Warm gradients
    ("Sunset", &["#FF6B35", "#FFD700"]),
    ("Fire", &["#FF4500", "#FFD700"]),
    ("Ember", &["#FF4500", "#FF8C00", "#FFD700"]),
    ("Coral", &["#FF6F61", "#FFB347"]),
    ("Peach", &["#FFAB91", "#FFE0B2"]),
    ("Amber", &["#FF8F00", "#FFD54F"]),
    // Cool gradients
    ("Ocean", &["#00CED1", "#1E90FF"]),
    ("Ice", &["#E0FFFF", "#4169E1"]),
    ("Teal", &["#00BFA5", "#00838F"]),
    ("Steel", &["#B0BEC5", "#546E7A"]),
    ("Arctic", &["#80DEEA", "#0097A7"]),
    // Nature
    ("Forest", &["#00FF7F", "#228B22"]),
    ("Mint", &["#B2FFD6", "#00C853"]),
    ("Aurora", &["#00FF7F", "#00CED1", "#9370DB"]),
    ("Spring", &["#76FF03", "#FFEB3B"]),
    // Neon / vibrant
    ("Neon", &["#FF00FF", "#00FFFF"]),
    ("Synthwave", &["#FF00FF", "#7B2FBE", "#00FFFF"]),
    ("Vaporwave", &["#FF71CE", "#01CDFE"]),
    ("Cyberpunk", &["#F706CF", "#FFF700"]),
    ("Matrix", &["#003B00", "#00FF41"]),
    // Purple
    ("Lavender", &["#E066FF", "#836FFF"]),
    ("Grape", &["#9C27B0", "#E040FB"]),
    ("Twilight", &["#4A148C", "#CE93D8"]),
    // Pastel
    ("Cotton Candy", &["#FFB6C1", "#B5EAD7"]),
    ("Bubblegum", &["#FF9AA2", "#FFB7B2", "#FFDAC1"]),
    ("Dreamy", &["#C5CAE9", "#F8BBD0"]),
    // Monochrome
    ("Silver", &["#E0E0E0", "#757575"]),
    ("Gold", &["#FFD700", "#B8860B"]),
    // Multi-stop
    ("Rainbow", &["#FF0000", "#FF8C00", "#FFFF00", "#00FF00", "#0000FF", "#8B00FF"]),
    ("Heatmap", &["#0000FF", "#00FFFF", "#00FF00", "#FFFF00", "#FF0000"]),
];

// Grid defaults
pub const DEFAULT_GRID_ROWS: u16 = 3;
pub const DEFAULT_GRID_COLUMNS: u16 = 2;

// UI popup / menu dimensions
pub const CONTEXT_MENU_WIDTH: u16 = 35;
pub const VISIBILITY_MENU_WIDTH: u16 = 40;
pub const ADD_MENU_WIDTH: u16 = 30;
pub const HELP_POPUP_WIDTH: u16 = 42;
pub const COLOR_BAR_WIDTH: usize = 12;
pub const SHORTCUT_KEY_WIDTH: usize = 14;
pub const INDICATOR_ARROW: &str = "\u{25b8}";
pub const GRADIENT_BLOCK: &str = "\u{2588}";

// World Clock / timezone search
pub const TZ_SEARCH_WIDTH: u16 = 45;
pub const TZ_SEARCH_MAX_RESULTS: usize = 10;
pub const TZ_REMOVE_MENU_WIDTH: u16 = 40;

// Theme color defaults
pub const DEFAULT_THEME_PRIMARY: &str = "cyan";
pub const DEFAULT_THEME_SECONDARY: &str = "yellow";
pub const DEFAULT_THEME_TERTIARY: &str = "green";
pub const DEFAULT_THEME_INFO: &str = "blue";
pub const DEFAULT_THEME_MUTED: &str = "dark_gray";
pub const DEFAULT_THEME_TEXT: &str = "white";
pub const DEFAULT_THEME_ERROR: &str = "red";
pub const DEFAULT_THEME_FOCUS: &str = "cyan";
