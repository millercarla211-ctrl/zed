use editor::{Editor, EditorEvent};
use futures::AsyncReadExt as _;
use gpui::{
    App, AppContext as _, AsyncWindowContext, BackgroundExecutor, ClipboardItem, Context, Entity,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, ObjectFit, Pixels, Render,
    ScrollHandle, SharedString, StatefulInteractiveElement, Subscription, WeakEntity, Window,
    actions, div, img, point, px,
};
use http_client::{AsyncBody, HttpClient};
use serde::Deserialize;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, VecDeque},
    fmt::Write as _,
    fs as std_fs,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, OnceLock},
    time::Duration,
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
const MAX_MEDIA_RESULTS: usize = 320;
const MAX_REMOTE_MEDIA_RESULTS: usize = 640;
const MAX_REMOTE_MEDIA_CACHE_ENTRIES: usize = 24;
const OPENVERSE_RESULT_LIMIT: usize = 90;
const OPENVERSE_FOCUSED_RESULT_LIMIT: usize = 150;
const WIKIMEDIA_RESULT_LIMIT: usize = 50;
const WIKIMEDIA_FOCUSED_RESULT_LIMIT: usize = 50;
const NASA_IMAGE_RESULT_LIMIT: usize = 90;
const NASA_IMAGE_FOCUSED_RESULT_LIMIT: usize = 120;
const NASA_MEDIA_SEARCH_LIMIT: usize = 36;
const NASA_MEDIA_FOCUSED_SEARCH_LIMIT: usize = 54;
const NASA_MEDIA_DETAIL_LIMIT: usize = 20;
const NASA_MEDIA_FOCUSED_DETAIL_LIMIT: usize = 32;
const LIBRARY_OF_CONGRESS_RESULT_LIMIT: usize = 90;
const LIBRARY_OF_CONGRESS_FOCUSED_RESULT_LIMIT: usize = 120;
const ART_INSTITUTE_RESULT_LIMIT: usize = 90;
const ART_INSTITUTE_FOCUSED_RESULT_LIMIT: usize = 120;
const CLEVELAND_ART_RESULT_LIMIT: usize = 90;
const CLEVELAND_ART_FOCUSED_RESULT_LIMIT: usize = 120;
const MET_MUSEUM_DETAIL_LIMIT: usize = 24;
const MET_MUSEUM_FOCUSED_DETAIL_LIMIT: usize = 40;
const INTERNET_ARCHIVE_SEARCH_LIMIT: usize = 42;
const INTERNET_ARCHIVE_FOCUSED_SEARCH_LIMIT: usize = 72;
const INTERNET_ARCHIVE_DETAIL_LIMIT: usize = 24;
const INTERNET_ARCHIVE_FOCUSED_DETAIL_LIMIT: usize = 36;
const MAX_REMOTE_JSON_BODY_RESERVE: usize = 2 * 1024 * 1024;
const REMOTE_MEDIA_FETCH_DEBOUNCE: Duration = Duration::from_millis(275);
const REMOTE_MEDIA_PROVIDER_TIMEOUT: Duration = Duration::from_secs(4);
const IGNORED_MEDIA_SEGMENTS: &[&str] = &[
    ".git",
    ".cache",
    "target",
    "tmp",
    "trash",
    "models",
    "tools",
    "tool",
    "mcp",
    "node_modules",
];
const IMAGE_MEDIA_EXTENSIONS: &[&str] =
    &["png", "jpg", "jpeg", "webp", "gif", "bmp", "avif", "svg"];
const VIDEO_MEDIA_EXTENSIONS: &[&str] = &["mp4", "webm", "mov", "m4v", "avi"];
const AUDIO_MEDIA_EXTENSIONS: &[&str] = &["mp3", "wav", "ogg", "flac", "m4a", "aac", "opus"];

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

#[derive(Clone)]
struct RemoteMediaAsset {
    id: Cow<'static, str>,
    label: Cow<'static, str>,
    provider: Cow<'static, str>,
    url: Cow<'static, str>,
    thumbnail_url: Option<Cow<'static, str>>,
    kind: DraggedMediaKind,
    license: Cow<'static, str>,
    tags: Cow<'static, str>,
}

