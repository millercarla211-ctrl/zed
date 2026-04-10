use std::sync::Arc;

use gpui::{AnyElement, Bounds, IntoElement, Pixels, RenderImage, Window, canvas};

#[derive(Clone, Debug)]
pub struct LiquidGlassStyle {
    pub power_factor: f32,
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
}

impl LiquidGlassStyle {
    pub fn paint(
        &self,
        window: &mut Window,
        source_bounds: Bounds<Pixels>,
        glass_bounds: Bounds<Pixels>,
        source_image: Arc<RenderImage>,
    ) {
        let _ = window.paint_liquid_glass(
            source_bounds,
            source_image,
            gpui::LiquidGlassParams {
                glass_bounds,
                use_backdrop: true,
                power_factor: self.power_factor,
                a: self.a,
                b: self.b,
                c: self.c,
                d: self.d,
                f_power: self.f_power,
                noise: self.noise,
                glow_weight: self.glow_weight,
                glow_edge0: self.glow_edge0,
                glow_edge1: self.glow_edge1,
                glow_bias: self.glow_bias,
                chromatic_aberration: self.chromatic_aberration,
                aberration_samples: self.aberration_samples,
                blur_radius: self.blur_radius,
                blur_iterations: self.blur_iterations,
                blur_downscale: self.blur_downscale,
            },
            0,
        );
    }
}

pub fn liquid_glass_layer(
    source_image: Arc<RenderImage>,
    glass_bounds: Bounds<Pixels>,
    style: LiquidGlassStyle,
) -> AnyElement {
    canvas(
        move |bounds, _, _| bounds,
        move |bounds, _, window, _cx| {
            style.paint(window, bounds, glass_bounds, source_image.clone());
        },
    )
    .into_any_element()
}

pub fn paint_liquid_glass_layer(
    window: &mut Window,
    source_bounds: Bounds<Pixels>,
    glass_bounds: Bounds<Pixels>,
    source_image: Arc<RenderImage>,
    style: &LiquidGlassStyle,
) {
    style.paint(window, source_bounds, glass_bounds, source_image);
}
