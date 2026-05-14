use editor::{Editor, EditorEvent};
use gpui::{
    App, AppContext as _, AsyncWindowContext, ClipboardItem, Context, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, ObjectFit, Pixels, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement, Subscription, WeakEntity, Window, actions, div, img,
    point, px,
};
use std::{
    fs as std_fs,
    path::{Path, PathBuf},
};
use ui::{TintColor, Tooltip, prelude::*};
use url::Url;
use workspace::{
    DraggedMediaAsset, DraggedMediaKind, Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

#[cfg(target_os = "windows")]
use web_preview::web_preview_view::WebPreviewView;

actions!(
    media_panel,
    [
        /// Toggles the media panel.
        Toggle,
        /// Toggles focus on the media panel.
        ToggleFocus,
    ]
);

const MEDIA_PANEL_KEY: &str = "MediaPanel";
const MAX_MEDIA_RESULTS: usize = 220;

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace.register_action(|workspace, _: &ToggleFocus, window, cx| {
            workspace.toggle_panel_focus::<MediaPanel>(window, cx);
        });
        workspace.register_action(|workspace, _: &Toggle, window, cx| {
            if !workspace.toggle_panel_focus::<MediaPanel>(window, cx) {
                workspace.close_panel::<MediaPanel>(window, cx);
            }
        });
    })
    .detach();
}

#[derive(Clone)]
struct MediaAsset {
    payload: DraggedMediaAsset,
    search_text: SharedString,
}

#[derive(Clone, Copy)]
struct RemoteMediaAsset {
    id: &'static str,
    label: &'static str,
    provider: &'static str,
    url: &'static str,
    kind: DraggedMediaKind,
    license: &'static str,
    tags: &'static str,
}