type RemoteMediaFetch =
    Pin<Box<dyn Future<Output = (&'static str, anyhow::Result<Vec<RemoteMediaAsset>>)>>>;

#[derive(Clone)]
struct RemoteMediaCacheEntry {
    assets: Vec<RemoteMediaAsset>,
    warning: Option<SharedString>,
}

struct RemoteMediaFetchResult {
    assets: Vec<RemoteMediaAsset>,
    warning: Option<SharedString>,
}

impl RemoteMediaAsset {
    const fn borrowed(
        id: &'static str,
        label: &'static str,
        provider: &'static str,
        url: &'static str,
        kind: DraggedMediaKind,
        license: &'static str,
        tags: &'static str,
    ) -> Self {
        Self {
            id: Cow::Borrowed(id),
            label: Cow::Borrowed(label),
            provider: Cow::Borrowed(provider),
            url: Cow::Borrowed(url),
            thumbnail_url: None,
            kind,
            license: Cow::Borrowed(license),
            tags: Cow::Borrowed(tags),
        }
    }

    fn owned(
        id: String,
        label: String,
        provider: &'static str,
        url: String,
        kind: DraggedMediaKind,
        license: String,
        tags: String,
    ) -> Self {
        Self {
            id: Cow::Owned(id),
            label: Cow::Owned(label),
            provider: Cow::Borrowed(provider),
            url: Cow::Owned(url),
            thumbnail_url: None,
            kind,
            license: Cow::Owned(license),
            tags: Cow::Owned(tags),
        }
    }

    fn owned_with_thumbnail(
        id: String,
        label: String,
        provider: &'static str,
        url: String,
        thumbnail_url: Option<String>,
        kind: DraggedMediaKind,
        license: String,
        tags: String,
    ) -> Self {
        Self {
            id: Cow::Owned(id),
            label: Cow::Owned(label),
            provider: Cow::Borrowed(provider),
            url: Cow::Owned(url),
            thumbnail_url: thumbnail_url.map(Cow::Owned),
            kind,
            license: Cow::Owned(license),
            tags: Cow::Owned(tags),
        }
    }

    fn thumbnail_or_url(&self) -> &str {
        self.thumbnail_url
            .as_ref()
            .map(|url| url.as_ref())
            .unwrap_or_else(|| self.url.as_ref())
    }
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

#[derive(Clone, Copy, Default)]
struct MediaKindCounts {
    images: usize,
    videos: usize,
    audio: usize,
}

impl MediaKindCounts {
    fn from_local_assets(assets: &[MediaAsset]) -> Self {
        let mut counts = Self::default();
        for asset in assets {
            counts.add(asset.payload.kind);
        }
        counts
    }

    fn from_remote_assets(assets: &[RemoteMediaAsset]) -> Self {
        let mut counts = Self::default();
        for asset in assets {
            counts.add(asset.kind);
        }
        counts
    }

    fn from_panel(panel: &MediaPanel) -> Self {
        let mut counts = panel.local_kind_counts;
        counts.add_counts(static_remote_kind_counts());
        counts.add_counts(panel.remote_kind_counts);
        counts
    }

    fn add(&mut self, kind: DraggedMediaKind) {
        match kind {
            DraggedMediaKind::Image => self.images += 1,
            DraggedMediaKind::Video => self.videos += 1,
            DraggedMediaKind::Audio => self.audio += 1,
        }
    }

    fn add_counts(&mut self, counts: Self) {
        self.images += counts.images;
        self.videos += counts.videos;
        self.audio += counts.audio;
    }

    fn count(&self, filter: MediaKindFilter) -> usize {
        match filter {
            MediaKindFilter::All => self.images + self.videos + self.audio,
            MediaKindFilter::Images => self.images,
            MediaKindFilter::Videos => self.videos,
            MediaKindFilter::Audio => self.audio,
        }
    }
}

fn static_remote_kind_counts() -> MediaKindCounts {
    static COUNTS: OnceLock<MediaKindCounts> = OnceLock::new();
    *COUNTS.get_or_init(|| MediaKindCounts::from_remote_assets(remote_media_assets()))
}

pub struct MediaPanel {
    workspace: WeakEntity<Workspace>,
    filter_editor: Entity<Editor>,
    http_client: Arc<dyn HttpClient>,
    media_roots: Vec<PathBuf>,
    assets: Vec<MediaAsset>,
    local_kind_counts: MediaKindCounts,
    remote_assets: Vec<RemoteMediaAsset>,
    remote_kind_counts: MediaKindCounts,
    remote_cache: HashMap<SharedString, RemoteMediaCacheEntry>,
    remote_cache_order: VecDeque<SharedString>,
    remote_warning: Option<SharedString>,
    kind_filter: MediaKindFilter,
    kind_scroll_handle: ScrollHandle,
    loading: bool,
    remote_loading: bool,
    index_loaded: bool,
    remote_signature: Option<SharedString>,
    remote_generation: u64,
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
        let http_client = cx.http_client();
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
                        panel.invalidate_remote_media();
                        cx.notify();
                    }
                },
            );

            Self {
                workspace: workspace_handle,
                filter_editor,
                http_client,
                media_roots,
                assets: Vec::with_capacity(MAX_MEDIA_RESULTS),
                local_kind_counts: MediaKindCounts::default(),
                remote_assets: Vec::with_capacity(MAX_REMOTE_MEDIA_RESULTS),
                remote_kind_counts: MediaKindCounts::default(),
                remote_cache: HashMap::with_capacity(MAX_REMOTE_MEDIA_CACHE_ENTRIES),
                remote_cache_order: VecDeque::with_capacity(MAX_REMOTE_MEDIA_CACHE_ENTRIES),
                remote_warning: None,
                kind_filter: MediaKindFilter::Images,
                kind_scroll_handle: ScrollHandle::new(),
                loading: false,
                remote_loading: false,
                index_loaded: false,
                remote_signature: None,
                remote_generation: 0,
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

    fn invalidate_remote_media(&mut self) {
        self.remote_generation = self.remote_generation.wrapping_add(1);
        self.remote_loading = false;
        self.remote_signature = None;
        self.remote_assets.clear();
        self.remote_kind_counts = MediaKindCounts::default();
        self.remote_warning = None;
    }

    fn refresh_remote_media(&mut self, cx: &mut Context<Self>) {
        let raw_query = self.raw_query(cx);
        let query = remote_media_query(&raw_query, self.kind_filter);
        let signature = media_remote_signature(self.kind_filter.label(), &query);
        self.remote_cache.remove(&signature);
        self.remote_cache_order
            .retain(|entry| entry.as_ref() != signature.as_ref());
        self.invalidate_remote_media();
        self.status = Some("Refreshing remote media".into());
        cx.notify();
    }

    fn ensure_remote_media_loaded(&mut self, raw_query: &str, cx: &mut Context<Self>) {
        let query = remote_media_query(raw_query, self.kind_filter);
        let signature = media_remote_signature(self.kind_filter.label(), &query);

        if self.remote_loading || self.remote_signature.as_deref() == Some(signature.as_ref()) {
            return;
        }

        if let Some((remote_kind_counts, warning)) = {
            if let Some(remote_entry) = self.remote_cache.get(&signature) {
                let remote_kind_counts = MediaKindCounts::from_remote_assets(&remote_entry.assets);
                self.remote_assets.clone_from(&remote_entry.assets);
                Some((remote_kind_counts, remote_entry.warning.clone()))
            } else {
                None
            }
        } {
            self.remote_kind_counts = remote_kind_counts;
            self.remote_signature = Some(signature.clone());
            self.remote_warning = warning;
            self.touch_remote_cache_entry(&signature);
            self.status = None;
            return;
        }

        self.remote_loading = true;
        self.remote_signature = Some(signature.clone());
        self.status = Some("Fetching remote media".into());

        let http_client = self.http_client.clone();
        let kind_filter = self.kind_filter;
        let generation = self.remote_generation;
        cx.spawn(async move |panel, cx| {
            cx.background_executor()
                .timer(REMOTE_MEDIA_FETCH_DEBOUNCE)
                .await;
            let should_fetch = panel
                .update(cx, |panel, _| {
                    panel.remote_generation == generation
                        && panel.remote_signature.as_deref() == Some(signature.as_ref())
                })
                .unwrap_or(false);
            if !should_fetch {
                return;
            }

            let result = fetch_remote_media_assets(
                http_client,
                query,
                kind_filter,
                cx.background_executor().clone(),
            )
            .await;
            panel
                .update(cx, |panel, cx| {
                    if panel.remote_generation == generation
                        && panel.remote_signature.as_deref() == Some(signature.as_ref())
                    {
                        panel.remote_loading = false;
                        match result {
                            Ok(result) => {
                                let remote_kind_counts =
                                    MediaKindCounts::from_remote_assets(&result.assets);
                                panel.remote_assets.clone_from(&result.assets);
                                panel.remote_warning = result.warning.clone();
                                panel.cache_remote_assets(signature.clone(), result);
                                panel.remote_kind_counts = remote_kind_counts;
                                panel.status = None;
                            }
                            Err(error) => {
                                panel.remote_assets.clear();
                                panel.remote_kind_counts = MediaKindCounts::default();
                                panel.remote_warning = None;
                                panel.status = Some(format!("Remote media: {error:#}").into());
                            }
                        }
                        cx.notify();
                    }
                })
                .ok();
        })
        .detach();
    }

    fn cache_remote_assets(&mut self, signature: SharedString, result: RemoteMediaFetchResult) {
        self.remote_cache.insert(
            signature.clone(),
            RemoteMediaCacheEntry {
                assets: result.assets,
                warning: result.warning,
            },
        );
        self.touch_remote_cache_entry(&signature);

        while self.remote_cache_order.len() > MAX_REMOTE_MEDIA_CACHE_ENTRIES {
            let Some(oldest_signature) = self.remote_cache_order.pop_front() else {
                break;
            };
            self.remote_cache.remove(&oldest_signature);
        }
    }

    fn touch_remote_cache_entry(&mut self, signature: &SharedString) {
        self.remote_cache_order
            .retain(|entry| entry.as_ref() != signature.as_ref());
        self.remote_cache_order.push_back(signature.clone());
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
                    let local_kind_counts = MediaKindCounts::from_local_assets(&assets);
                    panel.assets = assets;
                    panel.local_kind_counts = local_kind_counts;
                    panel.loading = false;
                    panel.index_loaded = true;
                    panel.status = Some(media_indexed_status(panel.assets.len()));
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn raw_query(&self, cx: &App) -> String {
        self.filter_editor.read(cx).text(cx).trim().to_string()
    }

    fn matching_assets(&self, query_terms: &[&str], limit: usize) -> (Vec<MediaAsset>, usize) {
        let kind_filter = self.kind_filter;
        if query_terms.is_empty() {
            let total_count = self.local_kind_counts.count(kind_filter);
            let mut visible_assets = Vec::with_capacity(limit.min(total_count));
            for asset in &self.assets {
                if !kind_filter.matches(asset.payload.kind) {
                    continue;
                }

                if visible_assets.len() >= limit {
                    break;
                }
                visible_assets.push(asset.clone());
            }
            return (visible_assets, total_count);
        }

        let mut visible_assets = Vec::with_capacity(limit);
        let mut match_count = 0;

        for asset in &self.assets {
            if !kind_filter.matches(asset.payload.kind) {
                continue;
            }

            if !query_terms.is_empty()
                && !media_search_matches(asset.search_text.as_ref(), &query_terms)
            {
                continue;
            }

            match_count += 1;
            if visible_assets.len() < limit {
                visible_assets.push(asset.clone());
            }
        }

        (visible_assets, match_count)
    }

    fn matching_remote_assets(
        &self,
        query_terms: &[&str],
        limit: usize,
    ) -> (Vec<RemoteMediaAsset>, usize) {
        let kind_filter = self.kind_filter;
        let static_assets = remote_media_assets();
        let candidate_count = static_assets.len() + self.remote_assets.len();
        let mut visible_assets = Vec::with_capacity(limit.min(candidate_count));
        let mut match_count = 0;
        let mut seen_urls = HashSet::with_capacity(candidate_count);

        for asset in static_assets {
            if !kind_filter.matches(asset.kind) {
                continue;
            }

            if !seen_urls.insert(asset.url.as_ref()) {
                continue;
            }

            if !query_terms.is_empty() && !remote_media_search_matches(asset, query_terms) {
                continue;
            }

            match_count += 1;
            if visible_assets.len() < limit {
                visible_assets.push(asset.clone());
            }
        }

        for asset in &self.remote_assets {
            if !kind_filter.matches(asset.kind) {
                continue;
            }

            if !seen_urls.insert(asset.url.as_ref()) {
                continue;
            }

            match_count += 1;
            if visible_assets.len() < limit {
                visible_assets.push(asset.clone());
            }
        }

        (visible_assets, match_count)
    }

    fn render_kind_filter_button(
        &self,
        filter: MediaKindFilter,
        count: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.kind_filter == filter;
        let label = media_count_label(filter.label(), count);
        let button_id = media_element_id("media-kind-filter-", filter.label());
        div().flex_none().child(
            Button::new(button_id, label)
                .style(ButtonStyle::Subtle)
                .size(ButtonSize::Compact)
                .toggle_state(selected)
                .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                .on_click(cx.listener(move |panel, _, _, cx| {
                    panel.kind_filter = filter;
                    panel.status = None;
                    panel.invalidate_remote_media();
                    cx.notify();
                })),
        )
    }

    fn render_kind_filters(
        &self,
        counts: &MediaKindCounts,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
                            .child(self.render_kind_filter_button(
                                MediaKindFilter::All,
                                counts.count(MediaKindFilter::All),
                                cx,
                            ))
                            .child(self.render_kind_filter_button(
                                MediaKindFilter::Images,
                                counts.count(MediaKindFilter::Images),
                                cx,
                            ))
                            .child(self.render_kind_filter_button(
                                MediaKindFilter::Videos,
                                counts.count(MediaKindFilter::Videos),
                                cx,
                            ))
                            .child(self.render_kind_filter_button(
                                MediaKindFilter::Audio,
                                counts.count(MediaKindFilter::Audio),
                                cx,
                            )),
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

        self.status = Some(media_status_label("Previewing ", &label));
        cx.notify();
    }

    fn copy_media_source(
        &mut self,
        source: String,
        label: impl AsRef<str>,
        cx: &mut Context<Self>,
    ) {
        cx.write_to_clipboard(ClipboardItem::new_string(source));
        self.status = Some(media_status_label("Copied ", label.as_ref()));
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
        let relative_display = payload.relative_display.clone();
        let thumbnail = media_thumbnail(kind, payload.path.as_path(), cx);
        let preview_payload = payload.clone();
        let copy_path = payload.path.clone();
        let copy_label = label.clone();
        let row_id = media_element_id("media-panel-row-", relative_display.as_ref());
        let preview_id = media_element_id("media-panel-preview-", relative_display.as_ref());
        let copy_id = media_element_id("media-panel-copy-path-", relative_display.as_ref());

        h_flex()
            .id(row_id)
            .gap_2()
            .items_center()
            .p_2()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().element_background)
            .cursor_pointer()
            .hover(|style| style.bg(cx.theme().colors().element_hover))
            .tooltip(Tooltip::text(relative_display.clone()))
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
            .child(thumbnail)
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
                Button::new(preview_id, "Preview")
                    .style(ButtonStyle::Subtle)
                    .size(ButtonSize::Compact)
                    .on_click(cx.listener(move |panel, _, window, cx| {
                        panel.preview_media_asset(preview_payload.clone(), window, cx);
                    })),
            )
            .child(
                Button::new(copy_id, "Copy")
                    .style(ButtonStyle::Subtle)
                    .size(ButtonSize::Compact)
                    .on_click(cx.listener(move |panel, _, _, cx| {
                        let copy_path = copy_path.to_string_lossy().into_owned();
                        panel.copy_media_source(copy_path, copy_label.clone(), cx);
                    })),
            )
    }

    fn render_remote_asset_row(
        &self,
        asset: RemoteMediaAsset,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let kind = asset.kind;
        let thumbnail = remote_media_thumbnail(&asset, cx);
        let url = asset.url.into_owned();
        let label = asset.label.into_owned();
        let provider = asset.provider.into_owned();
        let license = asset.license.into_owned();
        let id = asset.id.into_owned();
        let row_id = media_element_id("media-panel-remote-row-", id.as_str());
        let preview_id = media_element_id("media-panel-preview-remote-", id.as_str());
        let insert_id = media_element_id("media-panel-insert-remote-", id.as_str());
        let attribution = media_attribution_label(provider.as_str(), license.as_str());

        h_flex()
            .id(row_id)
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
            .child(thumbnail)
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(Label::new(label.clone()).size(LabelSize::Small).truncate())
                    .child(
                        Label::new(attribution)
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
                Button::new(preview_id, "Preview")
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
                Button::new(insert_id, "Insert URL")
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
        let description = remote_browser_description(provider_count, self.kind_filter.label());
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

    fn render_remote_warning_row(
        &self,
        warning: SharedString,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .id("media-panel-remote-warning-row")
            .gap_2()
            .items_center()
            .p_2()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().element_background)
            .tooltip(Tooltip::text(warning.clone()))
            .child(
                Icon::new(IconName::Warning)
                    .size(IconSize::Small)
                    .color(Color::Warning),
            )
            .child(
                Label::new(warning)
                    .size(LabelSize::XSmall)
                    .color(Color::Warning)
                    .truncate(),
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
        let raw_query = self.raw_query(cx);
        let normalized_query = lowercase_text(raw_query.trim());
        let mut query_terms_storage;
        let query_terms: &[&str] = if normalized_query.is_empty() {
            &[]
        } else {
            let query_term_count = normalized_query.split_whitespace().count();
            query_terms_storage = Vec::with_capacity(query_term_count);
            query_terms_storage.extend(normalized_query.split_whitespace());
            query_terms_storage.as_slice()
        };
        self.ensure_remote_media_loaded(raw_query.as_str(), cx);
        let (remote_assets, total_remote_matches) =
            self.matching_remote_assets(query_terms, MAX_MEDIA_RESULTS);
        let (assets, total_asset_matches) = self.matching_assets(
            query_terms,
            MAX_MEDIA_RESULTS.saturating_sub(remote_assets.len()),
        );
        let url_insert = self
            .render_url_insert(cx)
            .map(|element| element.into_any_element());
        let shown_count =
            total_asset_matches + total_remote_matches + usize::from(url_insert.is_some());
        let kind_counts = MediaKindCounts::from_panel(self);
        let total_count = kind_counts.count(self.kind_filter);
        let provider_count = remote_provider_count(self.kind_filter);
        let remote_warning = self.remote_warning.clone();
        let mut asset_rows = Vec::with_capacity(
            usize::from(url_insert.is_some())
                + usize::from(remote_warning.is_some())
                + remote_assets.len()
                + assets.len()
                + usize::from(provider_count > 0),
        );
        if let Some(url_insert) = url_insert {
            asset_rows.push(url_insert);
        }
        if let Some(warning) = remote_warning {
            asset_rows.push(
                self.render_remote_warning_row(warning, cx)
                    .into_any_element(),
            );
        }
        asset_rows.extend(
            remote_assets
                .into_iter()
                .map(|asset| self.render_remote_asset_row(asset, cx).into_any_element()),
        );
        asset_rows.extend(
            assets
                .into_iter()
                .map(|asset| self.render_asset_row(asset, cx).into_any_element()),
        );
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
            } else if self.remote_loading {
                "fetching".into()
            } else {
                media_fraction_label(shown_count, total_count)
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
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .child(
                                        IconButton::new(
                                            "media-panel-refresh-remote",
                                            IconName::RotateCw,
                                        )
                                        .shape(ui::IconButtonShape::Square)
                                        .icon_size(IconSize::Small)
                                        .tooltip(Tooltip::text("Refresh remote media"))
                                        .on_click(
                                            cx.listener(|panel, _, _, cx| {
                                                panel.refresh_remote_media(cx);
                                            }),
                                        ),
                                    )
                                    .child(
                                        Label::new(count_label)
                                            .size(LabelSize::XSmall)
                                            .color(Color::Muted)
                                            .truncate(),
                                    ),
                            ),
                    )
                    .child(self.filter_editor.clone()),
            )
            .child(self.render_kind_filters(&kind_counts, cx))
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
    let mut roots = Vec::with_capacity(8);
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
    let asset_capacity = roots.len().saturating_mul(32).min(MAX_MEDIA_RESULTS);
    let mut assets = Vec::with_capacity(asset_capacity);
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
            let search_text = local_media_search_text(
                payload.label.as_ref(),
                payload.relative_display.as_ref(),
                media_kind_label(kind),
            );
            assets.push(MediaAsset {
                payload,
                search_text,
            });
        }

        if assets.len() >= MAX_MEDIA_RESULTS {
            break;
        }
    }
}

