use editor::{Editor, EditorEvent};
use gpui::{
    AnyElement, App, AppContext as _, AsyncWindowContext, ClipboardItem, Context, Entity,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, ObjectFit, Pixels, Render,
    ScrollHandle, SharedString, StatefulInteractiveElement, Subscription, WeakEntity, Window,
    actions, div, img, point, px,
};
use memmap2::MmapOptions;
use rkyv::{
    Archive, Deserialize as RkyvDeserialize, Infallible, Serialize as RkyvSerialize, archived_root,
    ser::{Serializer, serializers::AllocSerializer},
};
use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fs::{self as std_fs, File},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use ui::{TintColor, Tooltip, prelude::*};
use url::Url;
use workspace::{
    DraggedShadcnAsset, DraggedShadcnKind, Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

mod registry_directory;

#[cfg(target_os = "windows")]
use web_preview::web_preview_view::WebPreviewView;

actions!(
    shadcn_ui_panel,
    [
        /// Toggles the UI panel.
        Toggle,
        /// Toggles focus on the UI panel.
        ToggleFocus,
    ]
);

const SHADCN_UI_PANEL_KEY: &str = "ShadcnUiPanel";
const MAX_SHADCN_ROWS: usize = 96;
const CATALOG_CACHE_FILE_NAME: &str = "catalog-v1.rkyv";
const STATIC_SHADCN_CATALOG_INDEX: &str = include_str!("shadcn_catalog_index.tsv");
static SHADCN_STATIC_CATALOG_CACHE: OnceLock<Vec<CatalogItem>> = OnceLock::new();
static SHADCN_CATALOG_CACHE: OnceLock<Vec<CatalogItem>> = OnceLock::new();
static SHADCN_PREVIEW_IMAGE_CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> =
    OnceLock::new();

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace.register_action(|workspace, _: &ToggleFocus, window, cx| {
            workspace.toggle_panel_focus::<ShadcnUiPanel>(window, cx);
        });
        workspace.register_action(|workspace, _: &Toggle, window, cx| {
            if !workspace.toggle_panel_focus::<ShadcnUiPanel>(window, cx) {
                workspace.close_panel::<ShadcnUiPanel>(window, cx);
            }
        });
    })
    .detach();
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CatalogSource {
    ShadcnComponent,
    ShadcnBlock,
    MagicUi,
    CommunityRegistry,
    TwentyFirst,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CatalogFilter {
    All,
    Components,
    Blocks,
    Magic,
    Registries,
}

impl CatalogFilter {
    fn matches(self, source: CatalogSource) -> bool {
        match self {
            Self::All => true,
            Self::Components => source == CatalogSource::ShadcnComponent,
            Self::Blocks => source == CatalogSource::ShadcnBlock,
            Self::Magic => source == CatalogSource::MagicUi,
            Self::Registries => {
                matches!(
                    source,
                    CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst
                )
            }
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Components => "UI",
            Self::Blocks => "Blocks",
            Self::Magic => "Magic",
            Self::Registries => "Registries",
        }
    }
}

#[derive(Clone, Copy, Default)]
struct CatalogFilterCounts {
    all: usize,
    components: usize,
    blocks: usize,
    magic: usize,
    registries: usize,
}

impl CatalogFilterCounts {
    fn from_items(items: &[CatalogItem]) -> Self {
        let mut counts = Self::default();
        for item in items {
            counts.all += 1;
            match item.source {
                CatalogSource::ShadcnComponent => counts.components += 1,
                CatalogSource::ShadcnBlock => counts.blocks += 1,
                CatalogSource::MagicUi => counts.magic += 1,
                CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst => {
                    counts.registries += 1
                }
            }
        }
        counts
    }

    fn count(&self, filter: CatalogFilter) -> usize {
        match filter {
            CatalogFilter::All => self.all,
            CatalogFilter::Components => self.components,
            CatalogFilter::Blocks => self.blocks,
            CatalogFilter::Magic => self.magic,
            CatalogFilter::Registries => self.registries,
        }
    }
}

#[derive(Clone)]
struct CatalogItem {
    id: SharedString,
    title: SharedString,
    description: SharedString,
    category: SharedString,
    source: CatalogSource,
    source_path: SharedString,
    target_file_name: SharedString,
    import_statement: SharedString,
    jsx: SharedString,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize)]
struct CachedCatalogItem {
    id: String,
    title: String,
    description: String,
    category: String,
    source: u8,
    source_path: String,
    target_file_name: String,
    import_statement: String,
    jsx: String,
}

impl CachedCatalogItem {
    fn from_catalog_item(item: &CatalogItem) -> Self {
        Self {
            id: item.id.to_string(),
            title: item.title.to_string(),
            description: item.description.to_string(),
            category: item.category.to_string(),
            source: catalog_source_to_u8(item.source),
            source_path: item.source_path.to_string(),
            target_file_name: item.target_file_name.to_string(),
            import_statement: item.import_statement.to_string(),
            jsx: item.jsx.to_string(),
        }
    }

    fn into_catalog_item(self) -> CatalogItem {
        CatalogItem {
            id: self.id.into(),
            title: self.title.into(),
            description: self.description.into(),
            category: self.category.into(),
            source: catalog_source_from_u8(self.source),
            source_path: self.source_path.into(),
            target_file_name: self.target_file_name.into(),
            import_statement: self.import_statement.into(),
            jsx: self.jsx.into(),
        }
    }
}

