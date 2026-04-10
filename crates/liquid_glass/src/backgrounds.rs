use std::sync::Arc;

use gpui::{App, RenderImage, SharedString};
use image::{Frame, Rgba, RgbaImage};
use smallvec::{SmallVec, smallvec};

#[derive(Clone, Debug)]
pub struct BackgroundAsset {
    pub name: SharedString,
    pub image: Arc<RenderImage>,
}

const BACKGROUND_ASSETS: [(&str, &str); 11] = [
    ("Cubes", "liquid_glass/textures/background-cubes.jpg"),
    ("Spring", "liquid_glass/textures/background-spring.png"),
    ("Summer", "liquid_glass/textures/background-summer.png"),
    ("Autumn", "liquid_glass/textures/background-autumn.png"),
    ("Winter", "liquid_glass/textures/background-winter.png"),
    (
        "Seasonal Landscape 1",
        "liquid_glass/textures/Seasonal Landscape 1.png",
    ),
    (
        "Seasonal Landscape 2",
        "liquid_glass/textures/Seasonal Landscape 2.png",
    ),
    ("Newspaper", "liquid_glass/textures/Newspaper.png"),
    (
        "Cartoon Cottage",
        "liquid_glass/textures/Cartoon Cottage.png",
    ),
    ("Anime girl", "liquid_glass/textures/anime.png"),
    (
        "Progressbar",
        "liquid_glass/textures/background-progress-bar.jpg",
    ),
];

pub fn load_backgrounds(cx: &App) -> Arc<[BackgroundAsset]> {
    BACKGROUND_ASSETS
        .iter()
        .enumerate()
        .map(|(ix, (name, path))| BackgroundAsset {
            name: (*name).into(),
            image: load_render_image(path, ix, cx),
        })
        .collect::<Vec<_>>()
        .into()
}

pub fn load_glass_surface() -> Arc<RenderImage> {
    let image = RgbaImage::from_pixel(2, 2, Rgba([0, 0, 0, 0]));
    Arc::new(RenderImage::new(to_bgra_frames(image)))
}

fn load_render_image(path: &str, index: usize, cx: &App) -> Arc<RenderImage> {
    match cx.asset_source().load(path) {
        Ok(Some(bytes)) => match image::load_from_memory(&bytes) {
            Ok(image) => Arc::new(RenderImage::new(to_bgra_frames(image.into_rgba8()))),
            Err(error) => {
                log::error!("failed to decode Liquid Glass background {path}: {error}");
                fallback_render_image(index)
            }
        },
        Ok(None) => {
            log::error!("missing Liquid Glass background asset at {path}");
            fallback_render_image(index)
        }
        Err(error) => {
            log::error!("failed to load Liquid Glass background asset {path}: {error}");
            fallback_render_image(index)
        }
    }
}

fn to_bgra_frames(mut image: RgbaImage) -> SmallVec<[Frame; 1]> {
    for pixel in image.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }

    smallvec![Frame::new(image)]
}

fn fallback_render_image(index: usize) -> Arc<RenderImage> {
    let palette = [
        [46, 57, 83, 255],
        [85, 126, 88, 255],
        [89, 113, 154, 255],
        [142, 95, 67, 255],
        [108, 124, 148, 255],
    ];
    let color = palette[index % palette.len()];
    let mut image = RgbaImage::from_pixel(2, 2, Rgba([color[2], color[1], color[0], color[3]]));
    for pixel in image.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Arc::new(RenderImage::new(smallvec![Frame::new(image)]))
}