fn has_ignored_media_segment(path: &Path) -> bool {
    path.components().any(|component| {
        let segment = component.as_os_str().to_string_lossy();
        IGNORED_MEDIA_SEGMENTS
            .iter()
            .any(|ignored| segment.eq_ignore_ascii_case(ignored))
    })
}

fn local_media_search_text(label: &str, relative_display: &str, kind_label: &str) -> SharedString {
    let mut text =
        String::with_capacity(label.len() + relative_display.len() + kind_label.len() + 2);
    push_lowercase(&mut text, label);
    text.push(' ');
    push_lowercase(&mut text, relative_display);
    text.push(' ');
    text.push_str(kind_label);
    text.into()
}

fn lowercase_text(value: &str) -> String {
    let mut text = String::with_capacity(value.len());
    push_lowercase(&mut text, value);
    text
}

fn push_lowercase(buffer: &mut String, value: &str) {
    for ch in value.chars() {
        for lower in ch.to_lowercase() {
            buffer.push(lower);
        }
    }
}

fn media_kind_for_path(path: &Path) -> Option<DraggedMediaKind> {
    media_kind_for_extension(path.extension()?.to_str()?)
}

fn media_kind_for_extension(extension: &str) -> Option<DraggedMediaKind> {
    if matches_ascii_ignore_case(extension, IMAGE_MEDIA_EXTENSIONS) {
        Some(DraggedMediaKind::Image)
    } else if matches_ascii_ignore_case(extension, VIDEO_MEDIA_EXTENSIONS) {
        Some(DraggedMediaKind::Video)
    } else if matches_ascii_ignore_case(extension, AUDIO_MEDIA_EXTENSIONS) {
        Some(DraggedMediaKind::Audio)
    } else {
        None
    }
}