enum MediaPreviewSource {
    Local(PathBuf),
    Remote(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MediaKindFilter {
    All,
    Images,
    Videos,
    Audio,
}

impl MediaKindFilter {
    fn matches(self, kind: DraggedMediaKind) -> bool {
        match self {
            Self::All => true,
            Self::Images => kind == DraggedMediaKind::Image,
            Self::Videos => kind == DraggedMediaKind::Video,
            Self::Audio => kind == DraggedMediaKind::Audio,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Images => "Images",
            Self::Videos => "Videos",
            Self::Audio => "Audio",
        }
    }

    fn fallback_kind(self) -> Option<DraggedMediaKind> {
        match self {
            Self::All => None,
            Self::Images => Some(DraggedMediaKind::Image),
            Self::Videos => Some(DraggedMediaKind::Video),
            Self::Audio => Some(DraggedMediaKind::Audio),
        }
    }
}

pub struct MediaPanel {
    workspace: WeakEntity<Workspace>,
    filter_editor: Entity<Editor>,
    media_roots: Vec<PathBuf>,
    assets: Vec<MediaAsset>,
    kind_filter: MediaKindFilter,
    kind_scroll_handle: ScrollHandle,
    loading: bool,
    index_loaded: bool,
    status: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl MediaPanel {
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> anyhow::Result<Entity<Self>> {
        workspace.update_in(&mut cx, |workspace, window, cx| {
            Self::new(workspace, window, cx)
        })
    }

    fn new(
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Entity<Self> {
        let workspace_handle = cx.entity().downgrade();
        let media_roots = media_roots_for_workspace(workspace, cx);

        cx.new(|cx| {
            let filter_editor = cx.new(|cx| {
                let mut editor = Editor::single_line(window, cx);
                editor.set_placeholder_text("Search media...", window, cx);
                editor
            });

            let filter_subscription = cx.subscribe_in(
                &filter_editor,
                window,
                |panel: &mut Self, _, event, _, cx| {
                    if matches!(event, EditorEvent::BufferEdited) {
                        panel.status = None;
                        cx.notify();
                    }
                },
            );

            Self {
                workspace: workspace_handle,
                filter_editor,
                media_roots,
                assets: Vec::new(),
                kind_filter: MediaKindFilter::Images,
                kind_scroll_handle: ScrollHandle::new(),
                loading: false,
                index_loaded: false,
                status: None,
                _subscriptions: vec![filter_subscription],
            }
        })
    }

    fn ensure_media_index_loaded(&mut self, cx: &mut Context<Self>) {
        if self.index_loaded || self.loading {
            return;
        }

        self.refresh_media_index_from_current_roots(cx);
    }

    fn refresh_media_index_from_current_roots(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        let media_roots = self.media_roots.clone();
        let executor = cx.background_executor().clone();
        cx.spawn(async move |panel, cx| {
            let assets = executor
                .spawn(async move { gather_media_assets(media_roots) })
                .await;
            panel
                .update(cx, |panel, cx| {
                    panel.assets = assets;
                    panel.loading = false;
                    panel.index_loaded = true;
                    panel.status =
                        Some(format!("Indexed {} media files", panel.assets.len()).into());
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn query(&self, cx: &App) -> String {
        self.filter_editor.read(cx).text(cx).trim().to_lowercase()
    }

    fn raw_query(&self, cx: &App) -> String {
        self.filter_editor.read(cx).text(cx).trim().to_string()
    }

    fn matching_assets(&self, cx: &App, limit: usize) -> (Vec<MediaAsset>, usize) {
        let query = self.query(cx);
        let kind_filter = self.kind_filter;
        let mut visible_assets = Vec::new();
        let mut match_count = 0;

        for asset in &self.assets {
            if !kind_filter.matches(asset.payload.kind) {
                continue;
            }

            if !media_search_matches(asset.search_text.as_ref(), query.as_str()) {
                continue;
            }

            match_count += 1;
            if visible_assets.len() < limit {
                visible_assets.push(asset.clone());
            }
        }

        (visible_assets, match_count)
    }

    fn matching_remote_assets(&self, cx: &App, limit: usize) -> (Vec<RemoteMediaAsset>, usize) {
        let query = self.query(cx);
        let kind_filter = self.kind_filter;
        let mut visible_assets = Vec::new();
        let mut match_count = 0;

        for asset in remote_media_assets() {
            if !kind_filter.matches(asset.kind) {
                continue;
            }

            let searchable = format!(
                "{} {} {} {} {}",
                asset.label,
                asset.provider,
                asset.license,
                asset.tags,
                media_kind_label(asset.kind)
            )
            .to_lowercase();
            if !media_search_matches(searchable.as_str(), query.as_str()) {
                continue;
            }

            match_count += 1;
            if visible_assets.len() < limit {
                visible_assets.push(*asset);
            }
        }

        (visible_assets, match_count)
    }

    fn filtered_count(&self, filter: MediaKindFilter) -> usize {
        self.assets
            .iter()
            .filter(|asset| filter.matches(asset.payload.kind))
            .count()
            + remote_media_assets()
                .iter()
                .filter(|asset| filter.matches(asset.kind))
                .count()
    }

    fn render_kind_filter_button(
        &self,
        filter: MediaKindFilter,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.kind_filter == filter;
        let label = format!("{} {}", filter.label(), self.filtered_count(filter));
        div().flex_none().child(
            Button::new(format!("media-kind-filter-{}", filter.label()), label)
                .style(ButtonStyle::Subtle)
                .size(ButtonSize::Compact)
                .toggle_state(selected)
                .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                .on_click(cx.listener(move |panel, _, _, cx| {
                    panel.kind_filter = filter;
                    panel.status = None;
                    cx.notify();
                })),
        )
    }

    fn render_kind_filters(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .h(px(42.))
            .gap_1()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().colors().border.opacity(0.6))
            .px_1()
            .child(
                IconButton::new("media-panel-kind-prev", IconName::ChevronLeft)
                    .shape(ui::IconButtonShape::Square)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Previous media groups"))
                    .on_click(cx.listener(|panel, _, _, cx| {
                        panel.scroll_kind_tabs(-1.0, cx);
                    })),
            )
            .child(
                h_flex()
                    .id("media-panel-kind-filter-scroll")
                    .flex_1()
                    .h_full()
                    .overflow_x_scroll()
                    .overflow_y_hidden()
                    .track_scroll(&self.kind_scroll_handle)
                    .child(
                        h_flex()
                            .flex_none()
                            .gap_1()
                            .items_center()
                            .px_1()
                            .py_1()
                            .child(self.render_kind_filter_button(MediaKindFilter::All, cx))
                            .child(self.render_kind_filter_button(MediaKindFilter::Images, cx))
                            .child(self.render_kind_filter_button(MediaKindFilter::Videos, cx))
                            .child(self.render_kind_filter_button(MediaKindFilter::Audio, cx)),
                    ),
            )
            .child(
                IconButton::new("media-panel-kind-next", IconName::ChevronRight)
                    .shape(ui::IconButtonShape::Square)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Next media groups"))
                    .on_click(cx.listener(|panel, _, _, cx| {
                        panel.scroll_kind_tabs(1.0, cx);
                    })),
            )
    }

    fn scroll_kind_tabs(&mut self, direction: f32, cx: &mut Context<Self>) {
        scroll_tab_handle(&self.kind_scroll_handle, direction);
        cx.notify();
    }

    fn insert_media(
        &mut self,
        asset: DraggedMediaAsset,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace) = self.workspace.upgrade() else {
            self.status = Some("No active workspace".into());
            cx.notify();
            return;
        };
        let Some(editor) = workspace.read(cx).active_item_as::<Editor>(cx) else {
            self.status = Some("Open an editor to insert the media".into());
            cx.notify();
            return;
        };

        let result = editor.update(cx, |editor, cx| {
            editor.focus_handle(cx).focus(window, cx);
            editor.insert_media_asset(&asset, window, cx)
        });

        self.status = match result {
            Ok(message) => Some(message),
            Err(error) => Some(format!("{error:#}").into()),
        };
        cx.notify();
    }

    fn insert_media_url(
        &mut self,
        url: String,
        kind: DraggedMediaKind,
        label: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace) = self.workspace.upgrade() else {
            self.status = Some("No active workspace".into());
            cx.notify();
            return;
        };
        let Some(editor) = workspace.read(cx).active_item_as::<Editor>(cx) else {
            self.status = Some("Open an editor to insert the media URL".into());
            cx.notify();
            return;
        };

        let message = editor.update(cx, |editor, cx| {
            editor.focus_handle(cx).focus(window, cx);
            editor.insert_media_url(&url, kind, &label, window, cx)
        });

        self.status = Some(message);
        cx.notify();
    }

    fn preview_media_asset(
        &mut self,
        asset: DraggedMediaAsset,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let label = asset.label.to_string();
        let Some(preview_url) = local_media_preview_url(
            &label,
            asset.kind,
            MediaPreviewSource::Local(asset.path.clone()),
        ) else {
            self.status = Some("Could not create media preview".into());
            cx.notify();
            return;
        };

        self.open_media_preview(preview_url, label, window, cx);
    }

    fn preview_media_url(
        &mut self,
        url: String,
        kind: DraggedMediaKind,
        label: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(preview_url) =
            local_media_preview_url(&label, kind, MediaPreviewSource::Remote(url))
        else {
            self.status = Some("Could not create media preview".into());
            cx.notify();
            return;
        };

        self.open_media_preview(preview_url, label, window, cx);
    }

    fn browse_remote_media(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.raw_query(cx);
        let Some(preview_url) = remote_media_search_url(&query, self.kind_filter) else {
            self.status = Some("Could not create remote media browser".into());
            cx.notify();
            return;
        };

        self.open_media_preview(preview_url, "remote media".to_string(), window, cx);
    }

    fn open_media_preview(
        &mut self,
        preview_url: String,
        label: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        #[cfg(target_os = "windows")]
        {
            let Some(workspace) = self.workspace.upgrade() else {
                self.status = Some("No active workspace".into());
                cx.notify();
                return;
            };

            workspace.update(cx, |workspace, cx| {
                WebPreviewView::open_url_in_active_pane(workspace, &preview_url, window, cx);
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            cx.open_url(&preview_url);
        }

        self.status = Some(format!("Previewing {label}").into());
        cx.notify();
    }

    fn copy_media_source(&mut self, source: String, label: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(source));
        self.status = Some(format!("Copied {label}").into());
        cx.notify();
    }

    fn render_url_insert(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let query = self.raw_query(cx);
        let candidate = media_url_candidate(&query, self.kind_filter.fallback_kind())?;
        let kind = candidate.kind;
        let url = candidate.url;
        let label = candidate.label;

        Some(
            h_flex()
                .gap_2()
                .items_center()
                .p_2()
                .rounded_sm()
                .border_1()
                .border_color(cx.theme().colors().border_variant)
                .bg(cx.theme().colors().element_background)
                .child(Icon::new(media_kind_icon(kind)).size(IconSize::Small))
                .child(
                    v_flex()
                        .flex_1()
                        .gap_0p5()
                        .child(Label::new(label.clone()).size(LabelSize::Small).truncate())
                        .child(
                            Label::new(url.clone())
                                .size(LabelSize::XSmall)
                                .color(Color::Muted)
                                .truncate(),
                        ),
                )
                .child(
                    h_flex()
                        .gap_1()
                        .child(
                            Button::new("media-panel-preview-url", "Preview")
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .on_click(cx.listener({
                                    let url = url.clone();
                                    let label = label.clone();
                                    move |panel, _, window, cx| {
                                        panel.preview_media_url(
                                            url.clone(),
                                            kind,
                                            label.clone(),
                                            window,
                                            cx,
                                        );
                                    }
                                })),
                        )
                        .child(
                            Button::new("media-panel-copy-url", "Copy")
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .on_click(cx.listener({
                                    let url = url.clone();
                                    let label = label.clone();
                                    move |panel, _, _, cx| {
                                        panel.copy_media_source(url.clone(), label.clone(), cx);
                                    }
                                })),
                        )
                        .child(
                            Button::new("media-panel-insert-url", "Insert URL")
                                .style(ButtonStyle::Filled)
                                .size(ButtonSize::Compact)
                                .on_click(cx.listener(move |panel, _, window, cx| {
                                    panel.insert_media_url(
                                        url.clone(),
                                        kind,
                                        label.clone(),
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                ),
        )
    }

    fn render_asset_row(&self, asset: MediaAsset, cx: &mut Context<Self>) -> impl IntoElement {
        let payload = asset.payload;
        let kind = payload.kind;
        let label = payload.label.clone();
        let path = payload.path.clone();
        let relative_display = payload.relative_display.clone();
        let preview_payload = payload.clone();
        let copy_path = path.to_string_lossy().to_string();
        let copy_label = label.to_string();

        h_flex()
            .id(format!("media-panel-row-{}", relative_display.as_ref()))
            .gap_2()
            .items_center()
            .p_2()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().element_background)
            .cursor_pointer()
            .hover(|style| style.bg(cx.theme().colors().element_hover))
            .tooltip(Tooltip::text(relative_display.to_string()))
            .on_click(cx.listener({
                let payload = payload.clone();
                move |panel, _, window, cx| {
                    panel.insert_media(payload.clone(), window, cx);
                }
            }))
            .on_drag(payload, |media, position, _, cx| {
                cx.new(|_| MediaDragPreview {
                    media: media.clone(),
                    position,
                })
            })
            .child(media_thumbnail(kind, path, cx))
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(Label::new(label).size(LabelSize::Small).truncate())
                    .child(
                        Label::new(relative_display.clone())
                            .size(LabelSize::XSmall)
                            .color(Color::Muted)
                            .truncate(),
                    ),
            )
            .child(
                Label::new(media_kind_label(kind))
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
            .child(
                Button::new(
                    format!("media-panel-preview-{}", relative_display.as_ref()),
                    "Preview",
                )
                .style(ButtonStyle::Subtle)
                .size(ButtonSize::Compact)
                .on_click(cx.listener(move |panel, _, window, cx| {
                    panel.preview_media_asset(preview_payload.clone(), window, cx);
                })),
            )
            .child(
                Button::new(
                    format!("media-panel-copy-path-{}", relative_display.as_ref()),
                    "Copy",
                )
                .style(ButtonStyle::Subtle)
                .size(ButtonSize::Compact)
                .on_click(cx.listener(move |panel, _, _, cx| {
                    panel.copy_media_source(copy_path.clone(), copy_label.clone(), cx);
                })),
            )
    }

    fn render_remote_asset_row(
        &self,
        asset: RemoteMediaAsset,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let url = asset.url.to_string();
        let label = asset.label.to_string();
        let provider = asset.provider.to_string();
        let license = asset.license.to_string();
        let kind = asset.kind;

        h_flex()
            .id(format!("media-panel-remote-row-{}", asset.id))
            .gap_2()
            .items_center()
            .p_2()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().element_background)
            .hover(|style| style.bg(cx.theme().colors().element_hover))
            .tooltip(Tooltip::text(url.clone()))
            .on_click(cx.listener({
                let url = url.clone();
                let label = label.clone();
                move |panel, _, window, cx| {
                    panel.preview_media_url(url.clone(), kind, label.clone(), window, cx);
                }
            }))
            .child(remote_media_thumbnail(asset, cx))
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(Label::new(label.clone()).size(LabelSize::Small).truncate())
                    .child(
                        Label::new(format!("{provider} - {license}"))
                            .size(LabelSize::XSmall)
                            .color(Color::Muted)
                            .truncate(),
                    ),
            )
            .child(
                Label::new(media_kind_label(kind))
                    .size(LabelSize::XSmall)
                    .color(Color::Accent),
            )
            .child(
                Button::new(
                    format!("media-panel-preview-remote-{}", asset.id),
                    "Preview",
                )
                .style(ButtonStyle::Subtle)
                .size(ButtonSize::Compact)
                .on_click(cx.listener({
                    let url = url.clone();
                    let label = label.clone();
                    move |panel, _, window, cx| {
                        panel.preview_media_url(url.clone(), kind, label.clone(), window, cx);
                    }
                })),
            )
            .child(
                Button::new(
                    format!("media-panel-insert-remote-{}", asset.id),
                    "Insert URL",
                )
                .style(ButtonStyle::Subtle)
                .size(ButtonSize::Compact)
                .on_click(cx.listener(move |panel, _, window, cx| {
                    panel.insert_media_url(url.clone(), kind, label.clone(), window, cx);
                })),
            )
    }

    fn render_remote_browser_row(
        &self,
        provider_count: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let description = format!(
            "{provider_count} no-key remote sources for {}",
            self.kind_filter.label().to_lowercase()
        );
        h_flex()
            .id("media-panel-remote-browser-row")
            .gap_2()
            .items_center()
            .p_2()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().element_background)
            .child(
                div()
                    .w(px(64.))
                    .h(px(48.))
                    .rounded_sm()
                    .border_1()
                    .border_color(cx.theme().colors().border_variant)
                    .bg(cx.theme().colors().elevated_surface_background)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(Icon::new(IconName::Public).size(IconSize::Medium)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(Label::new("Browse remote providers").size(LabelSize::Small))
                    .child(
                        Label::new(description)
                            .size(LabelSize::XSmall)
                            .color(Color::Muted)
                            .truncate(),
                    ),
            )
            .child(
                Button::new("media-panel-browse-remote-row", "Open")
                    .style(ButtonStyle::Subtle)
                    .size(ButtonSize::Compact)
                    .on_click(cx.listener(|panel, _, window, cx| {
                        panel.browse_remote_media(window, cx);
                    })),
            )
    }
}

impl Panel for MediaPanel {
    fn persistent_name() -> &'static str {
        "Media"
    }

    fn panel_key() -> &'static str {
        MEDIA_PANEL_KEY
    }

    fn position(&self, _: &Window, _: &App) -> DockPosition {
        DockPosition::Right
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        position == DockPosition::Right
    }

    fn set_position(
        &mut self,
        _position: DockPosition,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    fn default_size(&self, _: &Window, _: &App) -> Pixels {
        px(340.)
    }

    fn min_size(&self, _: &Window, _: &App) -> Option<Pixels> {
        Some(px(240.))
    }

    fn icon(&self, _: &Window, _: &App) -> Option<IconName> {
        None
    }

    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> {
        Some("Media")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(ToggleFocus)
    }

    fn activation_priority(&self) -> u32 {
        11
    }
}

impl Focusable for MediaPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.filter_editor.focus_handle(cx)
    }
}

impl EventEmitter<PanelEvent> for MediaPanel {}

impl Render for MediaPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_media_index_loaded(cx);
        let (assets, total_asset_matches) = self.matching_assets(cx, MAX_MEDIA_RESULTS);
        let (remote_assets, total_remote_matches) =
            self.matching_remote_assets(cx, MAX_MEDIA_RESULTS.saturating_sub(assets.len()));
        let url_insert = self
            .render_url_insert(cx)
            .map(|element| element.into_any_element());
        let shown_count =
            total_asset_matches + total_remote_matches + usize::from(url_insert.is_some());
        let total_count = self.filtered_count(self.kind_filter);
        let mut asset_rows = remote_assets
            .iter()
            .cloned()
            .map(|asset| self.render_remote_asset_row(asset, cx).into_any_element())
            .collect::<Vec<_>>();
        asset_rows.extend(
            assets
                .iter()
                .cloned()
                .map(|asset| self.render_asset_row(asset, cx).into_any_element()),
        );
        if let Some(url_insert) = url_insert {
            asset_rows.insert(0, url_insert);
        }
        let provider_count = remote_provider_count(self.kind_filter);
        if provider_count > 0 {
            asset_rows.push(
                self.render_remote_browser_row(provider_count, cx)
                    .into_any_element(),
            );
        }
        let is_empty = asset_rows.is_empty();
        let count_label = self.status.clone().unwrap_or_else(|| {
            if self.loading {
                "indexing".into()
            } else {
                format!("{shown_count} / {total_count}").into()
            }
        });

        v_flex()
            .id("media-panel")
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().colors().panel_background)
            .child(
                v_flex()
                    .gap_2()
                    .p_2()
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(Label::new("Media").size(LabelSize::Small))
                            .child(
                                Label::new(count_label)
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted)
                                    .truncate(),
                            ),
                    )
                    .child(self.filter_editor.clone()),
            )
            .child(self.render_kind_filters(cx))
            .child(
                div()
                    .image_cache(gpui::retain_all("media-panel-assets"))
                    .id("media-panel-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .p_2()
                    .when(is_empty, |this| {
                        this.child(
                            div().h_full().flex().items_center().justify_center().child(
                                Label::new(if self.loading {
                                    "Indexing media"
                                } else {
                                    "No matching media"
                                })
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                            ),
                        )
                    })
                    .when(!is_empty, |this| {
                        this.child(v_flex().gap_2().children(asset_rows))
                    }),
            )
    }
}

fn scroll_tab_handle(handle: &ScrollHandle, direction: f32) {
    let current = handle.offset();
    let max = handle.max_offset();
    let mut next_x = current.x - px(direction * 160.0);
    let min_x = -max.x;
    if next_x < min_x {
        next_x = min_x;
    }
    if next_x > px(0.) {
        next_x = px(0.);
    }
    handle.set_offset(point(next_x, current.y));
}

struct MediaDragPreview {
    media: DraggedMediaAsset,
    position: gpui::Point<Pixels>,
}

impl Render for MediaDragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .left(self.position.x - px(56.))
            .top(self.position.y - px(32.))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(cx.theme().colors().border)
                    .bg(cx.theme().colors().elevated_surface_background)
                    .shadow_md()
                    .child(Icon::new(media_kind_icon(self.media.kind)).size(IconSize::Small))
                    .child(Label::new(self.media.label.clone()).size(LabelSize::XSmall)),
            )
    }
}