pub struct ShadcnUiPanel {
    workspace: WeakEntity<Workspace>,
    filter_editor: Entity<Editor>,
    items: Vec<CatalogItem>,
    filter_counts: CatalogFilterCounts,
    search_text_cache: RefCell<HashMap<String, SharedString>>,
    loading_catalog: bool,
    catalog_loaded: bool,
    source_filter: CatalogFilter,
    warming_preview_image_keys: HashSet<String>,
    filter_scroll_handle: ScrollHandle,
    status: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl ShadcnUiPanel {
    pub async fn load(
        workspace: WeakEntity<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> anyhow::Result<Entity<Self>> {
        workspace.update_in(&mut cx, |workspace, window, cx| {
            Self::new(workspace, window, cx)
        })
    }

    fn new(
        _workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Entity<Self> {
        let workspace_handle = cx.entity().downgrade();

        cx.new(|cx| {
            let filter_editor = cx.new(|cx| {
                let mut editor = Editor::single_line(window, cx);
                editor.set_placeholder_text("Search UI, blocks, Magic UI...", window, cx);
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
            let should_write_initial_cache = !shadcn_catalog_cache_path().is_file();
            let items = initial_shadcn_catalog();
            let filter_counts = CatalogFilterCounts::from_items(&items);
            if should_write_initial_cache {
                let cache_items = items.clone();
                cx.background_executor()
                    .spawn(async move {
                        let _ = write_shadcn_catalog_rkyv_cache(&cache_items);
                    })
                    .detach();
            }

            Self {
                workspace: workspace_handle,
                filter_editor,
                items,
                filter_counts,
                search_text_cache: RefCell::default(),
                loading_catalog: false,
                catalog_loaded: true,
                source_filter: CatalogFilter::All,
                warming_preview_image_keys: HashSet::with_capacity(MAX_SHADCN_ROWS),
                filter_scroll_handle: ScrollHandle::new(),
                status: None,
                _subscriptions: vec![filter_subscription],
            }
        })
    }

    fn ensure_catalog_loaded(&mut self, cx: &mut Context<Self>) {
        if self.catalog_loaded || self.loading_catalog {
            return;
        }

        self.load_catalog(cx);
    }

    fn load_catalog(&mut self, cx: &mut Context<Self>) {
        if self.loading_catalog {
            return;
        }

        self.loading_catalog = true;
        self.status = Some("Loading UI catalog...".into());
        let executor = cx.background_executor().clone();
        cx.spawn(async move |panel, cx| {
            let items = executor.spawn(async move { shadcn_catalog_cached() }).await;
            panel
                .update(cx, |panel, cx| {
                    panel.items = items;
                    panel.filter_counts = CatalogFilterCounts::from_items(&panel.items);
                    panel.search_text_cache.borrow_mut().clear();
                    panel.loading_catalog = false;
                    panel.catalog_loaded = true;
                    panel.status = Some(format!("Loaded {} UI entries", panel.items.len()).into());
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn query(&self, cx: &App) -> String {
        self.filter_editor.read(cx).text(cx).trim().to_lowercase()
    }

    fn matching_items(&self, query: &str, limit: usize) -> (Vec<CatalogItem>, usize) {
        let source_filter = self.source_filter;
        if query.is_empty() {
            let total_count = self.filter_counts.count(source_filter);
            let mut visible_items = Vec::with_capacity(limit.min(total_count));
            for item in &self.items {
                if !source_filter.matches(item.source) {
                    continue;
                }

                if visible_items.len() >= limit {
                    break;
                }
                visible_items.push(item.clone());
            }
            return (visible_items, total_count);
        }

        let query_terms = query.split_whitespace().collect::<Vec<_>>();
        let mut visible_items =
            Vec::with_capacity(limit.min(self.filter_counts.count(source_filter)));
        let mut match_count = 0;

        for item in &self.items {
            if !source_filter.matches(item.source) {
                continue;
            }

            if !query_terms.is_empty() && !self.catalog_item_matches(item, &query_terms) {
                continue;
            }

            match_count += 1;
            if visible_items.len() < limit {
                visible_items.push(item.clone());
            }
        }

        (visible_items, match_count)
    }

    fn catalog_item_matches(&self, item: &CatalogItem, query_terms: &[&str]) -> bool {
        if let Some(matches) = {
            let search_text_cache = self.search_text_cache.borrow();
            search_text_cache
                .get(item.id.as_ref())
                .map(|search_text| catalog_search_matches(search_text.as_ref(), query_terms))
        } {
            return matches;
        }

        let search_text: SharedString = format!(
            "{} {} {} {}",
            item.id.as_ref().to_lowercase(),
            item.title.as_ref().to_lowercase(),
            item.category.as_ref().to_lowercase(),
            item.description.as_ref().to_lowercase()
        )
        .into();
        self.search_text_cache
            .borrow_mut()
            .insert(item.id.to_string(), search_text.clone());
        catalog_search_matches(search_text.as_ref(), query_terms)
    }

    fn ensure_visible_preview_images_warmed(
        &mut self,
        items: &[CatalogItem],
        cached_preview_images: &HashMap<String, Option<String>>,
        cx: &mut Context<Self>,
    ) {
        let pending_items = items
            .iter()
            .filter(|item| !cached_preview_images.contains_key(item.id.as_ref()))
            .filter(|item| !self.warming_preview_image_keys.contains(item.id.as_ref()))
            .cloned()
            .collect::<Vec<_>>();

        if pending_items.is_empty() {
            return;
        }

        for item in &pending_items {
            self.warming_preview_image_keys.insert(item.id.to_string());
        }

        let executor = cx.background_executor().clone();
        cx.spawn(async move |panel, cx| {
            let images = executor
                .spawn(async move { warm_shadcn_preview_images(pending_items) })
                .await;
            panel
                .update(cx, |panel, cx| {
                    for (key, image_url) in images {
                        panel.warming_preview_image_keys.remove(&key);
                        insert_preview_image_cache(key, image_url);
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn render_filter_button(
        &self,
        filter: CatalogFilter,
        count: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.source_filter == filter;
        let label = format!("{} {}", filter.label(), count);
        div().flex_none().child(
            Button::new(format!("shadcn-filter-{}", filter.label()), label)
                .style(ButtonStyle::Subtle)
                .size(ButtonSize::Compact)
                .toggle_state(selected)
                .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                .on_click(cx.listener(move |panel, _, _, cx| {
                    panel.source_filter = filter;
                    panel.status = None;
                    cx.notify();
                })),
        )
    }

    fn render_filter_tabs(
        &self,
        counts: &CatalogFilterCounts,
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
                IconButton::new("ui-panel-filter-prev", IconName::ChevronLeft)
                    .shape(ui::IconButtonShape::Square)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Previous UI groups"))
                    .on_click(cx.listener(|panel, _, _, cx| {
                        panel.scroll_filter_tabs(-1.0, cx);
                    })),
            )
            .child(
                h_flex()
                    .id("ui-panel-filter-scroll")
                    .flex_1()
                    .h_full()
                    .overflow_x_scroll()
                    .overflow_y_hidden()
                    .track_scroll(&self.filter_scroll_handle)
                    .child(
                        h_flex()
                            .flex_none()
                            .gap_1()
                            .items_center()
                            .px_1()
                            .py_1()
                            .child(self.render_filter_button(
                                CatalogFilter::All,
                                counts.count(CatalogFilter::All),
                                cx,
                            ))
                            .child(self.render_filter_button(
                                CatalogFilter::Components,
                                counts.count(CatalogFilter::Components),
                                cx,
                            ))
                            .child(self.render_filter_button(
                                CatalogFilter::Blocks,
                                counts.count(CatalogFilter::Blocks),
                                cx,
                            ))
                            .child(self.render_filter_button(
                                CatalogFilter::Magic,
                                counts.count(CatalogFilter::Magic),
                                cx,
                            ))
                            .child(self.render_filter_button(
                                CatalogFilter::Registries,
                                counts.count(CatalogFilter::Registries),
                                cx,
                            )),
                    ),
            )
            .child(
                IconButton::new("ui-panel-filter-next", IconName::ChevronRight)
                    .shape(ui::IconButtonShape::Square)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Next UI groups"))
                    .on_click(cx.listener(|panel, _, _, cx| {
                        panel.scroll_filter_tabs(1.0, cx);
                    })),
            )
    }

    fn scroll_filter_tabs(&mut self, direction: f32, cx: &mut Context<Self>) {
        scroll_tab_handle(&self.filter_scroll_handle, direction);
        cx.notify();
    }

    fn insert_item(&mut self, item: CatalogItem, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(
            item.source,
            CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst
        ) {
            self.open_item_docs(item, cx);
            return;
        }

        let Some(workspace) = self.workspace.upgrade() else {
            self.status = Some("No active workspace".into());
            cx.notify();
            return;
        };
        let Some(editor) = workspace.read(cx).active_item_as::<Editor>(cx) else {
            self.status = Some("Open a React editor to insert UI".into());
            cx.notify();
            return;
        };
        let payload = self.payload_for_item(&item);
        if !item_source_available(&item, &payload) {
            self.status = Some(
                format!(
                    "Missing source or registry manifest: {}",
                    payload.source_path.to_string_lossy().replace('\\', "/")
                )
                .into(),
            );
            cx.notify();
            return;
        }

        let result = editor.update(cx, |editor, cx| {
            editor.focus_handle(cx).focus(window, cx);
            editor.insert_shadcn_asset(&payload, window, cx)
        });

        self.status = match result {
            Ok(message) => Some(message),
            Err(error) => Some(format!("{error:#}").into()),
        };
        cx.notify();
    }

    fn preview_item(&mut self, item: CatalogItem, window: &mut Window, cx: &mut Context<Self>) {
        let preview_url =
            local_preview_url_for_item(&item).unwrap_or_else(|| preview_url_for_item(&item));
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

        self.status = Some(format!("Previewing {}", item.title.as_ref()).into());
        cx.notify();
    }

    fn copy_item_code(&mut self, item: CatalogItem, cx: &mut Context<Self>) {
        let mut code = String::new();
        if !item.import_statement.is_empty() {
            code.push_str(item.import_statement.as_ref());
            code.push_str("\n\n");
        }
        code.push_str(item.jsx.as_ref());
        cx.write_to_clipboard(ClipboardItem::new_string(code));
        self.status = Some(format!("Copied {}", item.title.as_ref()).into());
        cx.notify();
    }

    fn open_item_docs(&mut self, item: CatalogItem, cx: &mut Context<Self>) {
        cx.open_url(&preview_url_for_item(&item));
        self.status = Some(format!("Opened docs for {}", item.title.as_ref()).into());
        cx.notify();
    }

    fn payload_for_item(&self, item: &CatalogItem) -> DraggedShadcnAsset {
        let shadcn_root = shadcn_registry_root();
        let source_path = match item.source {
            CatalogSource::MagicUi => magic_registry_root().join(item.source_path.as_ref()),
            CatalogSource::ShadcnComponent | CatalogSource::ShadcnBlock => {
                shadcn_root.join(item.source_path.as_ref())
            }
            CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst => PathBuf::new(),
        };
        let kind = match item.source {
            CatalogSource::ShadcnComponent => DraggedShadcnKind::Component,
            CatalogSource::ShadcnBlock => DraggedShadcnKind::Block,
            CatalogSource::MagicUi
            | CatalogSource::CommunityRegistry
            | CatalogSource::TwentyFirst => DraggedShadcnKind::Magic,
        };

        DraggedShadcnAsset::new(
            item.id.clone(),
            item.title.clone(),
            kind,
            source_path,
            shadcn_root,
            item.target_file_name.clone(),
            item.import_statement.clone(),
            item.jsx.clone(),
        )
    }

    fn render_item_row(
        &self,
        item: CatalogItem,
        image_url: Option<String>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let can_drag = can_drag_into_editor(item.source);
        let primary_action = if can_drag { "Insert" } else { "Open" };
        let source_label = catalog_source_label(item.source);

        div()
            .id(format!("shadcn-item-{}", item.id.as_ref()))
            .v_flex()
            .gap_2()
            .p_2()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().element_background)
            .hover(|style| style.bg(cx.theme().colors().element_hover))
            .cursor_pointer()
            .tooltip(Tooltip::text(item.description.to_string()))
            .on_click(cx.listener({
                let item = item.clone();
                move |panel, _, window, cx| {
                    if can_drag {
                        panel.insert_item(item.clone(), window, cx);
                    } else {
                        panel.open_item_docs(item.clone(), cx);
                    }
                }
            }))
            .when(can_drag, |this| {
                let payload = self.payload_for_item(&item);
                this.on_drag(payload, |asset, position, _, cx| {
                    cx.new(|_| ShadcnDragPreview {
                        title: asset.title.clone(),
                        position,
                    })
                })
            })
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(shadcn_thumbnail(&item, image_url, cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_0p5()
                            .child(
                                Label::new(item.title.clone())
                                    .size(LabelSize::Small)
                                    .truncate(),
                            )
                            .child(
                                Label::new(item.description.clone())
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted)
                                    .truncate(),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_0p5()
                            .items_end()
                            .child(Label::new(source_label).size(LabelSize::XSmall).color(
                                if can_drag {
                                    Color::Accent
                                } else {
                                    Color::Warning
                                },
                            ))
                            .child(
                                Label::new(item.category.clone())
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .child(
                        Button::new(
                            format!("shadcn-insert-{}", item.id.as_ref()),
                            primary_action,
                        )
                        .style(ButtonStyle::Subtle)
                        .size(ButtonSize::Compact)
                        .on_click(cx.listener({
                            let item = item.clone();
                            move |panel, _, window, cx| {
                                if can_drag {
                                    panel.insert_item(item.clone(), window, cx);
                                } else {
                                    panel.open_item_docs(item.clone(), cx);
                                }
                            }
                        })),
                    )
                    .child(
                        Button::new(format!("shadcn-copy-{}", item.id.as_ref()), "Copy")
                            .style(ButtonStyle::Subtle)
                            .size(ButtonSize::Compact)
                            .on_click(cx.listener({
                                let item = item.clone();
                                move |panel, _, _, cx| {
                                    panel.copy_item_code(item.clone(), cx);
                                }
                            })),
                    )
                    .child(
                        Button::new(format!("shadcn-preview-{}", item.id.as_ref()), "Preview")
                            .style(ButtonStyle::Subtle)
                            .size(ButtonSize::Compact)
                            .on_click(cx.listener({
                                let item = item.clone();
                                move |panel, _, window, cx| {
                                    panel.preview_item(item.clone(), window, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new(format!("shadcn-docs-{}", item.id.as_ref()), "Docs")
                            .style(ButtonStyle::Subtle)
                            .size(ButtonSize::Compact)
                            .on_click(cx.listener({
                                let item = item.clone();
                                move |panel, _, _, cx| {
                                    panel.open_item_docs(item.clone(), cx);
                                }
                            })),
                    ),
            )
    }
}

impl Panel for ShadcnUiPanel {
    fn persistent_name() -> &'static str {
        "UI"
    }

    fn panel_key() -> &'static str {
        SHADCN_UI_PANEL_KEY
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
        px(360.)
    }

    fn min_size(&self, _: &Window, _: &App) -> Option<Pixels> {
        Some(px(260.))
    }

    fn icon(&self, _: &Window, _: &App) -> Option<IconName> {
        None
    }

    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> {
        Some("UI")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(ToggleFocus)
    }

    fn activation_priority(&self) -> u32 {
        12
    }
}

impl Focusable for ShadcnUiPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.filter_editor.focus_handle(cx)
    }
}

impl EventEmitter<PanelEvent> for ShadcnUiPanel {}

impl Render for ShadcnUiPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_catalog_loaded(cx);
        let query = self.query(cx);
        let (items, total_matches) = self.matching_items(query.as_str(), MAX_SHADCN_ROWS);
        let mut preview_images = cached_shadcn_preview_image_urls(&items);
        self.ensure_visible_preview_images_warmed(&items, &preview_images, cx);
        let is_empty = total_matches == 0;
        let filter_counts = self.filter_counts;
        let total_count = filter_counts.count(self.source_filter);
        let mut item_rows = Vec::with_capacity(items.len());
        item_rows.extend(items.into_iter().map(|item| {
            let image_url = preview_images.remove(item.id.as_ref()).flatten();
            self.render_item_row(item, image_url, cx).into_any_element()
        }));
        let count_label = self.status.clone().unwrap_or_else(|| {
            if self.loading_catalog {
                "loading".into()
            } else {
                format!("{total_matches} / {total_count}").into()
            }
        });

        v_flex()
            .id("shadcn-ui-panel")
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
                            .child(Label::new("UI").size(LabelSize::Small))
                            .child(
                                Label::new(count_label)
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted)
                                    .truncate(),
                            ),
                    )
                    .child(self.filter_editor.clone()),
            )
            .child(self.render_filter_tabs(&filter_counts, cx))
            .child(
                div()
                    .id("shadcn-ui-panel-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .p_2()
                    .when(is_empty, |this| {
                        this.child(
                            div().h_full().flex().items_center().justify_center().child(
                                Label::new("No matching components")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            ),
                        )
                    })
                    .when(!is_empty, |this| {
                        this.child(v_flex().gap_2().children(item_rows))
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

fn catalog_search_matches(searchable: &str, query_terms: &[&str]) -> bool {
    query_terms.iter().all(|term| searchable.contains(term))
}

struct ShadcnDragPreview {
    title: SharedString,
    position: gpui::Point<Pixels>,
}

impl Render for ShadcnDragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .left(self.position.x - px(72.))
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
                    .child(Icon::new(IconName::Blocks).size(IconSize::Small))
                    .child(Label::new(self.title.clone()).size(LabelSize::XSmall)),
            )
    }
}

fn icon_for_item(item: &CatalogItem) -> IconName {
    match item.source {
        CatalogSource::ShadcnComponent => IconName::Blocks,
        CatalogSource::ShadcnBlock => IconName::Library,
        CatalogSource::MagicUi => IconName::Sparkle,
        CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst => IconName::Link,
    }
}

fn catalog_source_label(source: CatalogSource) -> &'static str {
    match source {
        CatalogSource::ShadcnComponent => "shadcn",
        CatalogSource::ShadcnBlock => "block",
        CatalogSource::MagicUi => "magic-ui",
        CatalogSource::CommunityRegistry => "registry",
        CatalogSource::TwentyFirst => "21st",
    }
}

fn can_drag_into_editor(source: CatalogSource) -> bool {
    !matches!(
        source,
        CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst
    )
}

fn shadcn_thumbnail(
    item: &CatalogItem,
    image_url: Option<String>,
    cx: &mut Context<ShadcnUiPanel>,
) -> AnyElement {
    let colors = cx.theme().colors();
    let accent = colors.editor_foreground.opacity(0.72);
    let muted = colors.text_muted.opacity(0.56);

    if let Some(image_url) = image_url {
        return div()
            .w(px(70.))
            .h(px(46.))
            .flex_none()
            .rounded_sm()
            .border_1()
            .border_color(colors.border_variant)
            .bg(colors.element_background)
            .overflow_hidden()
            .child(img(image_url).size_full().object_fit(ObjectFit::Cover))
            .into_any_element();
    }

    let base = || {
        div()
            .w(px(70.))
            .h(px(46.))
            .flex_none()
            .rounded_sm()
            .border_1()
            .border_color(colors.border_variant)
            .bg(colors.element_background)
            .overflow_hidden()
            .p_1()
    };

    match item.source {
        CatalogSource::ShadcnBlock => base()
            .v_flex()
            .gap_1()
            .child(
                h_flex()
                    .gap_0p5()
                    .child(div().w(px(12.)).h(px(32.)).rounded_sm().bg(colors.border))
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_0p5()
                            .child(div().h(px(8.)).rounded_sm().bg(colors.element_hover))
                            .child(
                                h_flex()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .flex_1()
                                            .h(px(18.))
                                            .rounded_sm()
                                            .bg(colors.element_selected),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .h(px(18.))
                                            .rounded_sm()
                                            .bg(colors.element_selected),
                                    ),
                            ),
                    ),
            )
            .into_any_element(),
        CatalogSource::MagicUi => base()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(42.))
                    .h(px(22.))
                    .rounded_full()
                    .border_1()
                    .border_color(colors.border_focused)
                    .bg(colors.element_selected)
                    .child(div().m_1().h(px(4.)).rounded_full().bg(accent)),
            )
            .into_any_element(),
        CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst => base()
            .v_flex()
            .gap_1()
            .child(
                Icon::new(icon_for_item(item))
                    .size(IconSize::Small)
                    .color(Color::Muted),
            )
            .child(div().h(px(4.)).rounded_full().bg(muted))
            .child(div().w(px(40.)).h(px(4.)).rounded_full().bg(colors.border))
            .into_any_element(),
        CatalogSource::ShadcnComponent => match item.id.as_ref() {
            "button" | "rainbow-button" => base()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(44.))
                        .h(px(18.))
                        .rounded_sm()
                        .border_1()
                        .border_color(colors.border_focused)
                        .bg(colors.element_selected),
                )
                .into_any_element(),
            "input" | "select" => base()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(52.))
                        .h(px(18.))
                        .rounded_sm()
                        .border_1()
                        .border_color(colors.border)
                        .child(div().m_1().h(px(4.)).rounded_full().bg(muted)),
                )
                .into_any_element(),
            "textarea" | "form" => base()
                .v_flex()
                .gap_0p5()
                .child(div().w(px(38.)).h(px(5.)).rounded_full().bg(muted))
                .child(
                    div()
                        .h(px(22.))
                        .rounded_sm()
                        .border_1()
                        .border_color(colors.border)
                        .bg(colors.element_background),
                )
                .into_any_element(),
            "chart" => base()
                .h_flex()
                .items_end()
                .gap_0p5()
                .child(div().flex_1().h(px(14.)).rounded_sm().bg(colors.border))
                .child(
                    div()
                        .flex_1()
                        .h(px(28.))
                        .rounded_sm()
                        .bg(colors.border_focused),
                )
                .child(div().flex_1().h(px(20.)).rounded_sm().bg(colors.border))
                .child(
                    div()
                        .flex_1()
                        .h(px(34.))
                        .rounded_sm()
                        .bg(colors.border_focused),
                )
                .into_any_element(),
            "progress" | "slider" => base()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(52.))
                        .h(px(8.))
                        .rounded_full()
                        .bg(colors.border)
                        .child(
                            div()
                                .w(px(30.))
                                .h_full()
                                .rounded_full()
                                .bg(colors.border_focused),
                        ),
                )
                .into_any_element(),
            "switch" | "toggle" | "toggle-group" => base()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(38.))
                        .h(px(20.))
                        .rounded_full()
                        .bg(colors.element_selected)
                        .p_0p5()
                        .child(div().w(px(16.)).h(px(16.)).rounded_full().bg(accent)),
                )
                .into_any_element(),
            "calendar" | "date-picker" => base()
                .v_flex()
                .gap_0p5()
                .child(div().h(px(6.)).rounded_sm().bg(colors.element_selected))
                .child(
                    h_flex()
                        .gap_0p5()
                        .child(div().flex_1().h(px(7.)).rounded_sm().bg(colors.border))
                        .child(div().flex_1().h(px(7.)).rounded_sm().bg(colors.border))
                        .child(div().flex_1().h(px(7.)).rounded_sm().bg(colors.border)),
                )
                .child(
                    h_flex()
                        .gap_0p5()
                        .child(div().flex_1().h(px(7.)).rounded_sm().bg(colors.border))
                        .child(
                            div()
                                .flex_1()
                                .h(px(7.))
                                .rounded_sm()
                                .bg(colors.border_focused),
                        )
                        .child(div().flex_1().h(px(7.)).rounded_sm().bg(colors.border)),
                )
                .into_any_element(),
            "card" => base()
                .v_flex()
                .gap_1()
                .child(div().h(px(8.)).rounded_sm().bg(colors.element_selected))
                .child(div().h(px(4.)).rounded_full().bg(muted))
                .child(div().w(px(42.)).h(px(4.)).rounded_full().bg(colors.border))
                .into_any_element(),
            "badge" => base()
                .h_flex()
                .gap_1()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(24.))
                        .h(px(12.))
                        .rounded_full()
                        .bg(colors.element_selected),
                )
                .child(div().w(px(18.)).h(px(12.)).rounded_full().bg(colors.border))
                .into_any_element(),
            "avatar" => base()
                .h_flex()
                .gap_1()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(24.))
                        .h(px(24.))
                        .rounded_full()
                        .bg(colors.element_selected),
                )
                .child(
                    v_flex()
                        .gap_0p5()
                        .child(div().w(px(28.)).h(px(5.)).rounded_full().bg(muted))
                        .child(div().w(px(20.)).h(px(4.)).rounded_full().bg(colors.border)),
                )
                .into_any_element(),
            "tabs" => base()
                .v_flex()
                .gap_1()
                .child(
                    h_flex()
                        .gap_0p5()
                        .child(
                            div()
                                .flex_1()
                                .h(px(10.))
                                .rounded_sm()
                                .bg(colors.element_selected),
                        )
                        .child(div().flex_1().h(px(10.)).rounded_sm().bg(colors.border))
                        .child(div().flex_1().h(px(10.)).rounded_sm().bg(colors.border)),
                )
                .child(div().h(px(18.)).rounded_sm().bg(colors.element_hover))
                .into_any_element(),
            "table" => base()
                .v_flex()
                .gap_0p5()
                .child(div().h(px(6.)).rounded_sm().bg(colors.element_selected))
                .child(div().h(px(5.)).rounded_sm().bg(colors.border))
                .child(div().h(px(5.)).rounded_sm().bg(colors.border))
                .child(div().h(px(5.)).rounded_sm().bg(colors.border))
                .into_any_element(),
            "checkbox" => base()
                .h_flex()
                .gap_1()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(16.))
                        .h(px(16.))
                        .rounded_sm()
                        .border_1()
                        .border_color(colors.border_focused)
                        .bg(colors.element_selected),
                )
                .child(div().w(px(32.)).h(px(5.)).rounded_full().bg(muted))
                .into_any_element(),
            "dialog" | "alert-dialog" => base()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(46.))
                        .h(px(30.))
                        .rounded_sm()
                        .border_1()
                        .border_color(colors.border_focused)
                        .bg(colors.element_selected)
                        .p_1()
                        .child(div().h(px(5.)).rounded_full().bg(muted)),
                )
                .into_any_element(),
            "dropdown-menu" | "context-menu" | "menubar" => base()
                .v_flex()
                .gap_0p5()
                .child(
                    div()
                        .w(px(38.))
                        .h(px(7.))
                        .rounded_sm()
                        .bg(colors.element_selected),
                )
                .child(div().w(px(50.)).h(px(6.)).rounded_sm().bg(colors.border))
                .child(div().w(px(42.)).h(px(6.)).rounded_sm().bg(colors.border))
                .into_any_element(),
            "accordion" => base()
                .v_flex()
                .gap_0p5()
                .child(div().h(px(8.)).rounded_sm().bg(colors.element_selected))
                .child(div().h(px(8.)).rounded_sm().bg(colors.element_hover))
                .child(div().h(px(8.)).rounded_sm().bg(colors.border))
                .into_any_element(),
            "popover" | "tooltip" | "hover-card" => base()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(44.))
                        .h(px(24.))
                        .rounded_sm()
                        .border_1()
                        .border_color(colors.border_focused)
                        .bg(colors.element_selected),
                )
                .into_any_element(),
            "toast" | "sonner" => base()
                .v_flex()
                .gap_0p5()
                .child(
                    div()
                        .w(px(44.))
                        .h(px(8.))
                        .rounded_sm()
                        .bg(colors.element_selected),
                )
                .child(div().w(px(34.)).h(px(5.)).rounded_sm().bg(muted))
                .child(div().w(px(50.)).h(px(5.)).rounded_sm().bg(colors.border))
                .into_any_element(),
            "skeleton" => base()
                .v_flex()
                .gap_1()
                .child(div().w(px(44.)).h(px(8.)).rounded_full().bg(colors.border))
                .child(div().w(px(56.)).h(px(8.)).rounded_full().bg(colors.border))
                .child(div().w(px(32.)).h(px(8.)).rounded_full().bg(colors.border))
                .into_any_element(),
            "scroll-area" => base()
                .h_flex()
                .gap_0p5()
                .child(
                    v_flex()
                        .flex_1()
                        .gap_0p5()
                        .child(div().h(px(6.)).rounded_sm().bg(colors.element_selected))
                        .child(div().h(px(6.)).rounded_sm().bg(colors.border))
                        .child(div().h(px(6.)).rounded_sm().bg(colors.border))
                        .child(div().h(px(6.)).rounded_sm().bg(colors.border)),
                )
                .child(
                    div()
                        .w(px(3.))
                        .h_full()
                        .rounded_full()
                        .bg(colors.border_focused),
                )
                .into_any_element(),
            "sidebar" => base()
                .h_flex()
                .gap_0p5()
                .child(
                    div()
                        .w(px(18.))
                        .h_full()
                        .rounded_sm()
                        .bg(colors.element_selected),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .gap_0p5()
                        .child(div().h(px(8.)).rounded_sm().bg(colors.element_hover))
                        .child(div().h(px(18.)).rounded_sm().bg(colors.border)),
                )
                .into_any_element(),
            _ => base()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(icon_for_item(item))
                        .size(IconSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element(),
        },
    }
}