fn matches_ascii_ignore_case(value: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

fn remote_browser_description(provider_count: usize, kind_label: &str) -> SharedString {
    let mut text = String::with_capacity(32 + 6 + kind_label.len());
    let _ = write!(text, "{provider_count} no-key remote sources for ");
    push_lowercase(&mut text, kind_label);
    text.into()
}

fn remote_media_provider_warning(errors: &[String], provider_count: usize) -> Option<SharedString> {
    if errors.is_empty() {
        return None;
    }

    let healthy_count = provider_count.saturating_sub(errors.len());
    let shown_error_count = errors.len().min(3);
    let mut text = String::with_capacity(
        96 + errors
            .iter()
            .take(shown_error_count)
            .map(String::len)
            .sum::<usize>(),
    );
    let _ = write!(
        text,
        "{} of {provider_count} remote providers skipped",
        errors.len()
    );
    if healthy_count > 0 {
        let _ = write!(text, "; {healthy_count} returned results");
    }
    text.push_str(": ");
    for (index, error) in errors.iter().take(shown_error_count).enumerate() {
        if index > 0 {
            text.push_str("; ");
        }
        text.push_str(error);
    }
    if errors.len() > shown_error_count {
        let _ = write!(text, "; +{} more", errors.len() - shown_error_count);
    }
    Some(text.into())
}

struct MediaUrlCandidate {
    url: String,
    kind: DraggedMediaKind,
    label: String,
}

#[derive(Deserialize)]
struct OpenverseResponse {
    results: Vec<OpenverseItem>,
}

#[derive(Deserialize)]
struct OpenverseItem {
    id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    thumbnail: Option<String>,
    license: Option<String>,
    creator: Option<String>,
    foreign_landing_url: Option<String>,
}

#[derive(Deserialize)]
struct WikimediaResponse {
    query: Option<WikimediaQuery>,
}

#[derive(Deserialize)]
struct WikimediaQuery {
    pages: HashMap<String, WikimediaPage>,
}

#[derive(Deserialize)]
struct WikimediaPage {
    title: Option<String>,
    imageinfo: Option<Vec<WikimediaImageInfo>>,
}

#[derive(Deserialize)]
struct WikimediaImageInfo {
    url: Option<String>,
    thumburl: Option<String>,
    mime: Option<String>,
}

#[derive(Deserialize)]
struct NasaResponse {
    collection: NasaCollection,
}

#[derive(Deserialize)]
struct NasaCollection {
    items: Vec<NasaItem>,
}

#[derive(Deserialize)]
struct NasaItem {
    href: Option<String>,
    data: Vec<NasaData>,
    links: Option<Vec<NasaLink>>,
}

#[derive(Deserialize)]
struct NasaData {
    title: Option<String>,
    description: Option<String>,
    media_type: Option<String>,
    nasa_id: Option<String>,
}

#[derive(Deserialize)]
struct NasaLink {
    href: Option<String>,
}

#[derive(Deserialize)]
struct LibraryOfCongressResponse {
    results: Vec<LibraryOfCongressItem>,
}

#[derive(Deserialize)]
struct LibraryOfCongressItem {
    title: Option<String>,
    url: Option<String>,
    #[serde(default)]
    image_url: Vec<String>,
}

#[derive(Deserialize)]
struct ArtInstituteResponse {
    data: Vec<ArtInstituteItem>,
}

#[derive(Deserialize)]
struct ArtInstituteItem {
    id: u64,
    title: Option<String>,
    image_id: Option<String>,
    artist_display: Option<String>,
}

#[derive(Deserialize)]
struct ClevelandArtResponse {
    data: Vec<ClevelandArtItem>,
}

#[derive(Deserialize)]
struct ClevelandArtItem {
    id: u64,
    title: Option<String>,
    images: Option<ClevelandArtImages>,
    creators: Option<Vec<ClevelandArtCreator>>,
    department: Option<String>,
    collection: Option<String>,
}

#[derive(Deserialize)]
struct ClevelandArtImages {
    web: Option<ClevelandArtImage>,
    print: Option<ClevelandArtImage>,
}

#[derive(Deserialize)]
struct ClevelandArtImage {
    url: Option<String>,
}

#[derive(Deserialize)]
struct ClevelandArtCreator {
    description: Option<String>,
}

#[derive(Deserialize)]
struct MetMuseumSearchResponse {
    #[serde(rename = "objectIDs")]
    object_ids: Option<Vec<u64>>,
}

#[derive(Deserialize)]
struct MetMuseumObjectResponse {
    #[serde(rename = "objectID")]
    object_id: u64,
    title: Option<String>,
    #[serde(rename = "primaryImageSmall")]
    primary_image_small: Option<String>,
    #[serde(rename = "artistDisplayName")]
    artist_display_name: Option<String>,
    #[serde(rename = "isPublicDomain")]
    is_public_domain: bool,
}

#[derive(Deserialize)]
struct InternetArchiveSearchResponse {
    response: InternetArchiveSearchDocs,
}

#[derive(Deserialize)]
struct InternetArchiveSearchDocs {
    docs: Vec<InternetArchiveSearchDoc>,
}

#[derive(Deserialize)]
struct InternetArchiveSearchDoc {
    identifier: String,
    title: Option<String>,
    mediatype: Option<String>,
}

#[derive(Deserialize)]
struct InternetArchiveMetadataResponse {
    #[serde(default)]
    files: Vec<InternetArchiveFile>,
}

#[derive(Deserialize)]
struct InternetArchiveFile {
    name: String,
}

fn remote_media_query(query: &str, filter: MediaKindFilter) -> String {
    let query = query.trim();
    if !query.is_empty() {
        return query.to_string();
    }

    match filter {
        MediaKindFilter::All | MediaKindFilter::Images => "interface mockups".to_string(),
        MediaKindFilter::Videos => "motion background".to_string(),
        MediaKindFilter::Audio => "ambient music".to_string(),
    }
}

fn focused_remote_limit(
    filter: MediaKindFilter,
    broad_limit: usize,
    focused_limit: usize,
) -> usize {
    if filter == MediaKindFilter::All {
        broad_limit
    } else {
        focused_limit
    }
}

async fn fetch_remote_media_assets(
    http_client: Arc<dyn HttpClient>,
    query: String,
    filter: MediaKindFilter,
    executor: BackgroundExecutor,
) -> anyhow::Result<RemoteMediaFetchResult> {
    let provider_count = remote_provider_count(filter);
    let mut fetches: Vec<RemoteMediaFetch> = Vec::with_capacity(provider_count);

    let mut assets = Vec::with_capacity(MAX_REMOTE_MEDIA_RESULTS);
    let mut errors = Vec::with_capacity(provider_count);
    let openverse_result_limit = focused_remote_limit(
        filter,
        OPENVERSE_RESULT_LIMIT,
        OPENVERSE_FOCUSED_RESULT_LIMIT,
    );
    let wikimedia_result_limit = focused_remote_limit(
        filter,
        WIKIMEDIA_RESULT_LIMIT,
        WIKIMEDIA_FOCUSED_RESULT_LIMIT,
    );
    let nasa_image_result_limit = focused_remote_limit(
        filter,
        NASA_IMAGE_RESULT_LIMIT,
        NASA_IMAGE_FOCUSED_RESULT_LIMIT,
    );
    let nasa_media_search_limit = focused_remote_limit(
        filter,
        NASA_MEDIA_SEARCH_LIMIT,
        NASA_MEDIA_FOCUSED_SEARCH_LIMIT,
    );
    let nasa_media_detail_limit = focused_remote_limit(
        filter,
        NASA_MEDIA_DETAIL_LIMIT,
        NASA_MEDIA_FOCUSED_DETAIL_LIMIT,
    );
    let met_museum_detail_limit = focused_remote_limit(
        filter,
        MET_MUSEUM_DETAIL_LIMIT,
        MET_MUSEUM_FOCUSED_DETAIL_LIMIT,
    );
    let library_of_congress_result_limit = focused_remote_limit(
        filter,
        LIBRARY_OF_CONGRESS_RESULT_LIMIT,
        LIBRARY_OF_CONGRESS_FOCUSED_RESULT_LIMIT,
    );
    let art_institute_result_limit = focused_remote_limit(
        filter,
        ART_INSTITUTE_RESULT_LIMIT,
        ART_INSTITUTE_FOCUSED_RESULT_LIMIT,
    );
    let cleveland_art_result_limit = focused_remote_limit(
        filter,
        CLEVELAND_ART_RESULT_LIMIT,
        CLEVELAND_ART_FOCUSED_RESULT_LIMIT,
    );
    let internet_archive_search_limit = focused_remote_limit(
        filter,
        INTERNET_ARCHIVE_SEARCH_LIMIT,
        INTERNET_ARCHIVE_FOCUSED_SEARCH_LIMIT,
    );
    let internet_archive_detail_limit = focused_remote_limit(
        filter,
        INTERNET_ARCHIVE_DETAIL_LIMIT,
        INTERNET_ARCHIVE_FOCUSED_DETAIL_LIMIT,
    );

    if matches!(filter, MediaKindFilter::All | MediaKindFilter::Images) {
        fetches.push(remote_media_fetch(
            "Openverse images",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_openverse_media(
                        http_client,
                        &query,
                        DraggedMediaKind::Image,
                        openverse_result_limit,
                    )
                    .await
                }
            },
            executor.clone(),
        ));
        fetches.push(remote_media_fetch(
            "Wikimedia",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_wikimedia_media(http_client, &query, filter, wikimedia_result_limit).await
                }
            },
            executor.clone(),
        ));
        fetches.push(remote_media_fetch(
            "NASA",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move { fetch_nasa_images(http_client, &query, nasa_image_result_limit).await }
            },
            executor.clone(),
        ));
        fetches.push(remote_media_fetch(
            "Library of Congress",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_library_of_congress_images(
                        http_client,
                        &query,
                        library_of_congress_result_limit,
                    )
                    .await
                }
            },
            executor.clone(),
        ));
        fetches.push(remote_media_fetch(
            "Art Institute",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_art_institute_images(http_client, &query, art_institute_result_limit)
                        .await
                }
            },
            executor.clone(),
        ));
        fetches.push(remote_media_fetch(
            "Cleveland Museum",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_cleveland_art_images(http_client, &query, cleveland_art_result_limit)
                        .await
                }
            },
            executor.clone(),
        ));
        fetches.push(remote_media_fetch(
            "The Met",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_met_museum_images(http_client, &query, met_museum_detail_limit).await
                }
            },
            executor.clone(),
        ));
        fetches.push(remote_media_fetch(
            "Internet Archive images",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_internet_archive_media(
                        http_client,
                        &query,
                        DraggedMediaKind::Image,
                        internet_archive_search_limit,
                        internet_archive_detail_limit,
                    )
                    .await
                }
            },
            executor.clone(),
        ));
    }

    if matches!(filter, MediaKindFilter::All | MediaKindFilter::Audio) {
        fetches.push(remote_media_fetch(
            "Openverse audio",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_openverse_media(
                        http_client,
                        &query,
                        DraggedMediaKind::Audio,
                        openverse_result_limit,
                    )
                    .await
                }
            },
            executor.clone(),
        ));
        fetches.push(remote_media_fetch(
            "NASA audio",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_nasa_media(
                        http_client,
                        &query,
                        DraggedMediaKind::Audio,
                        nasa_media_search_limit,
                        nasa_media_detail_limit,
                    )
                    .await
                }
            },
            executor.clone(),
        ));
        if matches!(filter, MediaKindFilter::Audio) {
            fetches.push(remote_media_fetch(
                "Wikimedia audio",
                {
                    let http_client = http_client.clone();
                    let query = query.clone();
                    async move {
                        fetch_wikimedia_media(
                            http_client,
                            &query,
                            MediaKindFilter::Audio,
                            wikimedia_result_limit,
                        )
                        .await
                    }
                },
                executor.clone(),
            ));
        }
        fetches.push(remote_media_fetch(
            "Internet Archive audio",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_internet_archive_media(
                        http_client,
                        &query,
                        DraggedMediaKind::Audio,
                        internet_archive_search_limit,
                        internet_archive_detail_limit,
                    )
                    .await
                }
            },
            executor.clone(),
        ));
    }

    if matches!(filter, MediaKindFilter::All | MediaKindFilter::Videos) {
        fetches.push(remote_media_fetch(
            "NASA videos",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_nasa_media(
                        http_client,
                        &query,
                        DraggedMediaKind::Video,
                        nasa_media_search_limit,
                        nasa_media_detail_limit,
                    )
                    .await
                }
            },
            executor.clone(),
        ));
        fetches.push(remote_media_fetch(
            "Wikimedia video",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_wikimedia_media(
                        http_client,
                        &query,
                        MediaKindFilter::Videos,
                        wikimedia_result_limit,
                    )
                    .await
                }
            },
            executor.clone(),
        ));
        fetches.push(remote_media_fetch(
            "Internet Archive videos",
            {
                let http_client = http_client.clone();
                let query = query.clone();
                async move {
                    fetch_internet_archive_media(
                        http_client,
                        &query,
                        DraggedMediaKind::Video,
                        internet_archive_search_limit,
                        internet_archive_detail_limit,
                    )
                    .await
                }
            },
            executor.clone(),
        ));
    }

    for (provider, result) in futures::future::join_all(fetches).await {
        match result {
            Ok(items) => assets.extend(items),
            Err(error) => errors.push(format!("{provider}: {error:#}")),
        }
    }

    dedupe_remote_assets(&mut assets);
    assets.truncate(MAX_REMOTE_MEDIA_RESULTS);

    if assets.is_empty() && !errors.is_empty() {
        anyhow::bail!(errors.join("; "));
    }

    let warning = remote_media_provider_warning(&errors, provider_count);
    Ok(RemoteMediaFetchResult { assets, warning })
}

fn remote_media_fetch<F>(
    provider: &'static str,
    fetch: F,
    executor: BackgroundExecutor,
) -> RemoteMediaFetch
where
    F: Future<Output = anyhow::Result<Vec<RemoteMediaAsset>>> + 'static,
{
    Box::pin(async move {
        let fetch = Box::pin(fetch);
        let timeout = Box::pin(executor.timer(REMOTE_MEDIA_PROVIDER_TIMEOUT));
        match futures::future::select(fetch, timeout).await {
            futures::future::Either::Left((result, _)) => (provider, result),
            futures::future::Either::Right((_, _)) => (
                provider,
                Err(anyhow::anyhow!(
                    "timed out after {}s",
                    REMOTE_MEDIA_PROVIDER_TIMEOUT.as_secs()
                )),
            ),
        }
    })
}

async fn fetch_openverse_media(
    http_client: Arc<dyn HttpClient>,
    query: &str,
    kind: DraggedMediaKind,
    result_limit: usize,
) -> anyhow::Result<Vec<RemoteMediaAsset>> {
    let endpoint = match kind {
        DraggedMediaKind::Image => "images",
        DraggedMediaKind::Audio => "audio",
        DraggedMediaKind::Video => return Ok(Vec::new()),
    };
    let url = format!(
        "https://api.openverse.org/v1/{endpoint}/?q={}&page_size={result_limit}",
        encode_query(query)
    );
    let response: OpenverseResponse = fetch_json(http_client, &url).await?;
    let provider = match kind {
        DraggedMediaKind::Image => "Openverse Images",
        DraggedMediaKind::Audio => "Openverse Audio",
        DraggedMediaKind::Video => "Openverse",
    };

    let mut assets = Vec::with_capacity(response.results.len());
    for item in response.results {
        let thumbnail_url = item.thumbnail.clone();
        let Some(url) = item.url.or(item.thumbnail) else {
            continue;
        };
        let label = clean_remote_label(item.title.as_deref().unwrap_or("Openverse media"));
        let identifier = item
            .id
            .clone()
            .or_else(|| item.foreign_landing_url.clone())
            .unwrap_or_else(|| url.clone());
        let license = item
            .license
            .unwrap_or_else(|| "Creative Commons".to_string());
        let tag_capacity = item.creator.as_deref().map(str::len).unwrap_or(0)
            + item
                .foreign_landing_url
                .as_deref()
                .map(str::len)
                .unwrap_or(0)
            + 1;
        let mut tags = String::with_capacity(tag_capacity);
        let mut is_first_tag = true;
        for tag in [item.creator, item.foreign_landing_url]
            .into_iter()
            .flatten()
        {
            push_remote_tag(&mut tags, &mut is_first_tag, tag.as_str());
        }
        assets.push(RemoteMediaAsset::owned_with_thumbnail(
            remote_asset_id(provider, &identifier),
            label,
            provider,
            url,
            thumbnail_url,
            kind,
            license,
            tags,
        ));
    }
    Ok(assets)
}