fn media_roots_for_workspace(workspace: &Workspace, cx: &App) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let project = workspace.project().read(cx);

    for worktree in project.visible_worktrees(cx) {
        let root = worktree.read(cx).abs_path().to_path_buf();
        for candidate in [
            root.join("inspirations").join("media"),
            root.join("media"),
            root.join("assets").join("media"),
            root.join("public").join("media"),
        ] {
            if candidate.is_dir() && !has_ignored_media_segment(&candidate) {
                roots.push(candidate);
            }
        }
    }

    if roots.is_empty()
        && let Ok(current_dir) = std::env::current_dir()
    {
        let candidate = current_dir.join("inspirations").join("media");
        if candidate.is_dir() && !has_ignored_media_segment(&candidate) {
            roots.push(candidate);
        }
    }

    roots.sort();
    roots.dedup();
    roots
}

fn gather_media_assets(roots: Vec<PathBuf>) -> Vec<MediaAsset> {
    let mut assets = Vec::new();
    for root in roots {
        gather_media_assets_in_root(&root, &root, &mut assets);
    }
    assets.sort_by(|left, right| {
        left.payload
            .relative_display
            .as_ref()
            .cmp(right.payload.relative_display.as_ref())
    });
    assets
}

fn gather_media_assets_in_root(root: &PathBuf, path: &Path, assets: &mut Vec<MediaAsset>) {
    if assets.len() >= MAX_MEDIA_RESULTS || has_ignored_media_segment(path) {
        return;
    }

    let Ok(entries) = std_fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            gather_media_assets_in_root(root, &path, assets);
        } else if let Some(kind) = media_kind_for_path(&path) {
            let payload = DraggedMediaAsset::new(path.clone(), kind, root);
            let search_text = format!(
                "{} {} {}",
                payload.label.as_ref().to_lowercase(),
                payload.relative_display.as_ref().to_lowercase(),
                media_kind_label(kind)
            );
            assets.push(MediaAsset {
                payload,
                search_text: search_text.into(),
            });
        }

        if assets.len() >= MAX_MEDIA_RESULTS {
            break;
        }
    }
}