fn shadcn_registry_root() -> PathBuf {
    repo_root()
        .join("inspirations")
        .join("shadcn-ui")
        .join("apps")
        .join("v4")
        .join("registry")
        .join("new-york-v4")
}

fn shadcn_manifest_root() -> PathBuf {
    repo_root()
        .join("inspirations")
        .join("shadcn-ui")
        .join("apps")
        .join("v4")
        .join("public")
        .join("r")
        .join("styles")
        .join("new-york-v4")
}

fn shadcn_manifest_path(id: &str) -> PathBuf {
    shadcn_manifest_root().join(format!("{id}.json"))
}

fn shadcn_examples_root() -> PathBuf {
    repo_root()
        .join("inspirations")
        .join("shadcn-ui")
        .join("apps")
        .join("v4")
        .join("registry")
        .join("new-york-v4")
        .join("examples")
}

fn magic_registry_root() -> PathBuf {
    repo_root()
        .join("inspirations")
        .join("magicui")
        .join("apps")
        .join("www")
        .join("registry")
        .join("magicui")
}

fn repo_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("G:/Zed"))
}

fn shadcn_catalog_cached() -> Vec<CatalogItem> {
    SHADCN_CATALOG_CACHE
        .get_or_init(|| {
            load_shadcn_catalog_rkyv_cache().unwrap_or_else(|| {
                let items = shadcn_catalog();
                let _ = write_shadcn_catalog_rkyv_cache(&items);
                items
            })
        })
        .clone()
}