async fn fetch_wikimedia_media(
    http_client: Arc<dyn HttpClient>,
    query: &str,
    filter: MediaKindFilter,
    result_limit: usize,
) -> anyhow::Result<Vec<RemoteMediaAsset>> {
    let url = format!(
        "https://commons.wikimedia.org/w/api.php?action=query&generator=search&gsrnamespace=6&gsrsearch={}&gsrlimit={result_limit}&prop=imageinfo&iiprop=url%7Cmime&iiurlwidth=360&format=json&origin=*",
        encode_query(query)
    );
    let response: WikimediaResponse = fetch_json(http_client, &url).await?;
    let pages = response.query.map(|query| query.pages).unwrap_or_default();

    let mut assets = Vec::with_capacity(pages.len());
    for page in pages.into_values() {
        let Some(info) = page
            .imageinfo
            .and_then(|imageinfo| imageinfo.into_iter().next())
        else {
            continue;
        };
        let Some(kind) = info.mime.as_deref().and_then(media_kind_for_mime) else {
            continue;
        };
        if !filter.matches(kind) {
            continue;
        }
        let thumbnail_url = info.thumburl.clone();
        let Some(url) = info.url.or(info.thumburl) else {
            continue;
        };
        let title = page
            .title
            .as_deref()
            .map(strip_wikimedia_file_prefix)
            .unwrap_or("Wikimedia media");
        assets.push(RemoteMediaAsset::owned_with_thumbnail(
            remote_asset_id("Wikimedia", &url),
            clean_remote_label(title),
            "Wikimedia Commons",
            url,
            thumbnail_url,
            kind,
            "open license".to_string(),
            query.to_string(),
        ));
    }
    Ok(assets)
}

async fn fetch_nasa_images(
    http_client: Arc<dyn HttpClient>,
    query: &str,
    result_limit: usize,
) -> anyhow::Result<Vec<RemoteMediaAsset>> {
    let url = format!(
        "https://images-api.nasa.gov/search?q={}&media_type=image&page_size={result_limit}",
        encode_query(query)
    );
    let response: NasaResponse = fetch_json(http_client, &url).await?;

    let mut assets = Vec::with_capacity(response.collection.items.len());
    for item in response.collection.items {
        let Some(data) = item.data.into_iter().next() else {
            continue;
        };
        if data.media_type.as_deref() != Some("image") {
            continue;
        }
        let Some(url) = item
            .links
            .and_then(|links| links.into_iter().find_map(|link| link.href))
        else {
            continue;
        };
        let label = clean_remote_label(data.title.as_deref().unwrap_or("NASA image"));
        let identifier = data.nasa_id.clone().unwrap_or_else(|| url.clone());
        let tags = data.description.unwrap_or_default();
        assets.push(RemoteMediaAsset::owned(
            remote_asset_id("NASA", &identifier),
            label,
            "NASA",
            url,
            DraggedMediaKind::Image,
            "public domain".to_string(),
            tags,
        ));
    }
    Ok(assets)
}

async fn fetch_nasa_media(
    http_client: Arc<dyn HttpClient>,
    query: &str,
    kind: DraggedMediaKind,
    search_limit: usize,
    detail_limit: usize,
) -> anyhow::Result<Vec<RemoteMediaAsset>> {
    let media_type = match kind {
        DraggedMediaKind::Video => "video",
        DraggedMediaKind::Audio => "audio",
        DraggedMediaKind::Image => {
            return fetch_nasa_images(http_client, query, NASA_IMAGE_RESULT_LIMIT).await;
        }
    };
    let url = format!(
        "https://images-api.nasa.gov/search?q={}&media_type={media_type}&page_size={search_limit}",
        encode_query(query)
    );
    let response: NasaResponse = fetch_json(http_client.clone(), &url).await?;
    let items = response.collection.items;
    let request_count = items.len().min(detail_limit);
    let mut detail_requests = Vec::with_capacity(request_count);
    detail_requests.extend(
        items
            .into_iter()
            .take(detail_limit)
            .map(|item| fetch_nasa_media_asset(http_client.clone(), item, kind)),
    );

    let results = futures::future::join_all(detail_requests).await;
    let mut assets = Vec::with_capacity(results.len());
    for result in results {
        if let Ok(asset) = result {
            assets.push(asset);
        }
    }
    Ok(assets)
}

async fn fetch_nasa_media_asset(
    http_client: Arc<dyn HttpClient>,
    item: NasaItem,
    kind: DraggedMediaKind,
) -> anyhow::Result<RemoteMediaAsset> {
    let data = item
        .data
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing NASA media metadata"))?;
    let media_type = match kind {
        DraggedMediaKind::Video => "video",
        DraggedMediaKind::Audio => "audio",
        DraggedMediaKind::Image => "image",
    };
    if data.media_type.as_deref() != Some(media_type) {
        anyhow::bail!("unexpected NASA media type");
    }

    let collection_url = item
        .href
        .ok_or_else(|| anyhow::anyhow!("missing NASA media collection"))?;
    let thumbnail_url = item
        .links
        .and_then(|links| links.into_iter().find_map(|link| link.href));
    let files: Vec<String> = fetch_json(http_client, &collection_url).await?;
    let media_url = nasa_media_file_url(files.into_iter(), kind)
        .ok_or_else(|| anyhow::anyhow!("missing direct NASA media file"))?;
    let label = clean_remote_label(data.title.as_deref().unwrap_or("NASA media"));
    let identifier = data.nasa_id.unwrap_or_else(|| media_url.clone());
    let tags = data.description.unwrap_or_default();

    Ok(RemoteMediaAsset::owned_with_thumbnail(
        remote_asset_id("NASA", &identifier),
        label,
        "NASA",
        media_url,
        thumbnail_url,
        kind,
        "public domain".to_string(),
        tags,
    ))
}

fn nasa_media_file_url(
    files: impl Iterator<Item = String>,
    kind: DraggedMediaKind,
) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for url in files {
        let lower = url.to_lowercase();
        let matches_kind = match kind {
            DraggedMediaKind::Video => lower.ends_with(".mp4"),
            DraggedMediaKind::Audio => lower.ends_with(".mp3") || lower.ends_with(".m4a"),
            DraggedMediaKind::Image => false,
        };
        if !matches_kind {
            continue;
        }

        let url = url.replacen(
            "http://images-assets.nasa.gov",
            "https://images-assets.nasa.gov",
            1,
        );
        let rank = nasa_media_file_rank(&url, kind);
        let should_replace = match best.as_ref() {
            Some((best_rank, _)) => rank < *best_rank,
            None => true,
        };
        if should_replace {
            best = Some((rank, url));
        }
    }

    best.map(|(_, url)| url)
}

fn nasa_media_file_rank(url: &str, kind: DraggedMediaKind) -> usize {
    let lower = url.to_lowercase();
    match kind {
        DraggedMediaKind::Video => {
            if lower.contains("~preview.mp4") {
                0
            } else if lower.contains("~small.mp4") || lower.contains("~mobile.mp4") {
                1
            } else if lower.contains("~medium.mp4") {
                2
            } else if lower.ends_with(".mp4") && !lower.contains("~orig") {
                3
            } else {
                4
            }
        }
        DraggedMediaKind::Audio => {
            if lower.contains("~128k.mp3") {
                0
            } else if lower.contains("~64k.mp3") {
                1
            } else if lower.ends_with(".mp3") && !lower.contains("~orig") {
                2
            } else if lower.ends_with(".m4a") {
                3
            } else {
                4
            }
        }
        DraggedMediaKind::Image => 0,
    }
}

async fn fetch_library_of_congress_images(
    http_client: Arc<dyn HttpClient>,
    query: &str,
    result_limit: usize,
) -> anyhow::Result<Vec<RemoteMediaAsset>> {
    let url = format!(
        "https://www.loc.gov/photos/?fo=json&c={result_limit}&q={}",
        encode_query(query)
    );
    let response: LibraryOfCongressResponse = fetch_json(http_client, &url).await?;

    let mut assets = Vec::with_capacity(response.results.len());
    for item in response.results {
        let Some(url) = item
            .image_url
            .into_iter()
            .rev()
            .find(|url| url.starts_with("https://"))
        else {
            continue;
        };
        let label = clean_remote_label(item.title.as_deref().unwrap_or("Library image"));
        let source = item.url.unwrap_or_default();
        assets.push(RemoteMediaAsset::owned(
            remote_asset_id("LOC", &url),
            label,
            "Library of Congress",
            url,
            DraggedMediaKind::Image,
            "public domain / rights vary".to_string(),
            source,
        ));
    }
    Ok(assets)
}

async fn fetch_art_institute_images(
    http_client: Arc<dyn HttpClient>,
    query: &str,
    result_limit: usize,
) -> anyhow::Result<Vec<RemoteMediaAsset>> {
    let url = format!(
        "https://api.artic.edu/api/v1/artworks/search?q={}&query%5Bterm%5D%5Bis_public_domain%5D=true&limit={result_limit}&fields=id,title,image_id,artist_display",
        encode_query(query)
    );
    let response: ArtInstituteResponse = fetch_json(http_client, &url).await?;

    let mut assets = Vec::with_capacity(response.data.len());
    for item in response.data {
        let Some(image_id) = item.image_id else {
            continue;
        };
        let url = format!("https://www.artic.edu/iiif/2/{image_id}/full/843,/0/default.jpg");
        let label = clean_remote_label(item.title.as_deref().unwrap_or("Art Institute image"));
        assets.push(RemoteMediaAsset::owned(
            remote_asset_id("ArtInstitute", &item.id.to_string()),
            label,
            "Art Institute of Chicago",
            url,
            DraggedMediaKind::Image,
            "public domain".to_string(),
            item.artist_display.unwrap_or_default(),
        ));
    }
    Ok(assets)
}