fn has_ignored_media_segment(path: &Path) -> bool {
    path.components().any(|component| {
        let segment = component.as_os_str().to_string_lossy().to_lowercase();
        matches!(
            segment.as_str(),
            ".git"
                | ".cache"
                | "target"
                | "tmp"
                | "trash"
                | "models"
                | "tools"
                | "tool"
                | "mcp"
                | "node_modules"
        )
    })
}

fn media_kind_for_path(path: &Path) -> Option<DraggedMediaKind> {
    media_kind_for_extension(path.extension()?.to_str()?)
}

fn media_kind_for_extension(extension: &str) -> Option<DraggedMediaKind> {
    match extension.to_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "avif" | "svg" => {
            Some(DraggedMediaKind::Image)
        }
        "mp4" | "webm" | "mov" | "m4v" | "avi" => Some(DraggedMediaKind::Video),
        "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" | "opus" => Some(DraggedMediaKind::Audio),
        _ => None,
    }
}

struct MediaUrlCandidate {
    url: String,
    kind: DraggedMediaKind,
    label: String,
}

fn media_url_candidate(
    query: &str,
    fallback_kind: Option<DraggedMediaKind>,
) -> Option<MediaUrlCandidate> {
    let url = query.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return None;
    }

    let path = url
        .split(|character| character == '?' || character == '#')
        .next()
        .unwrap_or(url)
        .trim_end_matches('/');
    let file_name = path.rsplit('/').next().unwrap_or("media");
    let kind = file_name
        .rsplit_once('.')
        .and_then(|(_, extension)| media_kind_for_extension(extension))
        .or(fallback_kind)?;
    let label = media_label_from_url(file_name);

    Some(MediaUrlCandidate {
        url: url.to_string(),
        kind,
        label,
    })
}

