pub const GLASS_VARIANTS: &[&str] = &[
    "Regular",
    "Clear",
    "Dock",
    "App Icons",
    "Widgets",
    "Text",
    "AVPlayer",
    "FaceTime",
    "Control Center",
    "Notification Center",
    "Monogram",
    "Bubbles",
    "Identity",
    "Focus Border",
    "Focus Platter",
    "Keyboard",
    "Sidebar",
    "Abutted Sidebar",
    "Inspector",
    "Control",
    "Loupe",
    "Slider",
    "Camera",
    "Cartouche Popover",
];

pub const GLASS_VARIANT_SIZES: &[(f32, f32)] = &[
    (3.5, 3.5),
    (3.5, 3.5),
    (8.0, 0.8),
    (1.2, 1.2),
    (4.0, 4.0),
    (6.0, 1.0),
    (6.0, 4.0),
    (5.0, 5.0),
    (3.0, 6.0),
    (3.0, 7.0),
    (1.5, 1.5),
    (2.0, 2.0),
    (2.5, 2.5),
    (4.0, 4.0),
    (3.0, 1.5),
    (8.0, 3.0),
    (2.5, 8.0),
    (2.5, 8.0),
    (3.0, 5.0),
    (2.0, 2.0),
    (2.0, 2.0),
    (5.0, 0.5),
    (4.0, 3.0),
    (3.0, 2.0),
];

#[derive(Debug, Clone)]
pub struct UiState {
    pub power_factor: f32,
    pub width: f32,
    pub height: f32,
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub f_power: f32,
    pub noise: f32,
    pub glow_weight: f32,
    pub glow_edge0: f32,
    pub glow_edge1: f32,
    pub glow_bias: f32,
    pub chromatic_aberration: f32,
    pub aberration_samples: u32,
    pub blur_radius: f32,
    pub blur_iterations: u32,
    pub blur_downscale: f32,
    pub mouse_control: bool,
    pub position: [f32; 2],
    pub pixel_scale: f32,
    pub current_bg: usize,
    pub glass_variant: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            power_factor: 3.0,
            width: 3.5,
            height: 3.5,
            a: 0.7,
            b: 2.3,
            c: 5.2,
            d: 6.9,
            f_power: 0.992,
            noise: 0.0,
            glow_weight: -0.009,
            glow_edge0: 1.0,
            glow_edge1: -1.0,
            glow_bias: 0.132,
            chromatic_aberration: 0.0,
            aberration_samples: 1,
            blur_radius: 0.0,
            blur_iterations: 1,
            blur_downscale: 0.1,
            mouse_control: true,
            position: [7.0, 80.0],
            pixel_scale: 100.0,
            current_bg: 3,
            glass_variant: 0,
        }
    }
}

impl UiState {
    pub fn glass_width_px(&self) -> f32 {
        self.width * self.pixel_scale
    }

    pub fn glass_height_px(&self) -> f32 {
        self.height * self.pixel_scale
    }

    pub fn apply_variant_size(&mut self) {
        let (width, height) = GLASS_VARIANT_SIZES[self.glass_variant];
        self.width = width;
        self.height = height;
    }
}