async fn fetch_cleveland_art_images(
    http_client: Arc<dyn HttpClient>,
    query: &str,
    result_limit: usize,
) -> anyhow::Result<Vec<RemoteMediaAsset>> {
    let url = format!(
        "https://openaccess-api.clevelandart.org/api/artworks/?q={}&cc0=1&has_image=1&limit={result_limit}",
        encode_query(query)
    );
    let response: ClevelandArtResponse = fetch_json(http_client, &url).await?;

    let mut assets = Vec::with_capacity(response.data.len());
    for item in response.data {
        let Some(images) = item.images else {
            continue;
        };
        let Some(image_url) = images
            .web
            .as_ref()
            .and_then(|image| image.url.clone())
            .or_else(|| images.print.as_ref().and_then(|image| image.url.clone()))
        else {
            continue;
        };
        let tag_capacity = item
            .creators
            .as_ref()
            .map(|creators| {
                creators
                    .iter()
                    .filter_map(|creator| creator.description.as_deref())
                    .map(str::len)
                    .sum::<usize>()
            })
            .unwrap_or(0)
            + item.department.as_deref().map(str::len).unwrap_or(0)
            + item.collection.as_deref().map(str::len).unwrap_or(0)
            + 2;
        let mut tags = String::with_capacity(tag_capacity);
        let mut is_first_tag = true;
        for tag in item
            .creators
            .unwrap_or_default()
            .into_iter()
            .filter_map(|creator| creator.description)
            .chain(item.department)
            .chain(item.collection)
        {
            push_remote_tag(&mut tags, &mut is_first_tag, tag.as_str());
        }

        assets.push(RemoteMediaAsset::owned_with_thumbnail(
            remote_asset_id("ClevelandMuseum", &item.id.to_string()),
            clean_remote_label(item.title.as_deref().unwrap_or("Cleveland artwork")),
            "Cleveland Museum of Art",
            image_url.clone(),
            Some(image_url),
            DraggedMediaKind::Image,
            "CC0".to_string(),
            tags,
        ));
    }
    Ok(assets)
}

async fn fetch_met_museum_images(
    http_client: Arc<dyn HttpClient>,
    query: &str,
    detail_limit: usize,
) -> anyhow::Result<Vec<RemoteMediaAsset>> {
    let url = format!(
        "https://collectionapi.metmuseum.org/public/collection/v1/search?hasImages=true&q={}",
        encode_query(query)
    );
    let response: MetMuseumSearchResponse = fetch_json(http_client.clone(), &url).await?;
    let object_ids = response.object_ids.unwrap_or_default();
    let request_count = object_ids.len().min(detail_limit);
    let mut detail_requests = Vec::with_capacity(request_count);
    detail_requests.extend(
        object_ids
            .into_iter()
            .take(detail_limit)
            .map(|object_id| fetch_met_museum_object(http_client.clone(), object_id)),
    );

    let results = futures::future::join_all(detail_requests).await;
    let mut assets = Vec::with_capacity(results.len());
    for result in results {
        let Ok(item) = result else {
            continue;
        };
        if !item.is_public_domain {
            continue;
        }
        let Some(image_url) = item.primary_image_small else {
            continue;
        };
        if image_url.trim().is_empty() {
            continue;
        }

        assets.push(RemoteMediaAsset::owned(
            remote_asset_id("Met", &item.object_id.to_string()),
            clean_remote_label(item.title.as_deref().unwrap_or("The Met image")),
            "The Met",
            image_url,
            DraggedMediaKind::Image,
            "public domain / open access".to_string(),
            item.artist_display_name.unwrap_or_default(),
        ));
    }
    Ok(assets)
}

async fn fetch_met_museum_object(
    http_client: Arc<dyn HttpClient>,
    object_id: u64,
) -> anyhow::Result<MetMuseumObjectResponse> {
    let url =
        format!("https://collectionapi.metmuseum.org/public/collection/v1/objects/{object_id}");
    fetch_json(http_client, &url).await
}

async fn fetch_internet_archive_media(
    http_client: Arc<dyn HttpClient>,
    query: &str,
    kind: DraggedMediaKind,
    search_limit: usize,
    detail_limit: usize,
) -> anyhow::Result<Vec<RemoteMediaAsset>> {
    let mediatype = match kind {
        DraggedMediaKind::Image => "image",
        DraggedMediaKind::Video => "movies",
        DraggedMediaKind::Audio => "audio",
    };
    let search = format!("{query} AND mediatype:{mediatype}");
    let url = format!(
        "https://archive.org/advancedsearch.php?q={}&fl[]=identifier&fl[]=title&fl[]=mediatype&rows={search_limit}&page=1&output=json",
        encode_query(&search)
    );
    let response: InternetArchiveSearchResponse = fetch_json(http_client.clone(), &url).await?;
    let docs = response.response.docs;
    let request_count = docs.len().min(detail_limit);
    let mut detail_requests = Vec::with_capacity(request_count);
    detail_requests.extend(
        docs.into_iter()
            .take(detail_limit)
            .map(|item| fetch_internet_archive_asset(http_client.clone(), item, kind)),
    );

    let results = futures::future::join_all(detail_requests).await;
    let mut assets = Vec::with_capacity(results.len());
    for result in results {
        if let Ok(asset) = result {
            assets.push(asset);
        }
    }
    Ok(assets)
}

async fn fetch_internet_archive_asset(
    http_client: Arc<dyn HttpClient>,
    item: InternetArchiveSearchDoc,
    kind: DraggedMediaKind,
) -> anyhow::Result<RemoteMediaAsset> {
    let metadata_url = format!(
        "https://archive.org/metadata/{}",
        encode_path_component(&item.identifier)
    );
    let metadata: InternetArchiveMetadataResponse = fetch_json(http_client, &metadata_url).await?;
    let file_name = best_internet_archive_file(metadata.files.into_iter(), kind)
        .ok_or_else(|| anyhow::anyhow!("missing direct Internet Archive media file"))?;
    let media_url = format!(
        "https://archive.org/download/{}/{}",
        encode_path_component(&item.identifier),
        encode_path(&file_name)
    );
    let thumbnail_url = Some(format!(
        "https://archive.org/services/img/{}",
        encode_path_component(&item.identifier)
    ));
    let label = clean_remote_label(item.title.as_deref().unwrap_or(&item.identifier));
    let tags = item.mediatype.unwrap_or_default();

    Ok(RemoteMediaAsset::owned_with_thumbnail(
        remote_asset_id("InternetArchive", &item.identifier),
        label,
        "Internet Archive",
        media_url,
        thumbnail_url,
        kind,
        "public domain / Creative Commons / rights vary".to_string(),
        tags,
    ))
}

fn best_internet_archive_file(
    files: impl Iterator<Item = InternetArchiveFile>,
    kind: DraggedMediaKind,
) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for file in files {
        let lower = file.name.to_lowercase();
        if lower.contains("_meta.")
            || lower.contains("_files.")
            || lower.ends_with(".torrent")
            || lower.ends_with(".xml")
            || lower.ends_with(".sqlite")
        {
            continue;
        }

        let Some(extension) = Path::new(&lower)
            .extension()
            .and_then(|extension| extension.to_str())
        else {
            continue;
        };
        if media_kind_for_extension(extension) != Some(kind) {
            continue;
        }

        let rank = internet_archive_file_rank(&file.name, kind);
        let should_replace = match best.as_ref() {
            Some((best_rank, _)) => rank < *best_rank,
            None => true,
        };
        if should_replace {
            best = Some((rank, file.name));
        }
    }

    best.map(|(_, name)| name)
}

fn internet_archive_file_rank(name: &str, kind: DraggedMediaKind) -> usize {
    let lower = name.to_lowercase();
    let generated_penalty = if lower.contains("_thumb")
        || lower.contains("thumbs/")
        || lower.contains("_spectrogram")
        || lower.contains("_itemimage")
    {
        20
    } else {
        0
    };

    generated_penalty
        + match kind {
            DraggedMediaKind::Image => {
                if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
                    0
                } else if lower.ends_with(".png") {
                    1
                } else if lower.ends_with(".webp") {
                    2
                } else if lower.ends_with(".gif") {
                    3
                } else {
                    4
                }
            }
            DraggedMediaKind::Video => {
                if lower.ends_with(".mp4") && !lower.contains("ia.mp4") {
                    0
                } else if lower.ends_with(".mp4") {
                    1
                } else if lower.ends_with(".webm") {
                    2
                } else if lower.ends_with(".m4v") {
                    3
                } else {
                    4
                }
            }
            DraggedMediaKind::Audio => {
                if lower.ends_with(".mp3") {
                    0
                } else if lower.ends_with(".m4a") {
                    1
                } else if lower.ends_with(".ogg") {
                    2
                } else if lower.ends_with(".flac") {
                    3
                } else if lower.ends_with(".wav") {
                    4
                } else {
                    5
                }
            }
        }
}

async fn fetch_json<T: for<'de> Deserialize<'de>>(
    http_client: Arc<dyn HttpClient>,
    url: &str,
) -> anyhow::Result<T> {
    let mut response = http_client.get(url, AsyncBody::default(), true).await?;
    if !response.status().is_success() {
        anyhow::bail!("HTTP {}", response.status());
    }

    let body_capacity = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .map(|length| length.min(MAX_REMOTE_JSON_BODY_RESERVE))
        .unwrap_or(0);
    let mut body = String::with_capacity(body_capacity);
    response.body_mut().read_to_string(&mut body).await?;
    Ok(serde_json::from_str(&body)?)
}

fn dedupe_remote_assets(assets: &mut Vec<RemoteMediaAsset>) {
    let mut seen = HashSet::<SharedString>::with_capacity(assets.len());
    assets.retain(|asset| seen.insert(asset.url.clone()));
}

fn media_kind_for_mime(mime: &str) -> Option<DraggedMediaKind> {
    if mime.starts_with("image/") {
        Some(DraggedMediaKind::Image)
    } else if mime.starts_with("video/") {
        Some(DraggedMediaKind::Video)
    } else if mime.starts_with("audio/") {
        Some(DraggedMediaKind::Audio)
    } else {
        None
    }
}

fn strip_wikimedia_file_prefix(title: &str) -> &str {
    title
        .strip_prefix("File:")
        .or_else(|| title.strip_prefix("Image:"))
        .unwrap_or(title)
}

fn clean_remote_label(label: &str) -> String {
    let label = label
        .rsplit_once('.')
        .filter(|(_, extension)| extension.len() <= 5)
        .map_or(label, |(stem, _)| stem)
        .replace(['_', '-'], " ");
    let mut normalized = String::with_capacity(label.len());
    for word in label.split_whitespace() {
        if !normalized.is_empty() {
            normalized.push(' ');
        }
        normalized.push_str(word);
    }
    if normalized.is_empty() {
        "remote media".to_string()
    } else {
        normalized
    }
}

fn push_remote_tag(tags: &mut String, is_first_tag: &mut bool, tag: &str) {
    if !*is_first_tag {
        tags.push(' ');
    }
    *is_first_tag = false;
    tags.push_str(tag);
}