fn media_label_from_url(file_name: &str) -> String {
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    let label = stem
        .chars()
        .map(|character| {
            if character == '_' || character == '-' {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if label.is_empty() {
        "media".to_string()
    } else {
        label
    }
}

fn local_media_preview_url(
    label: &str,
    kind: DraggedMediaKind,
    source: MediaPreviewSource,
) -> Option<String> {
    let source_url = match source {
        MediaPreviewSource::Local(path) => Url::from_file_path(path).ok()?.to_string(),
        MediaPreviewSource::Remote(url) => url,
    };
    let preview_dir = repo_root().join("target").join("media-previews");
    std_fs::create_dir_all(&preview_dir).ok()?;
    let preview_path = preview_dir.join(format!(
        "{}-{}.html",
        media_kind_label(kind),
        preview_file_stem(label)
    ));
    let html = media_preview_html(label, kind, &source_url);
    std_fs::write(&preview_path, html).ok()?;
    Url::from_file_path(preview_path)
        .ok()
        .map(|url| url.to_string())
}

fn remote_media_search_url(query: &str, filter: MediaKindFilter) -> Option<String> {
    let preview_dir = repo_root().join("target").join("media-remote-search");
    std_fs::create_dir_all(&preview_dir).ok()?;
    let label = if query.trim().is_empty() {
        "creative workspace".to_string()
    } else {
        query.trim().to_string()
    };
    let preview_path = preview_dir.join(format!("{}.html", preview_file_stem(&label)));
    let html = remote_media_search_html(&label, filter);
    std_fs::write(&preview_path, html).ok()?;
    Url::from_file_path(preview_path)
        .ok()
        .map(|url| url.to_string())
}

fn remote_media_search_html(query: &str, filter: MediaKindFilter) -> String {
    let title = escape_html(query);
    let query_attr = escape_attr(query);
    let initial_type = match filter {
        MediaKindFilter::Videos => "video",
        MediaKindFilter::Audio => "audio",
        _ => "image",
    };
    let provider_links = free_media_sources()
        .iter()
        .filter_map(|source| {
            let search_url = source.search_url(query, filter)?;
            Some(format!(
                r#"<a class="provider" data-provider="{}" title="{}" href="{}">{}</a>"#,
                escape_attr(source.id),
                escape_attr(source.description),
                escape_attr(&search_url),
                escape_html(source.name)
            ))
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<!doctype html>
<html lang="en" class="dark">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title} - Zed remote media</title>
  <style>
    :root {{
      color-scheme: dark;
      --background: #09090b;
      --foreground: #f4f4f5;
      --card: #101113;
      --muted: #18181b;
      --muted-foreground: #a1a1aa;
      --border: #27272a;
      --accent: #3fb950;
      --accent-soft: rgba(63, 185, 80, .16);
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      background: var(--background);
      color: var(--foreground);
      font: 13px/1.5 Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      overflow: hidden;
    }}
    .scroll-area {{
      height: 100vh;
      overflow: auto;
      scrollbar-width: thin;
      scrollbar-color: var(--border) transparent;
    }}
    .scroll-area::-webkit-scrollbar {{ width: 10px; height: 10px; }}
    .scroll-area::-webkit-scrollbar-thumb {{
      background: var(--border);
      border: 3px solid transparent;
      border-radius: 999px;
      background-clip: padding-box;
    }}
    main {{
      width: min(1180px, calc(100vw - 32px));
      margin: 0 auto;
      padding: 18px 0 28px;
      display: grid;
      gap: 14px;
    }}
    header, .toolbar, .providers, .status {{
      border: 1px solid var(--border);
      border-radius: 8px;
      background: color-mix(in srgb, var(--card) 94%, transparent);
    }}
    header {{ padding: 16px; }}
    h1 {{ margin: 0; font-size: 22px; letter-spacing: 0; }}
    p {{ margin: 4px 0 0; color: var(--muted-foreground); }}
    .toolbar {{
      display: grid;
      grid-template-columns: 1fr auto auto;
      gap: 10px;
      padding: 10px;
      align-items: center;
      position: sticky;
      top: 0;
      z-index: 2;
    }}
    input, select, button {{
      height: 32px;
      border-radius: 6px;
      border: 1px solid var(--border);
      background: var(--muted);
      color: var(--foreground);
      padding: 0 10px;
      font: inherit;
    }}
    button {{
      background: var(--accent);
      color: #031108;
      font-weight: 600;
      cursor: pointer;
    }}
    .providers {{
      padding: 10px;
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
    }}
    .provider {{
      color: var(--accent);
      background: var(--accent-soft);
      border: 1px solid color-mix(in srgb, var(--accent) 40%, var(--border));
      border-radius: 999px;
      padding: 4px 10px;
      text-decoration: none;
    }}
    .status {{ padding: 10px 12px; color: var(--muted-foreground); }}
    .grid {{
      display: grid;
      gap: 12px;
      grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    }}
    .card {{
      min-width: 0;
      overflow: hidden;
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--card);
      display: grid;
      gap: 8px;
    }}
    .thumb {{
      width: 100%;
      aspect-ratio: 4 / 3;
      object-fit: cover;
      background: #050506;
      border-bottom: 1px solid var(--border);
    }}
    .body {{ display: grid; gap: 6px; padding: 10px; }}
    .title {{ font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
    .meta {{ color: var(--muted-foreground); font-size: 12px; }}
    .actions {{ display: flex; gap: 8px; flex-wrap: wrap; }}
    .actions a, .copy {{
      border-radius: 6px;
      border: 1px solid var(--border);
      color: var(--foreground);
      background: var(--muted);
      text-decoration: none;
      padding: 4px 8px;
      cursor: pointer;
    }}
    .copy {{ color: var(--accent); }}
  </style>
</head>
<body>
  <div class="scroll-area">
    <main>
      <header>
        <h1>Remote media</h1>
        <p>Search no-key open media APIs first, with every configured provider one click away.</p>
      </header>
      <section class="toolbar">
        <input id="query" value="{query_attr}" />
        <select id="type">
          <option value="image">Images</option>
          <option value="video">Videos</option>
          <option value="audio">Audio</option>
        </select>
        <button id="search">Search</button>
      </section>
      <section class="providers">{provider_links}</section>
      <section id="status" class="status">Loading remote assets...</section>
      <section id="grid" class="grid"></section>
    </main>
  </div>
  <script>
    const initialType = "{initial_type}";
    const queryInput = document.getElementById("query");
    const typeInput = document.getElementById("type");
    const status = document.getElementById("status");
    const grid = document.getElementById("grid");
    typeInput.value = initialType;

    const escapeText = (value) => String(value ?? "").replace(/[&<>"']/g, (ch) => ({{
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#39;",
    }}[ch]));

    const card = (item) => `
      <article class="card">
        <img class="thumb" src="${{escapeText(item.thumbnail || item.url)}}" loading="lazy" alt="${{escapeText(item.title)}}" />
        <div class="body">
          <div class="title">${{escapeText(item.title || "Untitled media")}}</div>
          <div class="meta">${{escapeText(item.provider)}} - ${{escapeText(item.license || item.kind || "open media")}}</div>
          <div class="actions">
            <a href="${{escapeText(item.url)}}" target="_blank">Open</a>
            <a href="${{escapeText(item.source || item.url)}}" target="_blank">Source</a>
            <button class="copy" data-url="${{escapeText(item.url)}}">Copy URL</button>
          </div>
        </div>
      </article>`;

    async function searchOpenverse(query, type) {{
      if (type === "audio") return [];
      const endpoint = type === "video" ? "videos" : "images";
      const response = await fetch(`https://api.openverse.org/v1/${{endpoint}}/?q=${{encodeURIComponent(query)}}&page_size=30`);
      const json = await response.json();
      return (json.results || []).map((item) => ({{
        provider: "Openverse",
        title: item.title,
        url: item.url,
        thumbnail: item.thumbnail || item.url,
        source: item.foreign_landing_url || item.url,
        license: item.license,
        kind: type,
      }}));
    }}

    async function searchNasa(query, type) {{
      if (type === "audio") return [];
      const media = type === "video" ? "video" : "image";
      const response = await fetch(`https://images-api.nasa.gov/search?q=${{encodeURIComponent(query)}}&media_type=${{media}}`);
      const json = await response.json();
      return ((json.collection || {{}}).items || []).slice(0, 24).map((item) => {{
        const link = (item.links || [])[0] || {{}};
        const data = (item.data || [])[0] || {{}};
        return {{
          provider: "NASA",
          title: data.title,
          url: link.href || item.href,
          thumbnail: link.href,
          source: item.href,
          license: "NASA",
          kind: media,
        }};
      }}).filter((item) => item.url);
    }}

    async function searchWikimedia(query, type) {{
      if (type !== "image") return [];
      const params = new URLSearchParams({{
        action: "query",
        generator: "search",
        gsrsearch: query,
        gsrnamespace: "6",
        gsrlimit: "24",
        prop: "imageinfo",
        iiprop: "url|mime|extmetadata",
        iiurlwidth: "640",
        format: "json",
        origin: "*",
      }});
      const response = await fetch(`https://commons.wikimedia.org/w/api.php?${{params}}`);
      const json = await response.json();
      return Object.values((json.query || {{}}).pages || {{}}).map((page) => {{
        const info = (page.imageinfo || [])[0] || {{}};
        return {{
          provider: "Wikimedia",
          title: page.title,
          url: info.url,
          thumbnail: info.thumburl || info.url,
          source: info.descriptionurl || info.url,
          license: ((info.extmetadata || {{}}).LicenseShortName || {{}}).value,
          kind: "image",
        }};
      }}).filter((item) => item.url);
    }}

    async function runSearch() {{
      const query = queryInput.value.trim() || "creative workspace";
      const type = typeInput.value;
      status.textContent = "Searching Openverse, Wikimedia, and NASA...";
      grid.innerHTML = "";
      const settled = await Promise.allSettled([
        searchOpenverse(query, type),
        searchWikimedia(query, type),
        searchNasa(query, type),
      ]);
      const items = settled.flatMap((result) => result.status === "fulfilled" ? result.value : []);
      grid.innerHTML = items.map(card).join("");
      status.textContent = items.length
        ? `Showing ${{items.length}} remote assets. Use provider chips for deeper searches.`
        : "No no-key API results found. Try the provider chips above.";
    }}

    document.getElementById("search").addEventListener("click", runSearch);
    queryInput.addEventListener("keydown", (event) => {{
      if (event.key === "Enter") runSearch();
    }});
    grid.addEventListener("click", async (event) => {{
      const button = event.target.closest(".copy");
      if (!button) return;
      await navigator.clipboard.writeText(button.dataset.url);
      button.textContent = "Copied";
      setTimeout(() => button.textContent = "Copy URL", 1200);
    }});
    runSearch();
  </script>
</body>
</html>"#,
    )
}

fn media_preview_html(label: &str, kind: DraggedMediaKind, source_url: &str) -> String {
    let title = escape_html(label);
    let source = escape_attr(source_url);
    let media = match kind {
        DraggedMediaKind::Image => {
            format!(r#"<img class="viewer-media" src="{source}" alt="{title}" />"#)
        }
        DraggedMediaKind::Video => {
            format!(r#"<video class="viewer-media" src="{source}" controls autoplay></video>"#)
        }
        DraggedMediaKind::Audio => {
            format!(
                r#"<div class="audio-shell" aria-label="{title}"><div class="audio-mark"></div><h1>{title}</h1><audio class="viewer-audio" src="{source}" controls autoplay></audio></div>"#
            )
        }
    };

    format!(
        r#"<!doctype html>
<html lang="en" class="dark">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title} - Zed media preview</title>
  <style>
    :root {{
      color-scheme: dark;
      --background: #09090b;
      --panel: #101113;
      --border: #272a2f;
      --foreground: #f4f4f5;
      --muted: #a1a1aa;
      --accent: #3fb950;
      --accent-soft: rgba(63, 185, 80, .16);
    }}
    * {{ box-sizing: border-box; }}
    html, body {{ height: 100%; }}
    body {{
      margin: 0;
      background: var(--background);
      color: var(--foreground);
      font: 13px/1.5 Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      overflow: hidden;
    }}
    .stage {{
      width: 100%;
      height: 100vh;
      display: grid;
      place-items: center;
      overflow: hidden;
      background: #050506;
    }}
    .viewer-media {{
      width: 100vw;
      height: 100vh;
      object-fit: contain;
      background: #050506;
      display: block;
    }}
    .audio-shell {{
      width: min(760px, calc(100vw - 48px));
      display: grid;
      justify-items: center;
      gap: 22px;
      padding: 40px;
      border: 1px solid var(--border);
      border-radius: 8px;
      background: color-mix(in srgb, var(--panel) 94%, transparent);
      box-shadow: 0 24px 80px rgba(0, 0, 0, .34);
    }}
    .audio-mark {{
      width: 128px;
      aspect-ratio: 1;
      border-radius: 999px;
      background:
        radial-gradient(circle at center, var(--background) 0 17%, transparent 18%),
        conic-gradient(from 130deg, var(--accent), #6ee7b7, #64748b, var(--accent));
      box-shadow: 0 0 0 1px var(--border), 0 0 70px rgba(63, 185, 80, .25);
    }}
    h1 {{
      margin: 0;
      max-width: 100%;
      font-size: 20px;
      letter-spacing: 0;
      text-align: center;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}
    .viewer-audio {{ width: 100%; }}
  </style>
</head>
<body>
  <section class="stage">{media}</section>
</body>
</html>"#,
    )
}

fn preview_file_stem(label: &str) -> String {
    let normalized = label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let stem = normalized
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if stem.is_empty() {
        "media".to_string()
    } else {
        stem
    }
}

fn repo_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("G:/Zed"))
}

fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn escape_attr(text: &str) -> String {
    escape_html(text)
}

fn media_thumbnail(
    kind: DraggedMediaKind,
    path: PathBuf,
    cx: &mut Context<MediaPanel>,
) -> AnyElement {
    match kind {
        DraggedMediaKind::Image => div()
            .w(px(64.))
            .h(px(48.))
            .rounded_sm()
            .overflow_hidden()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .child(img(path).size_full().object_fit(ObjectFit::ScaleDown))
            .into_any_element(),
        DraggedMediaKind::Video | DraggedMediaKind::Audio => div()
            .w(px(64.))
            .h(px(48.))
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().elevated_surface_background)
            .flex()
            .items_center()
            .justify_center()
            .child(Icon::new(media_kind_icon(kind)).size(IconSize::Medium))
            .into_any_element(),
    }
}

fn remote_media_thumbnail(asset: RemoteMediaAsset, cx: &mut Context<MediaPanel>) -> AnyElement {
    match asset.kind {
        DraggedMediaKind::Image => div()
            .w(px(64.))
            .h(px(48.))
            .rounded_sm()
            .overflow_hidden()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .child(img(asset.url).size_full().object_fit(ObjectFit::Cover))
            .into_any_element(),
        DraggedMediaKind::Video => div()
            .w(px(64.))
            .h(px(48.))
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().elevated_surface_background)
            .v_flex()
            .gap_0p5()
            .p_1()
            .child(
                div()
                    .flex_1()
                    .rounded_sm()
                    .bg(cx.theme().colors().element_background)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(Icon::new(IconName::PlayOutlined).size(IconSize::Small)),
            )
            .child(
                h_flex()
                    .gap_0p5()
                    .child(
                        div()
                            .flex_1()
                            .h(px(3.))
                            .rounded_full()
                            .bg(cx.theme().colors().border),
                    )
                    .child(
                        div()
                            .w(px(12.))
                            .h(px(3.))
                            .rounded_full()
                            .bg(cx.theme().colors().border_focused),
                    ),
            )
            .into_any_element(),
        DraggedMediaKind::Audio => div()
            .w(px(64.))
            .h(px(48.))
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().elevated_surface_background)
            .h_flex()
            .gap_0p5()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(4.))
                    .h(px(14.))
                    .rounded_full()
                    .bg(cx.theme().colors().border),
            )
            .child(
                div()
                    .w(px(4.))
                    .h(px(26.))
                    .rounded_full()
                    .bg(cx.theme().colors().border_focused),
            )
            .child(
                div()
                    .w(px(4.))
                    .h(px(18.))
                    .rounded_full()
                    .bg(cx.theme().colors().border),
            )
            .child(
                div()
                    .w(px(4.))
                    .h(px(30.))
                    .rounded_full()
                    .bg(cx.theme().colors().border_focused),
            )
            .child(
                div()
                    .w(px(4.))
                    .h(px(16.))
                    .rounded_full()
                    .bg(cx.theme().colors().border),
            )
            .into_any_element(),
    }
}