fn initial_shadcn_catalog() -> Vec<CatalogItem> {
    load_shadcn_catalog_rkyv_cache().unwrap_or_else(static_shadcn_catalog)
}

fn static_shadcn_catalog() -> Vec<CatalogItem> {
    SHADCN_STATIC_CATALOG_CACHE
        .get_or_init(|| {
            let mut items = curated_catalog();
            let static_items = static_catalog_index_items();
            let registry_items = registry_directory_items();
            let twenty_first_items = twenty_first_items();
            let extra_capacity =
                static_items.len() + registry_items.len() + twenty_first_items.len();

            items.reserve(extra_capacity);
            let mut existing_ids = HashSet::with_capacity(items.len() + extra_capacity);
            existing_ids.extend(items.iter().map(|item| item.id.to_string()));

            for item in static_items {
                if existing_ids.insert(item.id.to_string()) {
                    items.push(item);
                }
            }

            for item in registry_items {
                if existing_ids.insert(item.id.to_string()) {
                    items.push(item);
                }
            }

            for item in twenty_first_items {
                if existing_ids.insert(item.id.to_string()) {
                    items.push(item);
                }
            }

            items
        })
        .clone()
}

fn static_catalog_index_items() -> Vec<CatalogItem> {
    let lines = STATIC_SHADCN_CATALOG_INDEX.lines();
    let mut items = Vec::with_capacity(lines.size_hint().0);

    for line in lines {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }

        let mut columns = line.split('\t');
        let Some(id) = columns.next() else {
            continue;
        };
        let Some(title) = columns.next() else {
            continue;
        };
        let Some(description) = columns.next() else {
            continue;
        };
        let Some(category) = columns.next() else {
            continue;
        };
        let Some(source) = columns.next() else {
            continue;
        };
        let Some(source_path) = columns.next() else {
            continue;
        };
        let Some(target_file_name) = columns.next() else {
            continue;
        };
        let Some(import_statement) = columns.next() else {
            continue;
        };
        let Some(jsx) = columns.next() else {
            continue;
        };
        if columns.next().is_some() {
            continue;
        }
        let Ok(source) = source.parse::<u8>() else {
            continue;
        };

        items.push(CatalogItem {
            id: decode_static_catalog_field(id).into(),
            title: decode_static_catalog_field(title).into(),
            description: decode_static_catalog_field(description).into(),
            category: decode_static_catalog_field(category).into(),
            source: catalog_source_from_u8(source),
            source_path: decode_static_catalog_field(source_path).into(),
            target_file_name: decode_static_catalog_field(target_file_name).into(),
            import_statement: decode_static_catalog_field(import_statement).into(),
            jsx: decode_static_catalog_field(jsx).into(),
        });
    }

    items
}

fn decode_static_catalog_field(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            match chars.next() {
                Some('n') => decoded.push('\n'),
                Some('t') => decoded.push('\t'),
                Some('\\') => decoded.push('\\'),
                Some(other) => {
                    decoded.push('\\');
                    decoded.push(other);
                }
                None => decoded.push('\\'),
            }
        } else {
            decoded.push(character);
        }
    }
    decoded
}

fn catalog_source_to_u8(source: CatalogSource) -> u8 {
    match source {
        CatalogSource::ShadcnComponent => 0,
        CatalogSource::ShadcnBlock => 1,
        CatalogSource::MagicUi => 2,
        CatalogSource::CommunityRegistry => 3,
        CatalogSource::TwentyFirst => 4,
    }
}

fn catalog_source_from_u8(source: u8) -> CatalogSource {
    match source {
        1 => CatalogSource::ShadcnBlock,
        2 => CatalogSource::MagicUi,
        3 => CatalogSource::CommunityRegistry,
        4 => CatalogSource::TwentyFirst,
        _ => CatalogSource::ShadcnComponent,
    }
}

fn shadcn_catalog_cache_path() -> PathBuf {
    repo_root()
        .join("target")
        .join("shadcn-ui-panel")
        .join(CATALOG_CACHE_FILE_NAME)
}

fn load_shadcn_catalog_rkyv_cache() -> Option<Vec<CatalogItem>> {
    let file = File::open(shadcn_catalog_cache_path()).ok()?;
    let mmap = unsafe { MmapOptions::new().map(&file).ok()? };
    let archived = unsafe { archived_root::<Vec<CachedCatalogItem>>(&mmap) };
    let mut deserializer = Infallible;
    let records: Vec<CachedCatalogItem> = archived.deserialize(&mut deserializer).ok()?;
    let mut items = Vec::with_capacity(records.len());
    items.extend(
        records
            .into_iter()
            .map(CachedCatalogItem::into_catalog_item),
    );
    Some(items)
}

fn write_shadcn_catalog_rkyv_cache(items: &[CatalogItem]) -> Option<()> {
    let path = shadcn_catalog_cache_path();
    std_fs::create_dir_all(path.parent()?).ok()?;
    let mut records = Vec::with_capacity(items.len());
    records.extend(items.iter().map(CachedCatalogItem::from_catalog_item));
    let mut serializer = AllocSerializer::<4096>::default();
    serializer.serialize_value(&records).ok()?;
    let bytes = serializer.into_serializer().into_inner();
    std_fs::write(path, bytes).ok()?;
    Some(())
}

fn shadcn_catalog() -> Vec<CatalogItem> {
    let mut items = curated_catalog();
    let manifest_items = manifest_catalog_items();
    let magic_items = magic_catalog_items();
    let registry_items = registry_directory_items();
    let twenty_first_items = twenty_first_items();
    let extra_capacity =
        manifest_items.len() + magic_items.len() + registry_items.len() + twenty_first_items.len();

    items.reserve(extra_capacity);
    let mut existing_ids = HashSet::with_capacity(items.len() + extra_capacity);
    existing_ids.extend(items.iter().map(|item| item.id.to_string()));

    for item in manifest_items {
        if existing_ids.insert(item.id.to_string()) {
            items.push(item);
        }
    }

    for item in magic_items {
        if existing_ids.insert(item.id.to_string()) {
            items.push(item);
        }
    }

    for item in registry_items {
        if existing_ids.insert(item.id.to_string()) {
            items.push(item);
        }
    }

    for item in twenty_first_items {
        if existing_ids.insert(item.id.to_string()) {
            items.push(item);
        }
    }

    items
}

