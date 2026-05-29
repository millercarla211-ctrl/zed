use std::path::{Path, PathBuf};

use gpui::{ObjectFit, img};
use project::Entry;
use ui::prelude::*;

pub(crate) const MAX_PROJECT_PANEL_MEDIA_CHILD_SCAN: usize = 512;
pub(crate) const MAX_PROJECT_PANEL_MEDIA_PREVIEW_ITEMS: usize = 12;
pub(crate) const MAX_PROJECT_PANEL_MEDIA_INLINE_CARDS: usize = 4;

const PROJECT_PANEL_MEDIA_CARD_WIDTH: f32 = 30.;
const PROJECT_PANEL_AUDIO_CARD_WIDTH: f32 = 72.;
const PROJECT_PANEL_MEDIA_CARD_HEIGHT: f32 = 20.;

const IMAGE_MEDIA_EXTENSIONS: &[&str] = &[
    "avif", "bmp", "gif", "ico", "jpeg", "jpg", "png", "svg", "tif", "tiff", "webp",
];
const VIDEO_MEDIA_EXTENSIONS: &[&str] = &["avi", "m4v", "mkv", "mov", "mp4", "mpeg", "mpg", "webm"];
const AUDIO_MEDIA_EXTENSIONS: &[&str] = &["aac", "flac", "m4a", "mp3", "ogg", "opus", "wav"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MediaPreviewKind {
    Image,
    Video,
    Audio,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MediaPreviewItem {
    pub(crate) kind: MediaPreviewKind,
    pub(crate) name: String,
    pub(crate) absolute_path: PathBuf,
    pub(crate) video_frame_path: Option<PathBuf>,
    pub(crate) audio_duration_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FolderMediaPreview {
    pub(crate) image_count: usize,
    pub(crate) video_count: usize,
    pub(crate) audio_count: usize,
    pub(crate) total_count: usize,
    pub(crate) scanned_cap_hit: bool,
    pub(crate) items: Vec<MediaPreviewItem>,
}

pub(crate) fn build_folder_media_preview<'a>(
    parent_abs_path: &Path,
    children: impl Iterator<Item = &'a Entry>,
) -> Option<FolderMediaPreview> {
    let mut image_count = 0;
    let mut video_count = 0;
    let mut audio_count = 0;
    let mut scanned_count = 0;
    let mut media_scan_was_capped = false;
    let mut items = Vec::new();
    let mut image_frame_candidates = Vec::new();

    for child in children.take(MAX_PROJECT_PANEL_MEDIA_CHILD_SCAN + 1) {
        if scanned_count >= MAX_PROJECT_PANEL_MEDIA_CHILD_SCAN {
            media_scan_was_capped = true;
            break;
        }

        scanned_count += 1;

        if !child.is_file() {
            continue;
        }

        let absolute_path = child_absolute_path(parent_abs_path, child);
        let Some(kind) = media_preview_kind_for_path(&absolute_path) else {
            continue;
        };

        match kind {
            MediaPreviewKind::Image => {
                image_count += 1;
                if let Some(stem) = media_stem_key(&absolute_path) {
                    image_frame_candidates.push((stem, absolute_path.clone()));
                }
            }
            MediaPreviewKind::Video => video_count += 1,
            MediaPreviewKind::Audio => audio_count += 1,
        }

        if items.len() < MAX_PROJECT_PANEL_MEDIA_PREVIEW_ITEMS {
            let name = child
                .path
                .file_name()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| absolute_path.display().to_string());
            items.push(MediaPreviewItem {
                kind,
                name,
                absolute_path,
                video_frame_path: None,
                audio_duration_label: (kind == MediaPreviewKind::Audio)
                    .then(|| "Duration unavailable".to_string()),
            });
        }
    }

    for item in &mut items {
        if item.kind == MediaPreviewKind::Video {
            item.video_frame_path =
                video_preview_frame_path(&item.absolute_path, &image_frame_candidates);
        }
    }

    let total_count = image_count + video_count + audio_count;
    (total_count > 0).then_some(FolderMediaPreview {
        image_count,
        video_count,
        audio_count,
        total_count,
        scanned_cap_hit: media_scan_was_capped,
        items,
    })
}

pub(crate) fn render_folder_media_preview(
    preview: &FolderMediaPreview,
    cx: &mut App,
) -> AnyElement {
    let summary = media_preview_summary(preview);
    let tooltip_summary = summary.clone();
    let cards = preview
        .items
        .iter()
        .take(MAX_PROJECT_PANEL_MEDIA_INLINE_CARDS)
        .map(|item| render_media_preview_card(item, cx))
        .collect::<Vec<_>>();

    h_flex()
        .h_6()
        .max_w(px(184.))
        .gap_0p5()
        .overflow_hidden()
        .block_mouse_except_scroll()
        .tooltip(move |_window, cx| Tooltip::with_meta(tooltip_summary.clone(), None, "Media", cx))
        .children(cards)
        .child(
            Label::new(summary)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .single_line()
                .truncate(),
        )
        .into_any_element()
}

pub(crate) fn render_media_preview_card(item: &MediaPreviewItem, cx: &mut App) -> AnyElement {
    let colors = cx.theme().colors();
    let tooltip_title = item.name.clone();
    let tooltip_meta = media_preview_card_tooltip_meta(item);
    let card = match item.kind {
        MediaPreviewKind::Image => div()
            .w(px(PROJECT_PANEL_MEDIA_CARD_WIDTH))
            .h(px(PROJECT_PANEL_MEDIA_CARD_HEIGHT))
            .rounded_sm()
            .overflow_hidden()
            .border_1()
            .border_color(colors.border_variant)
            .child(
                img(item.absolute_path.clone())
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            ),
        MediaPreviewKind::Video => {
            if let Some(frame_path) = item.video_frame_path.as_ref() {
                div()
                    .relative()
                    .w(px(PROJECT_PANEL_MEDIA_CARD_WIDTH))
                    .h(px(PROJECT_PANEL_MEDIA_CARD_HEIGHT))
                    .rounded_sm()
                    .overflow_hidden()
                    .border_1()
                    .border_color(colors.border_variant)
                    .child(
                        img(frame_path.clone())
                            .size_full()
                            .object_fit(ObjectFit::Cover),
                    )
                    .child(
                        div()
                            .absolute()
                            .right_0()
                            .bottom_0()
                            .rounded_full()
                            .bg(colors.editor_background.opacity(0.72))
                            .child(
                                Icon::new(IconName::PlayOutlined)
                                    .size(IconSize::XSmall)
                                    .color(Color::Accent),
                            ),
                    )
            } else {
                div()
                    .w(px(PROJECT_PANEL_MEDIA_CARD_WIDTH))
                    .h(px(PROJECT_PANEL_MEDIA_CARD_HEIGHT))
                    .rounded_sm()
                    .border_1()
                    .border_color(colors.border_variant)
                    .bg(colors.elevated_surface_background)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(IconName::PlayOutlined)
                            .size(IconSize::Small)
                            .color(Color::Muted),
                    )
            }
        }
        MediaPreviewKind::Audio => div()
            .w(px(PROJECT_PANEL_AUDIO_CARD_WIDTH))
            .h(px(PROJECT_PANEL_MEDIA_CARD_HEIGHT))
            .rounded_sm()
            .border_1()
            .border_color(colors.border_variant)
            .bg(colors.elevated_surface_background)
            .overflow_hidden()
            .flex()
            .items_center()
            .gap_0p5()
            .px_1()
            .child(
                Icon::new(IconName::AudioOn)
                    .size(IconSize::XSmall)
                    .color(Color::Muted),
            )
            .child(
                Label::new(
                    item.audio_duration_label
                        .as_deref()
                        .unwrap_or("Duration unavailable"),
                )
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .single_line()
                .truncate(),
            ),
    };

    card.tooltip(move |_window, cx| {
        Tooltip::with_meta(tooltip_title.clone(), None, tooltip_meta.clone(), cx)
    })
    .into_any_element()
}