fn media_kind_icon(kind: DraggedMediaKind) -> IconName {
    match kind {
        DraggedMediaKind::Image => IconName::Image,
        DraggedMediaKind::Video => IconName::PlayOutlined,
        DraggedMediaKind::Audio => IconName::AudioOn,
    }
}

fn media_kind_label(kind: DraggedMediaKind) -> &'static str {
    match kind {
        DraggedMediaKind::Image => "image",
        DraggedMediaKind::Video => "video",
        DraggedMediaKind::Audio => "audio",
    }
}

fn media_search_matches(searchable: &str, query: &str) -> bool {
    query
        .split_whitespace()
        .all(|term| searchable.contains(term))
}

fn remote_media_assets() -> &'static [RemoteMediaAsset] {
    &[
        RemoteMediaAsset {
            id: "wikimedia-fronalpstock",
            label: "Fronalpstock landscape",
            provider: "Wikimedia Commons",
            url: "https://upload.wikimedia.org/wikipedia/commons/3/3f/Fronalpstock_big.jpg",
            kind: DraggedMediaKind::Image,
            license: "CC BY-SA",
            tags: "mountain landscape nature travel hero background",
        },
        RemoteMediaAsset {
            id: "wikimedia-van-gogh-starry-night",
            label: "The Starry Night",
            provider: "Wikimedia Commons",
            url: "https://upload.wikimedia.org/wikipedia/commons/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg",
            kind: DraggedMediaKind::Image,
            license: "public domain",
            tags: "painting art night museum impressionism texture",
        },
        RemoteMediaAsset {
            id: "wikimedia-mona-lisa",
            label: "Mona Lisa",
            provider: "Wikimedia Commons",
            url: "https://upload.wikimedia.org/wikipedia/commons/6/6a/Mona_Lisa.jpg",
            kind: DraggedMediaKind::Image,
            license: "public domain",
            tags: "portrait art museum renaissance people",
        },
        RemoteMediaAsset {
            id: "wikimedia-hubble-deep-field",
            label: "Hubble Deep Field",
            provider: "NASA / Wikimedia",
            url: "https://upload.wikimedia.org/wikipedia/commons/5/5f/HubbleDeepField.800px.jpg",
            kind: DraggedMediaKind::Image,
            license: "public domain",
            tags: "space galaxy nasa stars astronomy background",
        },
        RemoteMediaAsset {
            id: "wikimedia-great-wave",
            label: "The Great Wave",
            provider: "Wikimedia Commons",
            url: "https://upload.wikimedia.org/wikipedia/commons/a/a5/Tsunami_by_hokusai_19th_century.jpg",
            kind: DraggedMediaKind::Image,
            license: "public domain",
            tags: "wave ocean japan illustration art print",
        },
        RemoteMediaAsset {
            id: "wikimedia-blue-marble",
            label: "Blue Marble",
            provider: "NASA",
            url: "https://upload.wikimedia.org/wikipedia/commons/9/97/The_Earth_seen_from_Apollo_17.jpg",
            kind: DraggedMediaKind::Image,
            license: "public domain",
            tags: "earth space planet nasa globe science",
        },
        RemoteMediaAsset {
            id: "nasa-mars-pathfinder",
            label: "Mars Pathfinder panorama",
            provider: "NASA",
            url: "https://images-assets.nasa.gov/image/PIA00452/PIA00452~orig.jpg",
            kind: DraggedMediaKind::Image,
            license: "public domain",
            tags: "mars nasa space rover science panorama planet",
        },
        RemoteMediaAsset {
            id: "mdn-flower-video",
            label: "Flower video",
            provider: "MDN",
            url: "https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4",
            kind: DraggedMediaKind::Video,
            license: "CC0",
            tags: "flower nature macro loop video motion",
        },
        RemoteMediaAsset {
            id: "mdn-flower-webm",
            label: "Flower video WebM",
            provider: "MDN",
            url: "https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.webm",
            kind: DraggedMediaKind::Video,
            license: "CC0",
            tags: "flower nature macro webm loop motion",
        },
        RemoteMediaAsset {
            id: "wikimedia-big-buck-bunny",
            label: "Big Buck Bunny sample",
            provider: "Wikimedia Commons",
            url: "https://upload.wikimedia.org/wikipedia/commons/transcoded/7/70/Big.Buck.Bunny.-.Opening.Screen.ogv/Big.Buck.Bunny.-.Opening.Screen.ogv.360p.webm",
            kind: DraggedMediaKind::Video,
            license: "CC BY",
            tags: "animation sample open movie video webm",
        },
        RemoteMediaAsset {
            id: "blender-big-buck-bunny-mp4",
            label: "Big Buck Bunny MP4",
            provider: "Blender Open Movie",
            url: "https://download.blender.org/peach/bigbuckbunny_movies/BigBuckBunny_320x180.mp4",
            kind: DraggedMediaKind::Video,
            license: "CC BY",
            tags: "animation open movie blender video mp4 sample",
        },
        RemoteMediaAsset {
            id: "blender-sintel-trailer",
            label: "Sintel trailer",
            provider: "Blender Open Movie",
            url: "https://download.blender.org/durian/trailer/sintel_trailer-480p.mp4",
            kind: DraggedMediaKind::Video,
            license: "CC BY",
            tags: "animation fantasy trailer blender video mp4 open movie",
        },
        RemoteMediaAsset {
            id: "wikimedia-example-audio",
            label: "Example audio",
            provider: "Wikimedia Commons",
            url: "https://upload.wikimedia.org/wikipedia/commons/c/c8/Example.ogg",
            kind: DraggedMediaKind::Audio,
            license: "public sample",
            tags: "speech sample sound audio ogg",
        },
        RemoteMediaAsset {
            id: "mdn-t-rex-roar",
            label: "T-Rex roar",
            provider: "MDN",
            url: "https://interactive-examples.mdn.mozilla.net/media/cc0-audio/t-rex-roar.mp3",
            kind: DraggedMediaKind::Audio,
            license: "CC0",
            tags: "sound effect roar mp3 sample",
        },
        RemoteMediaAsset {
            id: "wikimedia-bach-brandenburg",
            label: "Bach Brandenburg sample",
            provider: "Wikimedia Commons",
            url: "https://upload.wikimedia.org/wikipedia/commons/4/45/Bach_-_Brandenburg_Concerto_No._3_-_1._Allegro.ogg",
            kind: DraggedMediaKind::Audio,
            license: "public domain",
            tags: "classical music bach orchestral sample",
        },
        RemoteMediaAsset {
            id: "soundhelix-song-1",
            label: "SoundHelix music sample",
            provider: "SoundHelix",
            url: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3",
            kind: DraggedMediaKind::Audio,
            license: "royalty-free sample",
            tags: "music mp3 sample soundtrack background audio",
        },
    ]
}