fn curated_catalog() -> Vec<CatalogItem> {
    vec![
        component(
            "button",
            "Button",
            "Action button variants",
            "ui/button.tsx",
            "button.tsx",
            "import { Button } from \"@/components/ui/button\";",
            "<Button>Button</Button>",
        ),
        component(
            "card",
            "Card",
            "Composable content surface",
            "ui/card.tsx",
            "card.tsx",
            "import { Card, CardContent, CardHeader, CardTitle } from \"@/components/ui/card\";",
            "<Card><CardHeader><CardTitle>Card title</CardTitle></CardHeader><CardContent>Card content</CardContent></Card>",
        ),
        component(
            "dialog",
            "Dialog",
            "Modal dialog primitives",
            "ui/dialog.tsx",
            "dialog.tsx",
            "import { Button } from \"@/components/ui/button\";\nimport { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from \"@/components/ui/dialog\";",
            "<Dialog><DialogTrigger asChild><Button>Open</Button></DialogTrigger><DialogContent><DialogHeader><DialogTitle>Dialog title</DialogTitle></DialogHeader></DialogContent></Dialog>",
        ),
        component(
            "chart",
            "Chart",
            "Recharts wrapper and theme bridge",
            "ui/chart.tsx",
            "chart.tsx",
            "import { Bar, BarChart } from \"recharts\";\nimport { ChartContainer } from \"@/components/ui/chart\";",
            "<ChartContainer config={{ desktop: { label: \"Desktop\", color: \"var(--chart-1)\" } }} className=\"min-h-[200px] w-full\"><BarChart data={[{ month: \"Jan\", desktop: 186 }]}><Bar dataKey=\"desktop\" fill=\"var(--color-desktop)\" radius={4} /></BarChart></ChartContainer>",
        ),
        component(
            "sidebar",
            "Sidebar",
            "App shell sidebar system",
            "ui/sidebar.tsx",
            "sidebar.tsx",
            "import { SidebarProvider, SidebarTrigger } from \"@/components/ui/sidebar\";",
            "<SidebarProvider><SidebarTrigger /></SidebarProvider>",
        ),
        component(
            "table",
            "Table",
            "Accessible table primitives",
            "ui/table.tsx",
            "table.tsx",
            "import { Table, TableBody, TableCell, TableRow } from \"@/components/ui/table\";",
            "<Table><TableBody><TableRow><TableCell>Cell</TableCell></TableRow></TableBody></Table>",
        ),
        component(
            "form",
            "Form",
            "React Hook Form wrappers",
            "ui/form.tsx",
            "form.tsx",
            "import { Form } from \"@/components/ui/form\";",
            "<Form {...form}></Form>",
        ),
        component(
            "input",
            "Input",
            "Text input primitive",
            "ui/input.tsx",
            "input.tsx",
            "import { Input } from \"@/components/ui/input\";",
            "<Input placeholder=\"Email\" />",
        ),
        component(
            "select",
            "Select",
            "Radix select component",
            "ui/select.tsx",
            "select.tsx",
            "import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from \"@/components/ui/select\";",
            "<Select><SelectTrigger><SelectValue placeholder=\"Select\" /></SelectTrigger><SelectContent><SelectItem value=\"one\">One</SelectItem></SelectContent></Select>",
        ),
        component(
            "tabs",
            "Tabs",
            "Tabbed interface",
            "ui/tabs.tsx",
            "tabs.tsx",
            "import { Tabs, TabsContent, TabsList, TabsTrigger } from \"@/components/ui/tabs\";",
            "<Tabs defaultValue=\"one\"><TabsList><TabsTrigger value=\"one\">One</TabsTrigger></TabsList><TabsContent value=\"one\">Content</TabsContent></Tabs>",
        ),
        component(
            "dropdown-menu",
            "Dropdown Menu",
            "Contextual menu actions",
            "ui/dropdown-menu.tsx",
            "dropdown-menu.tsx",
            "import { Button } from \"@/components/ui/button\";\nimport { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from \"@/components/ui/dropdown-menu\";",
            "<DropdownMenu><DropdownMenuTrigger asChild><Button>Menu</Button></DropdownMenuTrigger><DropdownMenuContent><DropdownMenuItem>Action</DropdownMenuItem></DropdownMenuContent></DropdownMenu>",
        ),
        component(
            "sheet",
            "Sheet",
            "Slide-over dialog",
            "ui/sheet.tsx",
            "sheet.tsx",
            "import { Button } from \"@/components/ui/button\";\nimport { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from \"@/components/ui/sheet\";",
            "<Sheet><SheetTrigger asChild><Button>Open</Button></SheetTrigger><SheetContent><SheetHeader><SheetTitle>Sheet title</SheetTitle></SheetHeader></SheetContent></Sheet>",
        ),
        component(
            "badge",
            "Badge",
            "Inline status badge",
            "ui/badge.tsx",
            "badge.tsx",
            "import { Badge } from \"@/components/ui/badge\";",
            "<Badge>Badge</Badge>",
        ),
        component(
            "avatar",
            "Avatar",
            "User image fallback",
            "ui/avatar.tsx",
            "avatar.tsx",
            "import { Avatar, AvatarFallback, AvatarImage } from \"@/components/ui/avatar\";",
            "<Avatar><AvatarImage src=\"/avatar.png\" alt=\"Avatar\" /><AvatarFallback>UI</AvatarFallback></Avatar>",
        ),
        component(
            "accordion",
            "Accordion",
            "Expandable sections",
            "ui/accordion.tsx",
            "accordion.tsx",
            "import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from \"@/components/ui/accordion\";",
            "<Accordion type=\"single\" collapsible><AccordionItem value=\"item-1\"><AccordionTrigger>Question</AccordionTrigger><AccordionContent>Answer</AccordionContent></AccordionItem></Accordion>",
        ),
        component(
            "checkbox",
            "Checkbox",
            "Boolean field control",
            "ui/checkbox.tsx",
            "checkbox.tsx",
            "import { Checkbox } from \"@/components/ui/checkbox\";",
            "<Checkbox />",
        ),
        component(
            "popover",
            "Popover",
            "Floating anchored content",
            "ui/popover.tsx",
            "popover.tsx",
            "import { Button } from \"@/components/ui/button\";\nimport { Popover, PopoverContent, PopoverTrigger } from \"@/components/ui/popover\";",
            "<Popover><PopoverTrigger asChild><Button>Open</Button></PopoverTrigger><PopoverContent>Content</PopoverContent></Popover>",
        ),
        component(
            "tooltip",
            "Tooltip",
            "Hover/focus help text",
            "ui/tooltip.tsx",
            "tooltip.tsx",
            "import { Button } from \"@/components/ui/button\";\nimport { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from \"@/components/ui/tooltip\";",
            "<TooltipProvider><Tooltip><TooltipTrigger asChild><Button>Hover</Button></TooltipTrigger><TooltipContent>Tooltip</TooltipContent></Tooltip></TooltipProvider>",
        ),
        block(
            "dashboard-01",
            "Dashboard 01",
            "Full analytics dashboard block",
            "blocks/dashboard-01",
            "dashboard-01",
            "import Dashboard01 from \"@/components/blocks/dashboard-01/page\";",
            "<Dashboard01 />",
        ),
        block(
            "login-01",
            "Login 01",
            "Authentication screen block",
            "blocks/login-01",
            "login-01",
            "import Login01 from \"@/components/blocks/login-01/page\";",
            "<Login01 />",
        ),
        block(
            "sidebar-01",
            "Sidebar 01",
            "Application shell sidebar block",
            "blocks/sidebar-01",
            "sidebar-01",
            "import Sidebar01 from \"@/components/blocks/sidebar-01/page\";",
            "<Sidebar01 />",
        ),
        block(
            "signup-01",
            "Signup 01",
            "Signup flow block",
            "blocks/signup-01",
            "signup-01",
            "import Signup01 from \"@/components/blocks/signup-01/page\";",
            "<Signup01 />",
        ),
        magic(
            "marquee",
            "Marquee",
            "Animated horizontal content rail",
            "marquee.tsx",
            "marquee.tsx",
            "import { Marquee } from \"@/components/magicui/marquee\";",
            "<Marquee><div>Magic UI</div></Marquee>",
        ),
        magic(
            "border-beam",
            "Border Beam",
            "Animated border highlight",
            "border-beam.tsx",
            "border-beam.tsx",
            "import { BorderBeam } from \"@/components/magicui/border-beam\";",
            "<div className=\"relative overflow-hidden rounded-lg border p-6\"><BorderBeam /></div>",
        ),
        magic(
            "bento-grid",
            "Bento Grid",
            "Responsive feature grid primitives",
            "bento-grid.tsx",
            "bento-grid.tsx",
            "import { BentoCard, BentoGrid } from \"@/components/magicui/bento-grid\";",
            "<BentoGrid><BentoCard name=\"Feature\" description=\"Description\" href=\"#\" cta=\"Open\" className=\"col-span-1\" background={null} Icon={() => null} /></BentoGrid>",
        ),
        magic(
            "magic-card",
            "Magic Card",
            "Interactive gradient card",
            "magic-card.tsx",
            "magic-card.tsx",
            "import { MagicCard } from \"@/components/magicui/magic-card\";",
            "<MagicCard>Magic Card</MagicCard>",
        ),
        magic(
            "rainbow-button",
            "Rainbow Button",
            "Animated gradient button",
            "rainbow-button.tsx",
            "rainbow-button.tsx",
            "import { RainbowButton } from \"@/components/magicui/rainbow-button\";",
            "<RainbowButton>Rainbow Button</RainbowButton>",
        ),
        magic(
            "sparkles-text",
            "Sparkles Text",
            "Animated text highlight",
            "sparkles-text.tsx",
            "sparkles-text.tsx",
            "import { SparklesText } from \"@/components/magicui/sparkles-text\";",
            "<SparklesText text=\"Sparkles\" />",
        ),
    ]
}

fn component(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    source_path: &'static str,
    target_file_name: &'static str,
    import_statement: &'static str,
    jsx: &'static str,
) -> CatalogItem {
    CatalogItem {
        id: id.into(),
        title: title.into(),
        description: description.into(),
        category: "ui".into(),
        source: CatalogSource::ShadcnComponent,
        source_path: source_path.into(),
        target_file_name: target_file_name.into(),
        import_statement: import_statement.into(),
        jsx: jsx.into(),
    }
}

fn block(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    source_path: &'static str,
    target_file_name: &'static str,
    import_statement: &'static str,
    jsx: &'static str,
) -> CatalogItem {
    CatalogItem {
        id: id.into(),
        title: title.into(),
        description: description.into(),
        category: "block".into(),
        source: CatalogSource::ShadcnBlock,
        source_path: source_path.into(),
        target_file_name: target_file_name.into(),
        import_statement: import_statement.into(),
        jsx: jsx.into(),
    }
}

fn magic(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    source_path: &'static str,
    target_file_name: &'static str,
    import_statement: &'static str,
    jsx: &'static str,
) -> CatalogItem {
    CatalogItem {
        id: id.into(),
        title: title.into(),
        description: description.into(),
        category: "magic".into(),
        source: CatalogSource::MagicUi,
        source_path: source_path.into(),
        target_file_name: target_file_name.into(),
        import_statement: import_statement.into(),
        jsx: jsx.into(),
    }
}

fn registry_directory_items() -> Vec<CatalogItem> {
    let registries = registry_directory::COMMUNITY_REGISTRIES;
    let mut items = Vec::with_capacity(registries.len());
    items.extend(registries.iter().map(|registry| {
        let name = registry.name.trim_start_matches('@');
        let _registry_description = registry.description;
        CatalogItem {
            id: format!("registry-{name}").into(),
            title: registry.name.into(),
            description: "External UI registry".into(),
            category: "registry".into(),
            source: CatalogSource::CommunityRegistry,
            source_path: registry.homepage.into(),
            target_file_name: registry.name.into(),
            import_statement: format!("npx shadcn@latest add {}/<component>", registry.name).into(),
            jsx: registry.url.into(),
        }
    }));
    items
}

fn twenty_first_items() -> Vec<CatalogItem> {
    vec![CatalogItem {
        id: "registry-21st-dev-magic".into(),
        title: "21st.dev Magic".into(),
        description: "21st.dev component source and Magic MCP reference".into(),
        category: "registry".into(),
        source: CatalogSource::TwentyFirst,
        source_path: "https://21st.dev/magic".into(),
        target_file_name: "@21st-dev/magic-mcp".into(),
        import_statement: "npx -y @21st-dev/magic-mcp".into(),
        jsx: "Requires TWENTY_FIRST_API_KEY; source reference is cloned at inspirations/21st-magic-mcp".into(),
    }]
}

#[derive(Deserialize)]
struct RegistryManifestSummary {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "type", default)]
    item_type: Option<String>,
    #[serde(default)]
    files: Vec<RegistryManifestFileSummary>,
}

#[derive(Deserialize)]
struct RegistryManifestFileSummary {
    path: String,
    #[serde(default)]
    content: String,
    #[serde(rename = "type", default)]
    file_type: Option<String>,
}

fn manifest_catalog_items() -> Vec<CatalogItem> {
    let manifest_root = shadcn_manifest_root();
    let Ok(entries) = std_fs::read_dir(&manifest_root) else {
        return Vec::new();
    };

    let entries = entries.flatten();
    let mut items = Vec::with_capacity(entries.size_hint().0);
    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = std_fs::read_to_string(&path) else {
            continue;
        };
        let Ok(summary) = serde_json::from_str::<RegistryManifestSummary>(&text) else {
            continue;
        };

        let name = summary.name.clone();
        let title = titleize_id(&name);
        let description = summary
            .description
            .clone()
            .unwrap_or_else(|| format!("Install {title} from the shadcn registry"));
        match summary.item_type.as_deref() {
            Some("registry:block" | "registry:example") => {
                let Some(primary_file) =
                    primary_registry_file(&summary, CatalogSource::ShadcnBlock)
                else {
                    continue;
                };
                let category = if summary.item_type.as_deref() == Some("registry:example") {
                    "example"
                } else {
                    "block"
                };
                let fallback_name = component_identifier(&name);
                let import_reference =
                    primary_export_reference(&primary_file.content, &fallback_name);
                let import_name = import_reference.local_name();
                let import_path = project_import_path_for_registry_file(&primary_file.path)
                    .unwrap_or_else(|| format!("@/components/blocks/{name}/page"));
                let source_path = source_path_for_registry_file(&primary_file.path)
                    .unwrap_or_else(|| format!("blocks/{name}/page.tsx"));
                let target_file_name = target_file_name_for_registry_file(&primary_file.path)
                    .unwrap_or_else(|| name.clone());
                items.push(CatalogItem {
                    id: name.clone().into(),
                    title: title.into(),
                    description: description.into(),
                    category: category.into(),
                    source: CatalogSource::ShadcnBlock,
                    source_path: source_path.into(),
                    target_file_name: target_file_name.into(),
                    import_statement: import_reference.import_statement(&import_path).into(),
                    jsx: format!("<{import_name} />").into(),
                });
            }
            Some("registry:ui") => {
                let Some(primary_file) =
                    primary_registry_file(&summary, CatalogSource::ShadcnComponent)
                else {
                    continue;
                };
                let fallback_name = component_identifier(&name);
                let import_reference =
                    primary_export_reference(&primary_file.content, &fallback_name);
                let import_name = import_reference.local_name();
                let import_path = project_import_path_for_registry_file(&primary_file.path)
                    .unwrap_or_else(|| format!("@/components/ui/{name}"));
                let source_path = source_path_for_registry_file(&primary_file.path)
                    .unwrap_or_else(|| format!("ui/{name}.tsx"));
                let target_file_name = target_file_name_for_registry_file(&primary_file.path)
                    .unwrap_or_else(|| format!("{name}.tsx"));
                items.push(CatalogItem {
                    id: name.clone().into(),
                    title: title.into(),
                    description: description.into(),
                    category: "ui".into(),
                    source: CatalogSource::ShadcnComponent,
                    source_path: source_path.into(),
                    target_file_name: target_file_name.into(),
                    import_statement: import_reference.import_statement(&import_path).into(),
                    jsx: format!("<{import_name} />").into(),
                });
            }
            _ => {}
        }
    }

    items.sort_by(|left, right| {
        left.category
            .as_ref()
            .cmp(right.category.as_ref())
            .then_with(|| left.title.as_ref().cmp(right.title.as_ref()))
    });
    items
}