fn remote_asset_id(provider: &str, value: &str) -> String {
    let id = format!("{provider}-{value}");
    let mut id = preview_file_stem(&id);
    if let Some((index, _)) = id.char_indices().nth(96) {
        id.truncate(index);
    }
    id
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
    let mut label = String::with_capacity(stem.len());
    let mut needs_separator = false;
    for character in stem.chars() {
        if character == '_' || character == '-' || character.is_whitespace() {
            needs_separator = !label.is_empty();
        } else {
            if needs_separator {
                label.push(' ');
            }
            label.push(character);
            needs_separator = false;
        }
    }
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
    let mut provider_links = String::with_capacity(remote_provider_count(filter) * 192);
    for source in free_media_sources() {
        let Some(search_url) = source.search_url(query, filter) else {
            continue;
        };
        let _ = write!(
            provider_links,
            r#"<a class="provider" data-provider="{}" title="{}" href="{}">{}</a>"#,
            escape_attr(source.id),
            escape_attr(source.description),
            escape_attr(&search_url),
            escape_html(source.name)
        );
    }

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
      if (type === "video") return [];
      const endpoint = type === "audio" ? "audio" : "images";
      const response = await fetch(`https://api.openverse.org/v1/${{endpoint}}/?q=${{encodeURIComponent(query)}}&page_size=60`);
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
      const media = type === "video" ? "video" : type === "audio" ? "audio" : "image";
      const response = await fetch(`https://images-api.nasa.gov/search?q=${{encodeURIComponent(query)}}&media_type=${{media}}`);
      const json = await response.json();
      const items = ((json.collection || {{}}).items || []).slice(0, 30);
      if (media === "image") return items.map((item) => {{
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
      const resolved = await Promise.all(items.map(async (item) => {{
        const data = (item.data || [])[0] || {{}};
        const files = await fetch(item.href).then((response) => response.json()).catch(() => []);
        const url = bestNasaMediaUrl(files, media);
        const link = (item.links || [])[0] || {{}};
        return url ? {{
          provider: "NASA",
          title: data.title,
          url,
          thumbnail: link.href,
          source: item.href,
          license: "NASA",
          kind: media,
        }} : null;
      }}));
      return resolved.filter(Boolean);
    }}

    function bestNasaMediaUrl(files, media) {{
      const candidates = (files || [])
        .filter((url) => media === "video" ? /\.mp4$/i.test(url) : /\.(mp3|m4a)$/i.test(url))
        .map((url) => url.replace("http://images-assets.nasa.gov", "https://images-assets.nasa.gov"));
      const rank = (url) => {{
        const lower = url.toLowerCase();
        if (media === "video") {{
          if (lower.includes("~preview.mp4")) return 0;
          if (lower.includes("~small.mp4") || lower.includes("~mobile.mp4")) return 1;
          if (lower.includes("~medium.mp4")) return 2;
          if (!lower.includes("~orig")) return 3;
          return 4;
        }}
        if (lower.includes("~128k.mp3")) return 0;
        if (lower.includes("~64k.mp3")) return 1;
        if (lower.endsWith(".mp3") && !lower.includes("~orig")) return 2;
        if (lower.endsWith(".m4a")) return 3;
        return 4;
      }};
      return candidates.sort((left, right) => rank(left) - rank(right))[0];
    }}

    async function searchWikimedia(query, type) {{
      if (type !== "image") return [];
      const params = new URLSearchParams({{
        action: "query",
        generator: "search",
        gsrsearch: query,
        gsrnamespace: "6",
        gsrlimit: "48",
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

    async function searchArtInstitute(query, type) {{
      if (type !== "image") return [];
      const response = await fetch(`https://api.artic.edu/api/v1/artworks/search?q=${{encodeURIComponent(query)}}&query%5Bterm%5D%5Bis_public_domain%5D=true&limit=48&fields=id,title,image_id,artist_display`);
      const json = await response.json();
      return (json.data || []).filter((item) => item.image_id).map((item) => ({{
        provider: "Art Institute of Chicago",
        title: item.title,
        url: `https://www.artic.edu/iiif/2/${{item.image_id}}/full/843,/0/default.jpg`,
        thumbnail: `https://www.artic.edu/iiif/2/${{item.image_id}}/full/400,/0/default.jpg`,
        source: `https://www.artic.edu/artworks/${{item.id}}`,
        license: "public domain",
        kind: "image",
      }}));
    }}

    async function searchClevelandArt(query, type) {{
      if (type !== "image") return [];
      const response = await fetch(`https://openaccess-api.clevelandart.org/api/artworks/?q=${{encodeURIComponent(query)}}&cc0=1&has_image=1&limit=48`);
      const json = await response.json();
      return (json.data || []).map((item) => {{
        const images = item.images || {{}};
        const image = (images.web || images.print || {{}}).url;
        return image ? {{
          provider: "Cleveland Museum of Art",
          title: item.title,
          url: image,
          thumbnail: image,
          source: item.url || `https://www.clevelandart.org/art/${{item.id}}`,
          license: "CC0",
          kind: "image",
        }} : null;
      }}).filter(Boolean);
    }}

    async function searchMet(query, type) {{
      if (type !== "image") return [];
      const search = await fetch(`https://collectionapi.metmuseum.org/public/collection/v1/search?hasImages=true&q=${{encodeURIComponent(query)}}`).then((response) => response.json());
      const ids = (search.objectIDs || []).slice(0, 24);
      const objects = await Promise.all(ids.map((id) =>
        fetch(`https://collectionapi.metmuseum.org/public/collection/v1/objects/${{id}}`).then((response) => response.json()).catch(() => null)
      ));
      return objects.filter((item) => item && item.isPublicDomain && item.primaryImageSmall).map((item) => ({{
        provider: "The Met",
        title: item.title,
        url: item.primaryImageSmall,
        thumbnail: item.primaryImageSmall,
        source: item.objectURL || item.primaryImageSmall,
        license: "public domain / open access",
        kind: "image",
      }}));
    }}

    async function searchInternetArchive(query, type) {{
      const mediaType = type === "video" ? "movies" : type === "audio" ? "audio" : "image";
      const params = new URLSearchParams();
      params.set("q", `${{query}} AND mediatype:${{mediaType}}`);
      params.append("fl[]", "identifier");
      params.append("fl[]", "title");
      params.append("fl[]", "mediatype");
      params.set("rows", "24");
      params.set("page", "1");
      params.set("output", "json");
      const search = await fetch(`https://archive.org/advancedsearch.php?${{params}}`).then((response) => response.json());
      const docs = (((search || {{}}).response || {{}}).docs || []).slice(0, 24);
      const resolved = await Promise.all(docs.map(async (doc) => {{
        const identifier = doc.identifier;
        if (!identifier) return null;
        const metadata = await fetch(`https://archive.org/metadata/${{encodeURIComponent(identifier)}}`).then((response) => response.json()).catch(() => null);
        const file = bestInternetArchiveFile((metadata || {{}}).files || [], type);
        if (!file) return null;
        const encodedFile = file.split("/").map(encodeURIComponent).join("/");
        return {{
          provider: "Internet Archive",
          title: doc.title || identifier,
          url: `https://archive.org/download/${{encodeURIComponent(identifier)}}/${{encodedFile}}`,
          thumbnail: `https://archive.org/services/img/${{encodeURIComponent(identifier)}}`,
          source: `https://archive.org/details/${{encodeURIComponent(identifier)}}`,
          license: "public domain / Creative Commons / rights vary",
          kind: type,
        }};
      }}));
      return resolved.filter(Boolean);
    }}

    function bestInternetArchiveFile(files, type) {{
      const extensions = type === "video"
        ? [".mp4", ".webm", ".m4v"]
        : type === "audio"
          ? [".mp3", ".m4a", ".ogg", ".flac", ".wav"]
          : [".jpg", ".jpeg", ".png", ".webp", ".gif"];
      return (files || [])
        .map((file) => file && file.name)
        .filter(Boolean)
        .filter((name) => {{
          const lower = name.toLowerCase();
          return extensions.some((extension) => lower.endsWith(extension))
            && !lower.includes("_meta.")
            && !lower.includes("_files.")
            && !lower.endsWith(".torrent")
            && !lower.endsWith(".xml");
        }})
        .sort((left, right) => internetArchiveFileRank(left, type) - internetArchiveFileRank(right, type))[0];
    }}

    function internetArchiveFileRank(name, type) {{
      const lower = name.toLowerCase();
      const generatedPenalty = lower.includes("_thumb")
        || lower.includes("thumbs/")
        || lower.includes("_spectrogram")
        || lower.includes("_itemimage")
        ? 20
        : 0;
      const rank = type === "video"
        ? lower.endsWith(".mp4") && !lower.includes("ia.mp4") ? 0 : lower.endsWith(".mp4") ? 1 : lower.endsWith(".webm") ? 2 : 3
        : type === "audio"
          ? lower.endsWith(".mp3") ? 0 : lower.endsWith(".m4a") ? 1 : lower.endsWith(".ogg") ? 2 : lower.endsWith(".flac") ? 3 : 4
          : lower.endsWith(".jpg") || lower.endsWith(".jpeg") ? 0 : lower.endsWith(".png") ? 1 : lower.endsWith(".webp") ? 2 : 3;
      return generatedPenalty + rank;
    }}

    async function runSearch() {{
      const query = queryInput.value.trim() || "creative workspace";
      const type = typeInput.value;
      status.textContent = "Searching Openverse, Wikimedia, NASA, Internet Archive, Art Institute, Cleveland Museum, and The Met...";
      grid.innerHTML = "";
      const settled = await Promise.allSettled([
        searchOpenverse(query, type),
        searchWikimedia(query, type),
        searchNasa(query, type),
        searchInternetArchive(query, type),
        searchArtInstitute(query, type),
        searchClevelandArt(query, type),
        searchMet(query, type),
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
    let mut stem = String::with_capacity(label.len());
    let mut needs_separator = false;
    for character in label.chars() {
        if character.is_ascii_alphanumeric() {
            if needs_separator && !stem.is_empty() {
                stem.push('-');
            }
            stem.push(character.to_ascii_lowercase());
            needs_separator = false;
        } else {
            needs_separator = !stem.is_empty();
        }
    }

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

fn media_element_id(prefix: &str, id: &str) -> String {
    let mut element_id = String::with_capacity(prefix.len() + id.len());
    element_id.push_str(prefix);
    element_id.push_str(id);
    element_id
}

fn media_count_label(label: &str, count: usize) -> String {
    let mut text = String::with_capacity(label.len() + 1 + 6);
    text.push_str(label);
    let _ = write!(text, " {count}");
    text
}

fn media_fraction_label(left: usize, right: usize) -> SharedString {
    let mut text = String::with_capacity(24);
    let _ = write!(text, "{left} / {right}");
    text.into()
}

fn media_indexed_status(count: usize) -> SharedString {
    let mut text = String::with_capacity("Indexed ".len() + 6 + " media files".len());
    let _ = write!(text, "Indexed {count} media files");
    text.into()
}

fn media_status_label(prefix: &str, value: &str) -> SharedString {
    let mut text = String::with_capacity(prefix.len() + value.len());
    text.push_str(prefix);
    text.push_str(value);
    text.into()
}

fn media_remote_signature(kind_label: &str, query: &str) -> SharedString {
    let mut signature = String::with_capacity(kind_label.len() + 1 + query.len());
    signature.push_str(kind_label);
    signature.push(':');
    signature.push_str(query);
    signature.into()
}

fn media_attribution_label(provider: &str, license: &str) -> String {
    let mut label = String::with_capacity(provider.len() + 3 + license.len());
    label.push_str(provider);
    label.push_str(" - ");
    label.push_str(license);
    label
}

fn media_thumbnail(
    kind: DraggedMediaKind,
    path: &Path,
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

fn remote_media_thumbnail(asset: &RemoteMediaAsset, cx: &mut Context<MediaPanel>) -> AnyElement {
    match asset.kind {
        DraggedMediaKind::Image => div()
            .w(px(64.))
            .h(px(48.))
            .rounded_sm()
            .overflow_hidden()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .child(
                img(asset.thumbnail_or_url())
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            )
            .into_any_element(),
        DraggedMediaKind::Video => {
            if let Some(thumbnail_url) = asset.thumbnail_url.as_ref() {
                remote_media_thumbnail_with_badge(
                    thumbnail_url.as_ref(),
                    IconName::PlayOutlined,
                    cx,
                )
            } else {
                div()
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
                    .into_any_element()
            }
        }
        DraggedMediaKind::Audio => {
            if let Some(thumbnail_url) = asset.thumbnail_url.as_ref() {
                remote_media_thumbnail_with_badge(thumbnail_url.as_ref(), IconName::AudioOn, cx)
            } else {
                div()
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
                    .into_any_element()
            }
        }
    }
}

fn remote_media_thumbnail_with_badge(
    thumbnail_url: &str,
    icon: IconName,
    cx: &mut Context<MediaPanel>,
) -> AnyElement {
    div()
        .relative()
        .w(px(64.))
        .h(px(48.))
        .rounded_sm()
        .overflow_hidden()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .child(img(thumbnail_url).size_full().object_fit(ObjectFit::Cover))
        .child(
            div()
                .absolute()
                .right(px(4.))
                .bottom(px(4.))
                .w(px(18.))
                .h(px(18.))
                .rounded_full()
                .border_1()
                .border_color(cx.theme().colors().border.opacity(0.6))
                .bg(cx
                    .theme()
                    .colors()
                    .elevated_surface_background
                    .opacity(0.88))
                .flex()
                .items_center()
                .justify_center()
                .child(Icon::new(icon).size(IconSize::XSmall).color(Color::Accent)),
        )
        .into_any_element()
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

fn media_search_matches(searchable: &str, query_terms: &[&str]) -> bool {
    query_terms.iter().all(|term| searchable.contains(term))
}

fn remote_media_search_matches(asset: &RemoteMediaAsset, query_terms: &[&str]) -> bool {
    query_terms.iter().all(|term| {
        contains_ascii_case_insensitive(asset.label.as_ref(), term)
            || contains_ascii_case_insensitive(asset.provider.as_ref(), term)
            || contains_ascii_case_insensitive(asset.license.as_ref(), term)
            || contains_ascii_case_insensitive(asset.tags.as_ref(), term)
            || media_kind_label(asset.kind).contains(term)
    })
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    let needle = needle.as_bytes();
    if needle.is_empty() {
        return true;
    }

    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

fn remote_media_assets() -> &'static [RemoteMediaAsset] {
    &[
        RemoteMediaAsset::borrowed(
            "wikimedia-fronalpstock",
            "Fronalpstock landscape",
            "Wikimedia Commons",
            "https://upload.wikimedia.org/wikipedia/commons/3/3f/Fronalpstock_big.jpg",
            DraggedMediaKind::Image,
            "CC BY-SA",
            "mountain landscape nature travel hero background",
        ),
        RemoteMediaAsset::borrowed(
            "wikimedia-van-gogh-starry-night",
            "The Starry Night",
            "Wikimedia Commons",
            "https://upload.wikimedia.org/wikipedia/commons/e/ea/Van_Gogh_-_Starry_Night_-_Google_Art_Project.jpg",
            DraggedMediaKind::Image,
            "public domain",
            "painting art night museum impressionism texture",
        ),
        RemoteMediaAsset::borrowed(
            "wikimedia-mona-lisa",
            "Mona Lisa",
            "Wikimedia Commons",
            "https://upload.wikimedia.org/wikipedia/commons/6/6a/Mona_Lisa.jpg",
            DraggedMediaKind::Image,
            "public domain",
            "portrait art museum renaissance people",
        ),
        RemoteMediaAsset::borrowed(
            "wikimedia-hubble-deep-field",
            "Hubble Deep Field",
            "NASA / Wikimedia",
            "https://upload.wikimedia.org/wikipedia/commons/5/5f/HubbleDeepField.800px.jpg",
            DraggedMediaKind::Image,
            "public domain",
            "space galaxy nasa stars astronomy background",
        ),
        RemoteMediaAsset::borrowed(
            "wikimedia-great-wave",
            "The Great Wave",
            "Wikimedia Commons",
            "https://upload.wikimedia.org/wikipedia/commons/a/a5/Tsunami_by_hokusai_19th_century.jpg",
            DraggedMediaKind::Image,
            "public domain",
            "wave ocean japan illustration art print",
        ),
        RemoteMediaAsset::borrowed(
            "wikimedia-blue-marble",
            "Blue Marble",
            "NASA",
            "https://upload.wikimedia.org/wikipedia/commons/9/97/The_Earth_seen_from_Apollo_17.jpg",
            DraggedMediaKind::Image,
            "public domain",
            "earth space planet nasa globe science",
        ),
        RemoteMediaAsset::borrowed(
            "nasa-mars-pathfinder",
            "Mars Pathfinder panorama",
            "NASA",
            "https://images-assets.nasa.gov/image/PIA00452/PIA00452~orig.jpg",
            DraggedMediaKind::Image,
            "public domain",
            "mars nasa space rover science panorama planet",
        ),
        RemoteMediaAsset::borrowed(
            "mdn-flower-video",
            "Flower video",
            "MDN",
            "https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4",
            DraggedMediaKind::Video,
            "CC0",
            "flower nature macro loop video motion",
        ),
        RemoteMediaAsset::borrowed(
            "mdn-flower-webm",
            "Flower video WebM",
            "MDN",
            "https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.webm",
            DraggedMediaKind::Video,
            "CC0",
            "flower nature macro webm loop motion",
        ),
        RemoteMediaAsset::borrowed(
            "wikimedia-big-buck-bunny",
            "Big Buck Bunny sample",
            "Wikimedia Commons",
            "https://upload.wikimedia.org/wikipedia/commons/transcoded/7/70/Big.Buck.Bunny.-.Opening.Screen.ogv/Big.Buck.Bunny.-.Opening.Screen.ogv.360p.webm",
            DraggedMediaKind::Video,
            "CC BY",
            "animation sample open movie video webm",
        ),
        RemoteMediaAsset::borrowed(
            "blender-big-buck-bunny-mp4",
            "Big Buck Bunny MP4",
            "Blender Open Movie",
            "https://download.blender.org/peach/bigbuckbunny_movies/BigBuckBunny_320x180.mp4",
            DraggedMediaKind::Video,
            "CC BY",
            "animation open movie blender video mp4 sample",
        ),
        RemoteMediaAsset::borrowed(
            "blender-sintel-trailer",
            "Sintel trailer",
            "Blender Open Movie",
            "https://download.blender.org/durian/trailer/sintel_trailer-480p.mp4",
            DraggedMediaKind::Video,
            "CC BY",
            "animation fantasy trailer blender video mp4 open movie",
        ),
        RemoteMediaAsset::borrowed(
            "wikimedia-example-audio",
            "Example audio",
            "Wikimedia Commons",
            "https://upload.wikimedia.org/wikipedia/commons/c/c8/Example.ogg",
            DraggedMediaKind::Audio,
            "public sample",
            "speech sample sound audio ogg",
        ),
        RemoteMediaAsset::borrowed(
            "mdn-t-rex-roar",
            "T-Rex roar",
            "MDN",
            "https://interactive-examples.mdn.mozilla.net/media/cc0-audio/t-rex-roar.mp3",
            DraggedMediaKind::Audio,
            "CC0",
            "sound effect roar mp3 sample",
        ),
        RemoteMediaAsset::borrowed(
            "wikimedia-bach-brandenburg",
            "Bach Brandenburg sample",
            "Wikimedia Commons",
            "https://upload.wikimedia.org/wikipedia/commons/4/45/Bach_-_Brandenburg_Concerto_No._3_-_1._Allegro.ogg",
            DraggedMediaKind::Audio,
            "public domain",
            "classical music bach orchestral sample",
        ),
        RemoteMediaAsset::borrowed(
            "soundhelix-song-1",
            "SoundHelix music sample",
            "SoundHelix",
            "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3",
            DraggedMediaKind::Audio,
            "royalty-free sample",
            "music mp3 sample soundtrack background audio",
        ),
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
            id: "internet-archive",
            name: "Internet Archive",
            description: "No-key public-domain and Creative Commons images, videos, and audio",
            homepage: "https://archive.org/",
            image_search: "https://archive.org/search?query={query}%20AND%20mediatype%3Aimage",
            video_search: Some(
                "https://archive.org/search?query={query}%20AND%20mediatype%3Amovies",
            ),
            audio_search: Some(
                "https://archive.org/search?query={query}%20AND%20mediatype%3Aaudio",
            ),
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
            id: "cleveland-art",
            name: "Cleveland Museum",
            description: "No-key CC0 artwork images from the Cleveland Museum of Art API",
            homepage: "https://www.clevelandart.org/open-access",
            image_search: "https://www.clevelandart.org/art/collection/search?search={query}&open_access=1",
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
    let query = query.trim();
    let mut encoded = String::with_capacity(query.len());
    for byte in query.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            _ => push_percent_encoded_byte(&mut encoded, byte),
        }
    }
    encoded
}

fn encode_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for (index, component) in path.split('/').enumerate() {
        if index > 0 {
            encoded.push('/');
        }
        encoded.push_str(&encode_path_component(component));
    }
    encoded
}

fn encode_path_component(component: &str) -> String {
    let mut encoded = String::with_capacity(component.len());
    for byte in component.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => push_percent_encoded_byte(&mut encoded, byte),
        }
    }
    encoded
}

fn push_percent_encoded_byte(encoded: &mut String, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    encoded.push('%');
    encoded.push(HEX[(byte >> 4) as usize] as char);
    encoded.push(HEX[(byte & 0x0f) as usize] as char);
}