struct FreeMediaSource {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    homepage: &'static str,
    image_search: &'static str,
    video_search: Option<&'static str>,
    audio_search: Option<&'static str>,
}

impl FreeMediaSource {
    fn search_url(&self, query: &str, filter: MediaKindFilter) -> Option<String> {
        let search_template = self.search_template(filter)?;
        if query.trim().is_empty() {
            Some(self.homepage.to_string())
        } else {
            Some(search_template.replace("{query}", &encode_query(query)))
        }
    }

    fn search_template(&self, filter: MediaKindFilter) -> Option<&'static str> {
        match filter {
            MediaKindFilter::Videos => self.video_search,
            MediaKindFilter::Audio => self.audio_search,
            MediaKindFilter::All | MediaKindFilter::Images => Some(self.image_search),
        }
    }
}

fn remote_provider_count(filter: MediaKindFilter) -> usize {
    free_media_sources()
        .iter()
        .filter(|source| source.search_template(filter).is_some())
        .count()
}

fn free_media_sources() -> &'static [FreeMediaSource] {
    &[
        FreeMediaSource {
            id: "openverse",
            name: "Openverse",
            description: "No-key Creative Commons image and audio search; broadest safe default",
            homepage: "https://openverse.org/",
            image_search: "https://openverse.org/search/image?q={query}",
            video_search: None,
            audio_search: Some("https://openverse.org/search/audio?q={query}"),
        },
        FreeMediaSource {
            id: "wikimedia",
            name: "Wikimedia",
            description: "No-key Wikimedia Commons media search",
            homepage: "https://commons.wikimedia.org/",
            image_search: "https://commons.wikimedia.org/w/index.php?search={query}&title=Special:MediaSearch&type=image",
            video_search: Some(
                "https://commons.wikimedia.org/w/index.php?search={query}&title=Special:MediaSearch&type=video",
            ),
            audio_search: Some(
                "https://commons.wikimedia.org/w/index.php?search={query}&title=Special:MediaSearch&type=audio",
            ),
        },
        FreeMediaSource {
            id: "met",
            name: "The Met",
            description: "No-key public-domain and open-access museum images",
            homepage: "https://www.metmuseum.org/art/collection",
            image_search: "https://www.metmuseum.org/art/collection/search?q={query}&showOnly=openAccess",
            video_search: None,
            audio_search: None,
        },
        FreeMediaSource {
            id: "nasa",
            name: "NASA",
            description: "No-key NASA image and video archive",
            homepage: "https://images.nasa.gov/",
            image_search: "https://images.nasa.gov/search?q={query}&media=image",
            video_search: Some("https://images.nasa.gov/search?q={query}&media=video"),
            audio_search: Some("https://images.nasa.gov/search?q={query}&media=audio"),
        },
        FreeMediaSource {
            id: "loc",
            name: "Library of Congress",
            description: "No-key public-domain historical images and collections",
            homepage: "https://www.loc.gov/pictures/",
            image_search: "https://www.loc.gov/pictures/search/?q={query}",
            video_search: Some(
                "https://www.loc.gov/film-and-videos/?fa=online-format:video&q={query}",
            ),
            audio_search: Some("https://www.loc.gov/audio/?fa=online-format:audio&q={query}"),
        },
        FreeMediaSource {
            id: "smithsonian",
            name: "Smithsonian",
            description: "Open Access museum, science, and archive images",
            homepage: "https://www.si.edu/openaccess",
            image_search: "https://www.si.edu/search/collection-images?edan_q={query}&oa=1",
            video_search: Some("https://www.si.edu/search/collection-videos?edan_q={query}&oa=1"),
            audio_search: Some(
                "https://www.si.edu/search/collection-sound-recordings?edan_q={query}&oa=1",
            ),
        },
        FreeMediaSource {
            id: "europeana",
            name: "Europeana",
            description: "European cultural heritage media search",
            homepage: "https://www.europeana.eu/",
            image_search: "https://www.europeana.eu/en/search?query={query}&media=true",
            video_search: Some(
                "https://www.europeana.eu/en/search?query={query}&media=true&view=grid",
            ),
            audio_search: Some(
                "https://www.europeana.eu/en/search?query={query}&media=true&view=grid",
            ),
        },
        FreeMediaSource {
            id: "rawpixel-public-domain",
            name: "Rawpixel Public Domain",
            description: "Public-domain design and art image search",
            homepage: "https://www.rawpixel.com/category/53/public-domain",
            image_search: "https://www.rawpixel.com/search/{query}?sort=curated&topic_group=_topics",
            video_search: None,
            audio_search: None,
        },
        FreeMediaSource {
            id: "unsplash",
            name: "Unsplash",
            description: "Free photo site; API requires a key, web search is open",
            homepage: "https://unsplash.com/",
            image_search: "https://unsplash.com/s/photos/{query}",
            video_search: None,
            audio_search: None,
        },
        FreeMediaSource {
            id: "pexels",
            name: "Pexels",
            description: "Free stock photos and videos; API requires a key",
            homepage: "https://www.pexels.com/",
            image_search: "https://www.pexels.com/search/{query}/",
            video_search: Some("https://www.pexels.com/search/videos/{query}/"),
            audio_search: None,
        },
        FreeMediaSource {
            id: "pixabay",
            name: "Pixabay",
            description: "Free images, video, and audio; API requires a key",
            homepage: "https://pixabay.com/",
            image_search: "https://pixabay.com/images/search/{query}/",
            video_search: Some("https://pixabay.com/videos/search/{query}/"),
            audio_search: Some("https://pixabay.com/sound-effects/search/{query}/"),
        },
        FreeMediaSource {
            id: "burst",
            name: "Burst",
            description: "Free stock photos from Shopify",
            homepage: "https://burst.shopify.com/",
            image_search: "https://burst.shopify.com/photos/search?utf8=%E2%9C%93&q={query}",
            video_search: None,
            audio_search: None,
        },
        FreeMediaSource {
            id: "reshot",
            name: "Reshot",
            description: "Free icons and illustrations for product work",
            homepage: "https://www.reshot.com/",
            image_search: "https://www.reshot.com/search/{query}/",
            video_search: None,
            audio_search: None,
        },
        FreeMediaSource {
            id: "stocksnap",
            name: "StockSnap",
            description: "CC0-style free stock photo search",
            homepage: "https://stocksnap.io/",
            image_search: "https://stocksnap.io/search/{query}",
            video_search: None,
            audio_search: None,
        },
        FreeMediaSource {
            id: "kaboompics",
            name: "Kaboompics",
            description: "Free lifestyle and product photography",
            homepage: "https://kaboompics.com/",
            image_search: "https://kaboompics.com/gallery?search={query}",
            video_search: None,
            audio_search: None,
        },
        FreeMediaSource {
            id: "foodiesfeed",
            name: "Foodiesfeed",
            description: "Free food photography",
            homepage: "https://www.foodiesfeed.com/",
            image_search: "https://www.foodiesfeed.com/?s={query}",
            video_search: None,
            audio_search: None,
        },
    ]
}

fn encode_query(query: &str) -> String {
    let mut encoded = String::new();
    for byte in query.trim().bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}