fn magic_catalog_items() -> Vec<CatalogItem> {
    let magic_root = magic_registry_root();
    let Ok(entries) = std_fs::read_dir(&magic_root) else {
        return Vec::new();
    };

    let entries = entries.flatten();
    let mut items = Vec::with_capacity(entries.size_hint().0);
    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("tsx") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let title = titleize_id(stem);
        let content = std_fs::read_to_string(&path).unwrap_or_default();
        let fallback_name = component_identifier(stem);
        let import_reference = primary_export_reference(&content, &fallback_name);
        let import_name = import_reference.local_name();
        items.push(CatalogItem {
            id: stem.to_string().into(),
            title: title.clone().into(),
            description: format!("Install {title} from Magic UI").into(),
            category: "magic".into(),
            source: CatalogSource::MagicUi,
            source_path: format!("{stem}.tsx").into(),
            target_file_name: format!("{stem}.tsx").into(),
            import_statement: import_reference
                .import_statement(&format!("@/components/magicui/{stem}"))
                .into(),
            jsx: format!("<{import_name} />").into(),
        });
    }

    items.sort_by(|left, right| left.title.as_ref().cmp(right.title.as_ref()));
    items
}

#[derive(Clone)]
enum ExportReference {
    Named(String),
    Default(String),
}

impl ExportReference {
    fn local_name(&self) -> &str {
        match self {
            Self::Named(name) | Self::Default(name) => name,
        }
    }

    fn import_statement(&self, import_path: &str) -> String {
        match self {
            Self::Named(name) => format!("import {{ {name} }} from \"{import_path}\";"),
            Self::Default(name) => format!("import {name} from \"{import_path}\";"),
        }
    }
}

fn primary_registry_file<'a>(
    summary: &'a RegistryManifestSummary,
    source: CatalogSource,
) -> Option<&'a RegistryManifestFileSummary> {
    match source {
        CatalogSource::ShadcnComponent => summary
            .files
            .iter()
            .find(|file| file.path.starts_with("registry/new-york-v4/ui/"))
            .or_else(|| summary.files.first()),
        CatalogSource::ShadcnBlock => summary
            .files
            .iter()
            .find(|file| file.file_type.as_deref() == Some("registry:page"))
            .or_else(|| {
                summary
                    .files
                    .iter()
                    .find(|file| file.path.ends_with("/page.tsx"))
            })
            .or_else(|| {
                summary.files.iter().find(|file| {
                    matches!(
                        file.file_type.as_deref(),
                        Some("registry:block" | "registry:component")
                    )
                })
            })
            .or_else(|| summary.files.first()),
        CatalogSource::MagicUi | CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst => {
            None
        }
    }
}

fn source_path_for_registry_file(path: &str) -> Option<String> {
    path.strip_prefix("registry/new-york-v4/")
        .map(ToOwned::to_owned)
}

fn target_file_name_for_registry_file(path: &str) -> Option<String> {
    const PREFIX: &str = "registry/new-york-v4/";
    let rest = path.strip_prefix(PREFIX)?;
    if let Some(rest) = rest.strip_prefix("ui/") {
        return Some(rest.to_string());
    }
    if let Some(rest) = rest.strip_prefix("blocks/") {
        return Some(rest.to_string());
    }
    if let Some(rest) = rest.strip_prefix("charts/") {
        return Some(format!("charts/{rest}"));
    }
    if let Some(rest) = rest.strip_prefix("examples/") {
        return Some(format!("examples/{rest}"));
    }
    if let Some(rest) = rest.strip_prefix("internal/") {
        return Some(format!("internal/{rest}"));
    }

    Some(rest.to_string())
}

fn project_import_path_for_registry_file(path: &str) -> Option<String> {
    let target = target_file_name_for_registry_file(path)?;
    let without_extension = target
        .strip_suffix(".tsx")
        .or_else(|| target.strip_suffix(".ts"))
        .or_else(|| target.strip_suffix(".jsx"))
        .or_else(|| target.strip_suffix(".js"))
        .unwrap_or(target.as_str());

    if path.starts_with("registry/new-york-v4/ui/") {
        Some(format!("@/components/ui/{without_extension}"))
    } else {
        Some(format!("@/components/blocks/{without_extension}"))
    }
}

fn primary_export_reference(content: &str, fallback: &str) -> ExportReference {
    if let Some(name) = exported_block_candidate(content) {
        return ExportReference::Named(name);
    }

    for marker in ["export function ", "export const ", "export class "] {
        let mut remaining = content;
        while let Some(index) = remaining.find(marker) {
            let after = &remaining[index + marker.len()..];
            if let Some(name) = read_identifier(after) {
                if is_component_export_name(&name) {
                    return ExportReference::Named(name);
                }
            }
            remaining = after;
        }
    }

    if content.contains("export default") {
        return ExportReference::Default(fallback.to_string());
    }

    ExportReference::Named(fallback.to_string())
}

fn exported_block_candidate(content: &str) -> Option<String> {
    let mut remaining = content;
    while let Some(index) = remaining.find("export {") {
        let after = &remaining[index + "export {".len()..];
        let Some(end) = after.find('}') else {
            return None;
        };
        let exports = &after[..end];
        for export in exports.split(',') {
            let candidate = export_name_candidate(export);
            if is_component_export_name(&candidate) {
                return Some(candidate);
            }
        }
        remaining = &after[end + 1..];
    }

    None
}

fn export_name_candidate(export: &str) -> String {
    let export = export.trim();
    let export = export.strip_prefix("type ").unwrap_or(export).trim();
    let name = export
        .split_once(" as ")
        .map(|(_, alias)| alias.trim())
        .unwrap_or(export);
    read_identifier(name).unwrap_or_default()
}

fn read_identifier(text: &str) -> Option<String> {
    let name = text
        .trim_start()
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect::<String>();
    if name.is_empty() { None } else { Some(name) }
}

fn is_component_export_name(name: &str) -> bool {
    let Some(first) = name.chars().next() else {
        return false;
    };

    first.is_ascii_uppercase()
        && !name.ends_with("Props")
        && !name.ends_with("Config")
        && !name.ends_with("Variants")
}

fn titleize_id(id: &str) -> String {
    id.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = first.to_uppercase().collect::<String>();
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn component_identifier(id: &str) -> String {
    let mut name = String::new();
    for segment in id
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|segment| !segment.is_empty())
    {
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            name.extend(first.to_uppercase());
            name.push_str(chars.as_str());
        }
    }

    if name.is_empty() {
        "ShadcnComponent".to_string()
    } else {
        name
    }
}

fn preview_url_for_item(item: &CatalogItem) -> String {
    match item.source {
        CatalogSource::ShadcnComponent => {
            format!("https://ui.shadcn.com/docs/components/{}", item.id.as_ref())
        }
        CatalogSource::ShadcnBlock => "https://ui.shadcn.com/blocks".to_string(),
        CatalogSource::MagicUi => {
            format!(
                "https://magicui.design/docs/components/{}",
                item.id.as_ref()
            )
        }
        CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst => {
            item.source_path.to_string()
        }
    }
}

fn item_source_available(item: &CatalogItem, payload: &DraggedShadcnAsset) -> bool {
    if matches!(
        item.source,
        CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst
    ) {
        return true;
    }

    payload.source_path.exists()
        || matches!(
            item.source,
            CatalogSource::ShadcnComponent | CatalogSource::ShadcnBlock
        ) && shadcn_manifest_path(item.id.as_ref()).is_file()
}

fn local_preview_url_for_item(item: &CatalogItem) -> Option<String> {
    if matches!(
        item.source,
        CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst
    ) {
        let preview_dir = repo_root().join("target").join("shadcn-previews");
        std_fs::create_dir_all(&preview_dir).ok()?;
        let preview_path = preview_dir.join(format!("{}.html", preview_file_stem(item)));
        let source_file = PathBuf::from(item.source_path.as_ref());
        let html = preview_html(item, &source_file, item.jsx.as_ref());
        std_fs::write(&preview_path, html).ok()?;
        return Url::from_file_path(preview_path)
            .ok()
            .map(|url| url.to_string());
    }

    let source_path = absolute_source_path_for_item(item);
    let source_file =
        example_source_path_for_item(item).or_else(|| source_file_for_preview(&source_path))?;
    let source = std_fs::read_to_string(&source_file).ok()?;
    let preview_dir = repo_root().join("target").join("shadcn-previews");
    std_fs::create_dir_all(&preview_dir).ok()?;
    let preview_path = preview_dir.join(format!("{}.html", preview_file_stem(item)));
    let html = preview_html(item, &source_file, &source);
    std_fs::write(&preview_path, html).ok()?;
    Url::from_file_path(preview_path)
        .ok()
        .map(|url| url.to_string())
}

fn absolute_source_path_for_item(item: &CatalogItem) -> PathBuf {
    match item.source {
        CatalogSource::MagicUi => magic_registry_root().join(item.source_path.as_ref()),
        CatalogSource::ShadcnComponent | CatalogSource::ShadcnBlock => {
            shadcn_registry_root().join(item.source_path.as_ref())
        }
        CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst => PathBuf::new(),
    }
}

fn source_file_for_preview(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    if !path.is_dir() {
        return None;
    }

    for candidate in ["page.tsx", "index.tsx", "index.ts", "component.tsx"] {
        let path = path.join(candidate);
        if path.is_file() {
            return Some(path);
        }
    }

    first_source_file(path)
}

fn example_source_path_for_item(item: &CatalogItem) -> Option<PathBuf> {
    if item.source != CatalogSource::ShadcnComponent {
        return None;
    }

    let root = shadcn_examples_root();
    [
        format!("{}-demo.tsx", item.id.as_ref()),
        format!("{}-default.tsx", item.id.as_ref()),
        format!("{}.tsx", item.id.as_ref()),
    ]
    .into_iter()
    .map(|file_name| root.join(file_name))
    .find(|path| path.is_file())
}

fn first_source_file(path: &Path) -> Option<PathBuf> {
    let mut entries = std_fs::read_dir(path)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();

    for entry in &entries {
        if is_preview_source_file(entry) {
            return Some(entry.clone());
        }
    }
    for entry in entries {
        if entry.is_dir()
            && let Some(source) = first_source_file(&entry)
        {
            return Some(source);
        }
    }

    None
}

fn is_preview_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("tsx" | "ts" | "jsx" | "js" | "css")
    )
}

fn preview_file_stem(item: &CatalogItem) -> String {
    let source = match item.source {
        CatalogSource::ShadcnComponent => "component",
        CatalogSource::ShadcnBlock => "block",
        CatalogSource::MagicUi => "magic",
        CatalogSource::CommunityRegistry => "registry",
        CatalogSource::TwentyFirst => "twenty-first",
    };
    let id = item
        .id
        .as_ref()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("{source}-{id}")
}