fn media_preview_card_tooltip_meta(item: &MediaPreviewItem) -> String {
    match item.kind {
        MediaPreviewKind::Image => "Image preview".to_string(),
        MediaPreviewKind::Video => {
            if item.video_frame_path.is_some() {
                "Video frame preview".to_string()
            } else {
                "Video preview unavailable".to_string()
            }
        }
        MediaPreviewKind::Audio => item
            .audio_duration_label
            .clone()
            .unwrap_or_else(|| "Duration unavailable".to_string()),
    }
}

fn child_absolute_path(parent_abs_path: &Path, child: &Entry) -> PathBuf {
    child
        .canonical_path
        .as_ref()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| match child.path.file_name() {
            Some(file_name) => parent_abs_path.join(file_name),
            None => parent_abs_path.join(child.path.as_std_path()),
        })
}

fn media_preview_summary(preview: &FolderMediaPreview) -> String {
    let mut parts = Vec::new();
    push_media_count(&mut parts, preview.image_count, "image", "images");
    push_media_count(&mut parts, preview.video_count, "video", "videos");
    push_media_count(&mut parts, preview.audio_count, "audio", "audio");

    if preview.scanned_cap_hit {
        parts.push("more".to_string());
    }

    parts.join(" / ")
}

fn push_media_count(parts: &mut Vec<String>, count: usize, singular: &str, plural: &str) {
    if count == 1 {
        parts.push(format!("1 {singular}"));
    } else if count > 1 {
        parts.push(format!("{count} {plural}"));
    }
}

fn media_preview_kind_for_path(path: &Path) -> Option<MediaPreviewKind> {
    let extension = path.extension()?.to_str()?;
    if matches_extension(extension, IMAGE_MEDIA_EXTENSIONS) {
        Some(MediaPreviewKind::Image)
    } else if matches_extension(extension, VIDEO_MEDIA_EXTENSIONS) {
        Some(MediaPreviewKind::Video)
    } else if matches_extension(extension, AUDIO_MEDIA_EXTENSIONS) {
        Some(MediaPreviewKind::Audio)
    } else {
        None
    }
}

fn matches_extension(extension: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
}

fn video_preview_frame_path(
    video_path: &Path,
    image_frame_candidates: &[(String, PathBuf)],
) -> Option<PathBuf> {
    let video_stem = media_stem_key(video_path)?;
    image_frame_candidates
        .iter()
        .find_map(|(stem, path)| (stem == &video_stem).then(|| path.clone()))
}

fn media_stem_key(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_ascii_lowercase())
}