fn preview_html(item: &CatalogItem, source_file: &Path, source: &str) -> String {
    let docs_url = preview_url_for_item(item);
    let source_path = source_file.to_string_lossy().replace('\\', "/");
    let kind = match item.source {
        CatalogSource::ShadcnComponent => "UI component",
        CatalogSource::ShadcnBlock => "shadcn block",
        CatalogSource::MagicUi => "Magic UI component",
        CatalogSource::CommunityRegistry => "shadcn registry",
        CatalogSource::TwentyFirst => "21st.dev Magic source",
    };
    let highlighted_source = highlight_tsx(source);

    format!(
        r#"<!doctype html>
<html lang="en" class="dark">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title} - Zed shadcn preview</title>
  <style>
    :root {{
      color-scheme: dark;
      --bg: #09090b;
      --panel: #101113;
      --panel-2: #15171a;
      --border: #272a2f;
      --fg: #f4f4f5;
      --muted: #a1a1aa;
      --accent: #3fb950;
      --accent-soft: rgba(63, 185, 80, .16);
      --code: #0c0d0f;
      --keyword: #ff7b72;
      --string: #a5d6ff;
      --function: #d2a8ff;
      --type: #ffa657;
      --comment: #8b949e;
      --punctuation: #79c0ff;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      background: var(--bg);
      color: var(--fg);
      font: 13px/1.5 Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }}
    main {{
      min-height: 100vh;
      padding: 24px;
      display: grid;
      gap: 16px;
      grid-template-columns: minmax(280px, 380px) minmax(0, 1fr);
    }}
    header, section {{
      border: 1px solid var(--border);
      background: var(--panel);
      border-radius: 8px;
    }}
    header {{ padding: 18px; }}
    section {{ overflow: hidden; }}
    h1 {{ margin: 0 0 6px; font-size: 22px; letter-spacing: 0; }}
    h2 {{ margin: 0; padding: 12px 14px; border-bottom: 1px solid var(--border); font-size: 13px; color: var(--muted); font-weight: 500; }}
    p {{ margin: 0; color: var(--muted); }}
    a {{ color: var(--accent); text-decoration: none; }}
    .stack {{ display: grid; gap: 12px; }}
    .pill {{
      display: inline-flex;
      align-items: center;
      width: max-content;
      border: 1px solid var(--border);
      border-radius: 999px;
      padding: 2px 8px;
      color: var(--accent);
      background: var(--accent-soft);
      font-size: 12px;
      margin-bottom: 12px;
    }}
    .meta {{
      display: grid;
      gap: 8px;
      padding: 14px;
      background: var(--panel-2);
      border-top: 1px solid var(--border);
    }}
    .row {{ display: grid; gap: 2px; }}
    .label {{ color: var(--muted); font-size: 11px; text-transform: uppercase; letter-spacing: .06em; }}
    code, pre {{
      font-family: "Zed Mono", "SFMono-Regular", Consolas, monospace;
      letter-spacing: 0;
    }}
    .source-scrollarea {{
      max-height: calc(100vh - 112px);
      overflow: auto;
      background: var(--code);
      border-radius: 0 0 8px 8px;
      scrollbar-color: var(--border) transparent;
      scrollbar-width: thin;
    }}
    .source-scrollarea::-webkit-scrollbar {{ width: 10px; height: 10px; }}
    .source-scrollarea::-webkit-scrollbar-thumb {{
      background: var(--border);
      border: 3px solid var(--code);
      border-radius: 999px;
    }}
    .source-scrollarea::-webkit-scrollbar-track {{ background: transparent; }}
    pre {{
      margin: 0;
      padding: 16px;
      background: var(--code);
      color: #d4d4d8;
      min-height: 280px;
      white-space: pre;
      tab-size: 2;
    }}
    .kw {{ color: var(--keyword); }}
    .str {{ color: var(--string); }}
    .fn {{ color: var(--function); }}
    .ty {{ color: var(--type); }}
    .cm {{ color: var(--comment); font-style: italic; }}
    .pn {{ color: var(--punctuation); }}
    .snippet {{
      padding: 14px;
      background: var(--code);
      border-top: 1px solid var(--border);
      overflow: auto;
    }}
    .demo {{
      padding: 18px;
      display: grid;
      place-items: center;
      min-height: 220px;
      background:
        linear-gradient(rgba(255,255,255,.025) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255,255,255,.025) 1px, transparent 1px),
        var(--panel-2);
      background-size: 24px 24px;
    }}
    .demo-surface {{
      width: min(100%, 440px);
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--panel);
      padding: 16px;
      box-shadow: 0 16px 48px rgba(0,0,0,.28);
    }}
    .demo-screenshot-shell {{
      width: min(100%, 620px);
      padding: 0;
      overflow: hidden;
    }}
    .demo-screenshot {{
      display: block;
      width: 100%;
      max-height: 420px;
      object-fit: contain;
      background: var(--code);
    }}
    .demo-row {{ display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }}
    .demo-stack {{ display: grid; gap: 12px; }}
    .demo-button {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-height: 32px;
      border: 1px solid var(--accent);
      border-radius: 6px;
      padding: 0 12px;
      color: #04130a;
      background: var(--accent);
      font-weight: 600;
    }}
    .demo-button.secondary {{ color: var(--fg); background: transparent; border-color: var(--border); }}
    .demo-input {{
      width: 100%;
      height: 34px;
      border: 1px solid var(--border);
      border-radius: 6px;
      background: var(--code);
      color: var(--fg);
      padding: 0 10px;
    }}
    .demo-badge {{
      display: inline-flex;
      width: max-content;
      border-radius: 999px;
      padding: 2px 8px;
      background: var(--accent-soft);
      color: var(--accent);
      border: 1px solid rgba(63, 185, 80, .42);
    }}
    .demo-table {{ width: 100%; border-collapse: collapse; }}
    .demo-table td, .demo-table th {{ padding: 8px; border-bottom: 1px solid var(--border); text-align: left; }}
    .demo-muted {{ color: var(--muted); }}
    .demo-chart {{ height: 120px; display: flex; align-items: end; gap: 10px; }}
    .demo-bar {{ flex: 1; border-radius: 6px 6px 0 0; background: var(--accent); min-height: 28px; opacity: .92; }}
    .demo-avatar {{ width: 42px; height: 42px; border-radius: 999px; background: var(--accent-soft); color: var(--accent); display: grid; place-items: center; border: 1px solid rgba(63, 185, 80, .42); font-weight: 700; }}
    @media (max-width: 860px) {{
      main {{ grid-template-columns: 1fr; padding: 16px; }}
      pre {{ max-height: none; }}
    }}
  </style>
</head>
<body>
  <main>
    <div class="stack">
      <header>
        <div class="pill">{kind}</div>
        <h1>{title}</h1>
        <p>{description}</p>
      </header>
      <section>
        <h2>Preview</h2>
        <div class="demo">{demo}</div>
      </section>
      <section>
        <h2>Insert Snippet</h2>
        <div class="snippet"><code>{jsx}</code></div>
        <div class="meta">
          <div class="row"><span class="label">Import</span><code>{import_statement}</code></div>
          <div class="row"><span class="label">Source</span><code>{source_path}</code></div>
          <div class="row"><span class="label">Docs</span><a href="{docs_url}">{docs_url}</a></div>
        </div>
      </section>
    </div>
    <section>
      <h2>Source</h2>
      <div class="source-scrollarea" data-shadcn-component="ScrollArea">
        <pre><code>{source}</code></pre>
      </div>
    </section>
  </main>
</body>
</html>
"#,
        kind = escape_html(kind),
        title = escape_html(item.title.as_ref()),
        description = escape_html(item.description.as_ref()),
        jsx = escape_html(item.jsx.as_ref()),
        import_statement = escape_html(item.import_statement.as_ref()),
        source_path = escape_html(&source_path),
        docs_url = escape_attr(&docs_url),
        demo = component_demo_html(item),
        source = highlighted_source,
    )
}

fn component_demo_html(item: &CatalogItem) -> String {
    if let Some(image_url) = shadcn_preview_image_url(item) {
        return format!(
            r#"<div class="demo-surface demo-screenshot-shell">
  <img class="demo-screenshot" src="{}" alt="{} preview" />
</div>"#,
            escape_attr(&image_url),
            escape_attr(item.title.as_ref())
        );
    }

    match item.source {
        CatalogSource::CommunityRegistry | CatalogSource::TwentyFirst => {
            return format!(
                r#"<div class="demo-surface demo-stack">
  <span class="demo-badge">{}</span>
  <strong>{}</strong>
  <p class="demo-muted">{}</p>
  <div class="demo-input">{}</div>
</div>"#,
                escape_html(item.category.as_ref()),
                escape_html(item.title.as_ref()),
                escape_html(item.description.as_ref()),
                escape_html(item.import_statement.as_ref())
            );
        }
        CatalogSource::ShadcnBlock => {
            return r#"<div class="demo-surface demo-stack">
  <div class="demo-row" style="justify-content: space-between">
    <div><strong>Dashboard</strong><div class="demo-muted">Revenue, traffic, and active users</div></div>
    <span class="demo-badge">Block</span>
  </div>
  <div class="demo-row">
    <div class="demo-surface" style="flex:1; box-shadow:none"><strong>12.8k</strong><div class="demo-muted">Visitors</div></div>
    <div class="demo-surface" style="flex:1; box-shadow:none"><strong>42%</strong><div class="demo-muted">Conversion</div></div>
  </div>
  <div class="demo-chart"><div class="demo-bar" style="height:42%"></div><div class="demo-bar" style="height:72%"></div><div class="demo-bar" style="height:56%"></div><div class="demo-bar" style="height:88%"></div></div>
</div>"#
                .to_string();
        }
        CatalogSource::MagicUi => {
            return format!(
                r#"<div class="demo-surface demo-stack">
  <span class="demo-badge">Magic UI</span>
  <strong>{}</strong>
  <p class="demo-muted">Animated component preview shell for source-backed install.</p>
  <div class="demo-button">Preview motion</div>
</div>"#,
                escape_html(item.title.as_ref())
            );
        }
        CatalogSource::ShadcnComponent => {}
    }

    match item.id.as_ref() {
        "button" => r#"<div class="demo-row"><button class="demo-button">Primary</button><button class="demo-button secondary">Secondary</button></div>"#.to_string(),
        "card" => r#"<div class="demo-surface demo-stack"><strong>Card title</strong><p class="demo-muted">Cards frame focused product content.</p><button class="demo-button">Continue</button></div>"#.to_string(),
        "input" => r#"<div class="demo-surface demo-stack"><label class="demo-muted">Email</label><input class="demo-input" value="hello@zed.dev"></div>"#.to_string(),
        "textarea" | "form" => r#"<div class="demo-surface demo-stack"><label class="demo-muted">Message</label><textarea class="demo-input" style="height:86px; padding-top:8px">Build with shadcn/ui inside Zed.</textarea><button class="demo-button">Submit</button></div>"#.to_string(),
        "badge" => r#"<div class="demo-row"><span class="demo-badge">Ready</span><span class="demo-badge">UI</span></div>"#.to_string(),
        "avatar" => r#"<div class="demo-row"><div class="demo-avatar">ZE</div><div><strong>Zed User</strong><div class="demo-muted">Design engineer</div></div></div>"#.to_string(),
        "tabs" => r#"<div class="demo-surface demo-stack"><div class="demo-row"><span class="demo-badge">Preview</span><span class="demo-muted">Code</span><span class="demo-muted">Install</span></div><p>Tabbed content keeps related workflows compact.</p></div>"#.to_string(),
        "table" => r#"<div class="demo-surface"><table class="demo-table"><tr><th>Name</th><th>Status</th></tr><tr><td>Button</td><td>Ready</td></tr><tr><td>Card</td><td>Installed</td></tr></table></div>"#.to_string(),
        "checkbox" => r#"<div class="demo-row"><span class="demo-button" style="width:24px; min-height:24px; padding:0">&#10003;</span><span>Enable component install</span></div>"#.to_string(),
        "select" => r#"<div class="demo-surface demo-stack"><label class="demo-muted">Theme</label><div class="demo-input">Dx Dark</div></div>"#.to_string(),
        "switch" | "toggle" | "toggle-group" => r#"<div class="demo-row"><span class="demo-muted">Preview mode</span><span class="demo-button" style="border-radius:999px; min-width:52px; padding:0 6px; justify-content:flex-end"><span style="width:18px;height:18px;border-radius:999px;background:#04130a;display:inline-block"></span></span></div>"#.to_string(),
        "slider" | "progress" => r#"<div class="demo-surface demo-stack"><strong>Progress</strong><div style="height:10px;border-radius:999px;background:var(--border);overflow:hidden"><div style="width:68%;height:100%;background:var(--accent)"></div></div></div>"#.to_string(),
        "calendar" | "date-picker" => r#"<div class="demo-surface demo-stack"><strong>May 2026</strong><div style="display:grid;grid-template-columns:repeat(7,1fr);gap:6px"><span class="demo-muted">M</span><span class="demo-muted">T</span><span class="demo-muted">W</span><span class="demo-muted">T</span><span class="demo-muted">F</span><span class="demo-muted">S</span><span class="demo-muted">S</span><span>11</span><span>12</span><span class="demo-badge">13</span><span>14</span><span>15</span><span>16</span><span>17</span></div></div>"#.to_string(),
        "dialog" => r#"<div class="demo-surface demo-stack"><strong>Dialog title</strong><p class="demo-muted">Modal content with focused actions.</p><div class="demo-row"><button class="demo-button">Confirm</button><button class="demo-button secondary">Cancel</button></div></div>"#.to_string(),
        "alert-dialog" => r#"<div class="demo-surface demo-stack"><strong>Confirm action</strong><p class="demo-muted">This keeps destructive flows explicit.</p><div class="demo-row"><button class="demo-button secondary">Cancel</button><button class="demo-button">Continue</button></div></div>"#.to_string(),
        "dropdown-menu" => r#"<div class="demo-surface demo-stack"><button class="demo-button secondary">Open menu</button><div class="demo-surface" style="box-shadow:none"><div>Copy</div><div>Edit</div><div>Delete</div></div></div>"#.to_string(),
        "context-menu" | "menubar" | "navigation-menu" => r#"<div class="demo-surface demo-stack"><div class="demo-row"><span class="demo-badge">File</span><span class="demo-muted">Edit</span><span class="demo-muted">View</span></div><div class="demo-surface" style="box-shadow:none"><div>New component</div><div>Copy import</div><div>Open docs</div></div></div>"#.to_string(),
        "accordion" => r#"<div class="demo-surface demo-stack"><strong>What is included?</strong><p class="demo-muted">Components, blocks, CSS variables, and dependencies.</p></div>"#.to_string(),
        "popover" | "tooltip" | "hover-card" => r#"<div class="demo-surface demo-stack"><button class="demo-button secondary">Hover target</button><div class="demo-surface" style="box-shadow:none"><strong>Preview detail</strong><p class="demo-muted">Context appears close to the focused control.</p></div></div>"#.to_string(),
        "toast" | "sonner" => r#"<div class="demo-surface demo-stack"><strong>Saved changes</strong><p class="demo-muted">The UI registry is ready.</p></div>"#.to_string(),
        "skeleton" => r#"<div class="demo-surface demo-stack"><div style="height:14px;width:70%;border-radius:999px;background:var(--border)"></div><div style="height:14px;width:90%;border-radius:999px;background:var(--border)"></div><div style="height:14px;width:48%;border-radius:999px;background:var(--border)"></div></div>"#.to_string(),
        "scroll-area" => r#"<div class="demo-surface demo-stack" style="max-height:150px;overflow:auto;scrollbar-color:var(--border) transparent"><p>ScrollArea keeps long component source readable.</p><p class="demo-muted">Install files, preview source, copy snippets, and review tokens without leaving Zed.</p><p class="demo-muted">The native panel uses compact rows and predictable tabs.</p></div>"#.to_string(),
        "chart" => r#"<div class="demo-surface demo-stack"><strong>Usage</strong><div class="demo-chart"><div class="demo-bar" style="height:60%"></div><div class="demo-bar" style="height:88%"></div><div class="demo-bar" style="height:48%"></div><div class="demo-bar" style="height:78%"></div></div></div>"#.to_string(),
        "sidebar" => r#"<div class="demo-surface demo-row"><div style="width:96px; border-right:1px solid var(--border); padding-right:12px" class="demo-stack"><strong>App</strong><span class="demo-muted">Home</span><span class="demo-muted">Files</span></div><div class="demo-stack"><strong>Workspace</strong><p class="demo-muted">Content area</p></div></div>"#.to_string(),
        _ => format!(
            r#"<div class="demo-surface demo-stack"><span class="demo-badge">{}</span><strong>{}</strong><p class="demo-muted">{}</p></div>"#,
            escape_html(item.category.as_ref()),
            escape_html(item.title.as_ref()),
            escape_html(item.description.as_ref())
        ),
    }
}

fn shadcn_preview_image_url(item: &CatalogItem) -> Option<String> {
    let public_root = repo_root()
        .join("inspirations")
        .join("shadcn-ui")
        .join("apps")
        .join("v4")
        .join("public");
    let id = item.id.as_ref();
    let mut candidates = Vec::with_capacity(16);

    match id {
        "authentication" => {
            candidates.extend([
                public_root.join("examples/authentication-dark.png"),
                public_root.join("examples/authentication-light.png"),
            ]);
        }
        "dashboard-01" | "dashboard" => {
            candidates.extend([
                public_root.join("r/styles/new-york-v4/dashboard-01-dark.png"),
                public_root.join("r/styles/new-york/dashboard-01-dark.png"),
                public_root.join("examples/dashboard-dark.png"),
            ]);
        }
        "login-01" | "login" => {
            candidates.extend([
                public_root.join("r/styles/new-york-v4/login-01-dark.png"),
                public_root.join("r/styles/new-york/login-01-dark.png"),
                public_root.join("examples/authentication-dark.png"),
            ]);
        }
        "button" | "button-01" => {
            candidates.extend([
                public_root.join("r/styles/new-york-v4/button-01-dark.png"),
                public_root.join("r/styles/new-york/button-01-dark.png"),
            ]);
        }
        "card" | "cards" => {
            candidates.extend([
                public_root.join("examples/cards-dark.png"),
                public_root.join("examples/cards-light.png"),
            ]);
        }
        "sidebar" => {
            candidates.extend([
                public_root.join("images/sidebar-menu-dark.png"),
                public_root.join("images/sidebar-structure-dark.png"),
            ]);
        }
        "calendar" | "date-picker" => {
            candidates.push(public_root.join("images/calendar-2.png"));
        }
        "playground" => {
            candidates.push(public_root.join("examples/playground-dark.png"));
        }
        "tasks" | "task-list" => {
            candidates.extend([
                public_root.join("examples/tasks-dark.png"),
                public_root.join("examples/tasks-light.png"),
            ]);
        }
        _ => {}
    }

    candidates.extend([
        public_root.join(format!("r/styles/new-york-v4/{id}-dark.png")),
        public_root.join(format!("r/styles/new-york/{id}-dark.png")),
        public_root.join(format!("examples/{id}-dark.png")),
        public_root.join(format!("images/{id}-dark.png")),
        public_root.join(format!("images/{id}.png")),
    ]);

    if id.starts_with("sidebar-") {
        candidates.extend([
            public_root.join(format!("r/styles/new-york-v4/{id}-dark.png")),
            public_root.join(format!("r/styles/new-york/{id}-dark.png")),
            public_root.join("images/sidebar-menu-dark.png"),
        ]);
    }

    if id.starts_with("login-") {
        candidates.extend([
            public_root.join(format!("r/styles/new-york-v4/{id}-dark.png")),
            public_root.join(format!("r/styles/new-york/{id}-dark.png")),
        ]);
    }

    if id.starts_with("dashboard-") {
        candidates.extend([
            public_root.join(format!("r/styles/new-york-v4/{id}-dark.png")),
            public_root.join(format!("r/styles/new-york/{id}-dark.png")),
        ]);
    }

    let mut seen_candidates = HashSet::with_capacity(candidates.len());
    candidates
        .into_iter()
        .filter(|path| seen_candidates.insert(path.clone()))
        .find(|path| path.is_file())
        .and_then(|path| Url::from_file_path(path).ok())
        .map(|url| url.to_string())
}

fn cached_shadcn_preview_image_urls(items: &[CatalogItem]) -> HashMap<String, Option<String>> {
    let cache = SHADCN_PREVIEW_IMAGE_CACHE.get_or_init(|| Mutex::new(HashMap::default()));
    let Ok(cache) = cache.lock() else {
        return HashMap::default();
    };

    let mut cached = HashMap::with_capacity(items.len().min(cache.len()));
    for item in items {
        if let Some(image_url) = cache.get(item.id.as_ref()) {
            cached.insert(item.id.to_string(), image_url.clone());
        }
    }
    cached
}

fn insert_preview_image_cache(key: String, image_url: Option<String>) {
    let cache = SHADCN_PREVIEW_IMAGE_CACHE.get_or_init(|| Mutex::new(HashMap::default()));
    if let Ok(mut cache) = cache.lock() {
        cache.insert(key, image_url);
    }
}

fn warm_shadcn_preview_images(items: Vec<CatalogItem>) -> Vec<(String, Option<String>)> {
    let mut warmed = Vec::with_capacity(items.len());
    for item in items {
        let key = item.id.to_string();
        let image_url = shadcn_preview_image_url(&item);
        warmed.push((key, image_url));
    }
    warmed
}

fn highlight_tsx(source: &str) -> String {
    let keywords = [
        "as",
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "from",
        "function",
        "if",
        "import",
        "in",
        "interface",
        "let",
        "new",
        "null",
        "of",
        "return",
        "satisfies",
        "switch",
        "throw",
        "true",
        "try",
        "type",
        "typeof",
        "undefined",
        "use",
        "var",
        "while",
    ];
    let types = [
        "Array",
        "Boolean",
        "Element",
        "HTMLDivElement",
        "MouseEvent",
        "Node",
        "Promise",
        "React",
        "ReactNode",
        "Record",
        "Set",
        "String",
    ];

    let mut output = String::new();
    for line in source.lines() {
        let rest = line;
        if let Some(comment_index) = rest.find("//") {
            let (code, comment) = rest.split_at(comment_index);
            output.push_str(&highlight_tsx_code(code, &keywords, &types));
            output.push_str(r#"<span class="cm">"#);
            output.push_str(&escape_html(comment));
            output.push_str("</span>\n");
            continue;
        }

        output.push_str(&highlight_tsx_code(rest, &keywords, &types));
        output.push('\n');
    }
    output
}

fn highlight_tsx_code(source: &str, keywords: &[&str], types: &[&str]) -> String {
    let mut output = String::new();
    let mut chars = source.char_indices().peekable();
    while let Some((start, character)) = chars.next() {
        if character == '"' || character == '\'' || character == '`' {
            let quote = character;
            let mut end = start + character.len_utf8();
            let mut escaped = false;
            while let Some((index, next)) = chars.next() {
                end = index + next.len_utf8();
                if escaped {
                    escaped = false;
                } else if next == '\\' {
                    escaped = true;
                } else if next == quote {
                    break;
                }
            }
            output.push_str(r#"<span class="str">"#);
            output.push_str(&escape_html(&source[start..end]));
            output.push_str("</span>");
            continue;
        }

        if character.is_ascii_alphabetic() || character == '_' {
            let mut end = start + character.len_utf8();
            while let Some((index, next)) = chars.peek().copied() {
                if next.is_ascii_alphanumeric() || next == '_' {
                    chars.next();
                    end = index + next.len_utf8();
                } else {
                    break;
                }
            }
            let word = &source[start..end];
            if keywords.contains(&word) {
                output.push_str(r#"<span class="kw">"#);
                output.push_str(word);
                output.push_str("</span>");
            } else if types.contains(&word)
                || word
                    .chars()
                    .next()
                    .is_some_and(|first| first.is_ascii_uppercase())
            {
                output.push_str(r#"<span class="ty">"#);
                output.push_str(word);
                output.push_str("</span>");
            } else {
                output.push_str(&escape_html(word));
            }
            continue;
        }

        if matches!(
            character,
            '<' | '>' | '{' | '}' | '(' | ')' | '[' | ']' | ':' | '=' | '/'
        ) {
            output.push_str(r#"<span class="pn">"#);
            output.push_str(&escape_html(&character.to_string()));
            output.push_str("</span>");
        } else {
            output.push_str(&escape_html(&character.to_string()));
        }
    }
    output
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn escape_attr(text: &str) -> String {
    escape_html(text)
}
