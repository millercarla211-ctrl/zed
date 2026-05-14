use editor::{Editor, EditorEvent};
use gpui::{
    AnyElement, App, AppContext as _, AsyncWindowContext, Context, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, Pixels, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Subscription, WeakEntity, Window, actions, div, point, px,
};
use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::OnceLock,
};
use strum::IntoEnumIterator;
use ui::{TintColor, Tooltip, prelude::*};
use workspace::{
    DraggedIconAsset, Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

actions!(
    icon_picker,
    [
        /// Toggles the icon picker panel.
        Toggle,
        /// Toggles focus on the icon picker panel.
        ToggleFocus,
    ]
);

const ICON_PICKER_PANEL_KEY: &str = "IconPickerPanel";
const DX_ICON_DATA_DIR: &str = "G:/Assets/icon/data";
const ICON_PACK_INDEX: &str = include_str!("icon_pack_index.tsv");
const ICON_REPRESENTATIVE_BODIES: &str = include_str!("icon_representative_bodies.tsv");
const MAX_ICON_RESULTS: usize = 360;
const STARTUP_ICON_PREVIEW_WARM_LIMIT: usize = MAX_ICON_RESULTS;
const EXTERNAL_ICON_PREVIEW_CACHE_VERSION: &str = "v3";
static EXTERNAL_ICON_CATALOG_CACHE: OnceLock<ExternalIconCatalog> = OnceLock::new();
static REPRESENTATIVE_ICON_BODY_CACHE: OnceLock<HashMap<String, ExternalIconBody>> =
    OnceLock::new();

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace.register_action(|workspace, _: &ToggleFocus, window, cx| {
            workspace.toggle_panel_focus::<IconPickerPanel>(window, cx);
        });
        workspace.register_action(|workspace, _: &Toggle, window, cx| {
            if !workspace.toggle_panel_focus::<IconPickerPanel>(window, cx) {
                workspace.close_panel::<IconPickerPanel>(window, cx);
            }
        });
    })
    .detach();
}

#[derive(Clone)]
enum PickerIcon {
    Zed(IconName),
    External(ExternalIcon),
}

impl PickerIcon {
    fn id(&self) -> String {
        match self {
            Self::Zed(icon_name) => {
                let stem: &'static str = icon_name.into();
                format!("zed:{stem}")
            }
            Self::External(icon) => icon.id(),
        }
    }
}

#[derive(Clone)]
struct ExternalIcon {
    pack: SharedString,
    pack_name: SharedString,
    name: SharedString,
    label: SharedString,
    stem: SharedString,
    width: u32,
    height: u32,
    search_text: SharedString,
}

impl ExternalIcon {
    fn id(&self) -> String {
        format!("{}:{}", self.pack.as_ref(), self.name.as_ref())
    }
}

#[derive(Clone)]
struct IconPackSummary {
    prefix: SharedString,
    name: SharedString,
    total: usize,
    width: u32,
    height: u32,
    sample_names: Vec<SharedString>,
}

#[derive(Clone)]
struct ExternalSvg {
    preview_path: SharedString,
}

pub struct IconPickerPanel {
    workspace: WeakEntity<Workspace>,
    filter_editor: Entity<Editor>,
    zed_icons: Vec<IconName>,
    zed_icon_search_text_cache: RefCell<HashMap<&'static str, SharedString>>,
    external_icons: Vec<ExternalIcon>,
    external_icons_by_pack: HashMap<String, Vec<ExternalIcon>>,
    representative_external_icons: Vec<ExternalIcon>,
    packs: Vec<IconPackSummary>,
    selected_pack: Option<SharedString>,
    selected_icon: Option<PickerIcon>,
    loading_external_icons: bool,
    external_catalog_loaded: bool,
    preview_cache: RefCell<HashMap<String, Option<ExternalSvg>>>,
    warming_preview_keys: HashSet<String>,
    warming_preview_signature: Option<SharedString>,
    warmed_preview_signatures: HashSet<String>,
    representative_preview_warm_started: bool,
    pack_scroll_handle: ScrollHandle,
    status: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl IconPickerPanel {
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
                editor.set_placeholder_text("Search icons or packs...", window, cx);
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

            let zed_icons = IconName::iter().collect::<Vec<_>>();
            let selected_icon = zed_icons.first().copied().map(PickerIcon::Zed);
            let packs = static_icon_pack_summaries();
            let representative_external_icons = representative_icons_from_pack_summaries(&packs);
            Self {
                workspace: workspace_handle,
                filter_editor,
                zed_icons,
                zed_icon_search_text_cache: RefCell::default(),
                external_icons: Vec::new(),
                external_icons_by_pack: HashMap::default(),
                representative_external_icons,
                packs,
                selected_pack: None,
                selected_icon,
                loading_external_icons: false,
                external_catalog_loaded: false,
                preview_cache: RefCell::default(),
                warming_preview_keys: HashSet::default(),
                warming_preview_signature: None,
                warmed_preview_signatures: HashSet::default(),
                representative_preview_warm_started: false,
                pack_scroll_handle: ScrollHandle::new(),
                status: None,
                _subscriptions: vec![filter_subscription],
            }
        })
    }

    fn ensure_representative_external_previews_warmed(&mut self, cx: &mut Context<Self>) {
        if self.representative_preview_warm_started {
            return;
        }
        self.representative_preview_warm_started = true;

        let external_icons = self
            .representative_external_icons
            .iter()
            .take(STARTUP_ICON_PREVIEW_WARM_LIMIT)
            .cloned()
            .collect::<Vec<_>>();
        self.queue_external_preview_warm(external_icons, false, cx);
    }

    fn ensure_icon_data_loaded_for_view(&mut self, query: &str, cx: &mut Context<Self>) {
        let selected_pack = self
            .selected_pack
            .as_ref()
            .map(|pack| pack.to_string())
            .filter(|pack| pack != "zed");

        if let Some(pack) = selected_pack {
            self.ensure_icon_pack_loaded(pack, cx);
        } else if !query.is_empty() {
            self.ensure_external_icon_catalog_loaded(cx);
        }
    }

    fn ensure_icon_pack_loaded(&mut self, pack: String, cx: &mut Context<Self>) {
        if self.external_icons_by_pack.contains_key(&pack) || self.loading_external_icons {
            return;
        }

        let Some(pack_summary) = self
            .packs
            .iter()
            .find(|summary| summary.prefix.as_ref() == pack.as_str())
            .cloned()
        else {
            return;
        };

        self.loading_external_icons = true;
        self.status = Some(format!("Loading {}", pack_summary.name.as_ref()).into());
        let executor = cx.background_executor().clone();
        cx.spawn(async move |panel, cx| {
            let icons = executor
                .spawn(async move { load_external_icon_pack_catalog(&pack_summary) })
                .await;
            panel
                .update(cx, |panel, cx| {
                    if !icons.is_empty() {
                        let pack = icons[0].pack.to_string();
                        panel
                            .external_icons_by_pack
                            .insert(pack.clone(), icons.clone());
                        panel.external_icons.extend(icons);
                        panel.external_icons.sort_by(|left, right| {
                            left.pack_name
                                .as_ref()
                                .cmp(right.pack_name.as_ref())
                                .then_with(|| left.name.as_ref().cmp(right.name.as_ref()))
                        });
                        panel.external_icons.dedup_by(|left, right| {
                            left.pack.as_ref() == right.pack.as_ref()
                                && left.name.as_ref() == right.name.as_ref()
                        });
                    }
                    panel.loading_external_icons = false;
                    panel.status = None;
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn ensure_external_icon_catalog_loaded(&mut self, cx: &mut Context<Self>) {
        if self.external_catalog_loaded || self.loading_external_icons {
            return;
        }

        self.loading_external_icons = true;
        self.status = Some("Loading external icon sets...".into());
        let executor = cx.background_executor().clone();
        cx.spawn(async move |panel, cx| {
            let catalog = executor
                .spawn(async move { load_external_icon_catalog_cached() })
                .await;
            panel
                .update(cx, |panel, cx| {
                    panel.external_icons = catalog.icons;
                    panel.external_icons_by_pack = catalog.icons_by_pack;
                    panel.representative_external_icons = catalog.representative_icons;
                    panel.packs = catalog.packs;
                    panel.loading_external_icons = false;
                    panel.external_catalog_loaded = true;
                    panel.status = None;
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn query(&self, cx: &App) -> String {
        self.filter_editor.read(cx).text(cx).trim().to_lowercase()
    }

    fn filtered_icons(&self, query: &str) -> (Vec<PickerIcon>, usize, usize) {
        let selected_pack = self.selected_pack.as_ref().map(|pack| pack.as_ref());
        let total_count = self.total_count_for_selection(selected_pack);
        let mut icons = Vec::with_capacity(MAX_ICON_RESULTS.min(total_count));
        let mut match_count = 0;

        if selected_pack.is_none() && query.is_empty() {
            icons.extend(
                self.representative_external_icons
                    .iter()
                    .take(MAX_ICON_RESULTS)
                    .cloned()
                    .map(PickerIcon::External),
            );
            if icons.len() < MAX_ICON_RESULTS {
                icons.extend(
                    self.zed_icons
                        .iter()
                        .take(MAX_ICON_RESULTS - icons.len())
                        .copied()
                        .map(PickerIcon::Zed),
                );
            }
            return (icons, MAX_ICON_RESULTS.min(total_count), total_count);
        }

        if query.is_empty() {
            if selected_pack == Some("zed") {
                icons.extend(self.zed_icons.iter().copied().map(PickerIcon::Zed));
                icons.truncate(MAX_ICON_RESULTS);
                return (icons, total_count, total_count);
            } else if let Some(selected_pack) = selected_pack
                && let Some(pack_icons) = self.external_icons_by_pack.get(selected_pack)
            {
                icons.extend(
                    pack_icons
                        .iter()
                        .take(MAX_ICON_RESULTS)
                        .cloned()
                        .map(PickerIcon::External),
                );
                return (icons, total_count, total_count);
            }
        }

        let query_terms = query.split_whitespace().collect::<Vec<_>>();

        if selected_pack.is_none() || selected_pack == Some("zed") {
            icons.extend(self.zed_icons.iter().copied().filter_map(|icon_name| {
                if !self.zed_icon_matches(icon_name, &query_terms) {
                    return None;
                }
                match_count += 1;
                Some(PickerIcon::Zed(icon_name))
            }));
        }

        if selected_pack != Some("zed") {
            if let Some(selected_pack) = selected_pack {
                if let Some(pack_icons) = self.external_icons_by_pack.get(selected_pack) {
                    for icon in pack_icons {
                        if icon_search_matches(icon.search_text.as_ref(), &query_terms) {
                            match_count += 1;
                            if icons.len() < MAX_ICON_RESULTS {
                                icons.push(PickerIcon::External(icon.clone()));
                            }
                        }
                    }
                }
            } else {
                for icon in &self.external_icons {
                    if icon_search_matches(icon.search_text.as_ref(), &query_terms) {
                        match_count += 1;
                        if icons.len() < MAX_ICON_RESULTS {
                            icons.push(PickerIcon::External(icon.clone()));
                        }
                    }
                }
            }
        }

        if icons.len() > MAX_ICON_RESULTS {
            icons.truncate(MAX_ICON_RESULTS);
        }
        (icons, match_count, total_count)
    }

    fn zed_icon_matches(&self, icon_name: IconName, query_terms: &[&str]) -> bool {
        let stem: &'static str = (&icon_name).into();
        if let Some(matches) = {
            let search_text_cache = self.zed_icon_search_text_cache.borrow();
            search_text_cache
                .get(stem)
                .map(|search_text| icon_search_matches(search_text.as_ref(), query_terms))
        } {
            return matches;
        }

        let payload = DraggedIconAsset::new(icon_name);
        let search_text: SharedString = format!(
            "{} {} zed",
            payload.stem.as_ref(),
            payload.label.as_ref().to_lowercase()
        )
        .into();
        self.zed_icon_search_text_cache
            .borrow_mut()
            .insert(stem, search_text.clone());
        icon_search_matches(search_text.as_ref(), query_terms)
    }

    fn payload_for_icon(&self, icon: &PickerIcon) -> DraggedIconAsset {
        match icon {
            PickerIcon::Zed(icon_name) => DraggedIconAsset::new(*icon_name),
            PickerIcon::External(icon) => DraggedIconAsset::from_iconify(
                icon.stem.clone(),
                icon.label.clone(),
                icon.pack.clone(),
                icon.name.clone(),
                icon.width,
                icon.height,
            ),
        }
    }

    fn cached_external_svg(&self, icon: &ExternalIcon) -> Option<ExternalSvg> {
        let key = icon.id();
        let cached_svg = self.preview_cache.borrow().get(&key).cloned();
        if let Some(svg) = cached_svg {
            return svg;
        }

        if self.warming_preview_keys.contains(&key) {
            return None;
        }

        if let Some(preview_path) = existing_external_icon_preview(icon) {
            let external_svg = ExternalSvg {
                preview_path: preview_path.into(),
            };
            self.preview_cache
                .borrow_mut()
                .insert(key, Some(external_svg.clone()));
            return Some(external_svg);
        }

        None
    }

    fn ensure_visible_external_previews_warmed(
        &mut self,
        icons: &[PickerIcon],
        cx: &mut Context<Self>,
    ) {
        let external_icons = {
            let preview_cache = self.preview_cache.borrow();
            icons
                .iter()
                .filter_map(|icon| match icon {
                    PickerIcon::External(icon) => {
                        let key = icon.id();
                        (!self.warming_preview_keys.contains(&key)
                            && !preview_cache.contains_key(&key))
                        .then(|| icon.clone())
                    }
                    PickerIcon::Zed(_) => None,
                })
                .collect::<Vec<_>>()
        };
        self.queue_external_preview_warm(external_icons, true, cx);
    }

    fn queue_external_preview_warm(
        &mut self,
        external_icons: Vec<ExternalIcon>,
        update_status: bool,
        cx: &mut Context<Self>,
    ) {
        let external_icons = self.uncached_external_icons(external_icons);

        if external_icons.is_empty() {
            return;
        }

        let signature = icon_preview_batch_signature(&external_icons);
        if self
            .warming_preview_signature
            .as_ref()
            .is_some_and(|current| current.as_ref() == signature.as_str())
            || self.warmed_preview_signatures.contains(&signature)
        {
            return;
        }

        for icon in &external_icons {
            self.warming_preview_keys.insert(icon.id());
        }
        self.warming_preview_signature = Some(signature.clone().into());
        if update_status && self.status.is_none() {
            self.status = Some("Preparing icon previews".into());
        }

        let executor = cx.background_executor().clone();
        cx.spawn(async move |panel, cx| {
            let previews = executor
                .spawn(async move { warm_external_icon_previews(external_icons) })
                .await;
            panel
                .update(cx, |panel, cx| {
                    {
                        let mut preview_cache = panel.preview_cache.borrow_mut();
                        for (key, preview_path) in previews {
                            panel.warming_preview_keys.remove(&key);
                            preview_cache.insert(
                                key,
                                preview_path.map(|preview_path| ExternalSvg { preview_path }),
                            );
                        }
                    }

                    panel.warmed_preview_signatures.insert(signature.clone());
                    if panel
                        .warming_preview_signature
                        .as_ref()
                        .is_some_and(|current| current.as_ref() == signature.as_str())
                    {
                        panel.warming_preview_signature = None;
                        if panel
                            .status
                            .as_ref()
                            .is_some_and(|status| status.as_ref() == "Preparing icon previews")
                        {
                            panel.status = None;
                        }
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn uncached_external_icons(&self, icons: Vec<ExternalIcon>) -> Vec<ExternalIcon> {
        let mut uncached_icons = Vec::new();
        let mut preview_cache = self.preview_cache.borrow_mut();
        for icon in icons {
            let key = icon.id();
            if self.warming_preview_keys.contains(&key) || preview_cache.contains_key(&key) {
                continue;
            }

            if let Some(preview_path) = existing_external_icon_preview(&icon) {
                preview_cache.insert(
                    key,
                    Some(ExternalSvg {
                        preview_path: preview_path.into(),
                    }),
                );
                continue;
            }

            uncached_icons.push(icon);
        }
        uncached_icons
    }

    fn insert_icon(&mut self, icon: PickerIcon, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_icon = Some(icon.clone());
        let payload = self.payload_for_icon(&icon);
        let Some(workspace) = self.workspace.upgrade() else {
            self.status = Some("No active workspace".into());
            cx.notify();
            return;
        };
        let Some(editor) = workspace.read(cx).active_item_as::<Editor>(cx) else {
            self.status = Some("Open an editor to insert the icon".into());
            cx.notify();
            return;
        };

        let result = editor.update(cx, |editor, cx| {
            editor.focus_handle(cx).focus(window, cx);
            editor.insert_icon_asset(&payload, window, cx)
        });

        self.status = match result {
            Ok(message) => Some(message),
            Err(error) => Some(format!("{error:#}").into()),
        };
        cx.notify();
    }

    fn render_pack_filters(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self
            .selected_pack
            .as_ref()
            .map(|pack| pack.as_ref())
            .unwrap_or("all");
        let mut pack_buttons = Vec::new();
        pack_buttons.push(
            self.render_pack_button(
                "all",
                "All sets",
                None,
                self.external_icon_total_count(),
                current,
                cx,
            )
            .into_any_element(),
        );
        pack_buttons.push(
            self.render_pack_button("zed", "Zed", None, self.zed_icons.len(), current, cx)
                .into_any_element(),
        );
        for pack in &self.packs {
            pack_buttons.push(
                self.render_pack_button(
                    pack.prefix.as_ref(),
                    pack.name.as_ref(),
                    Some(format!("{} ({})", pack.name.as_ref(), pack.prefix.as_ref()).into()),
                    pack.total,
                    current,
                    cx,
                )
                .into_any_element(),
            );
        }

        h_flex()
            .h(px(42.))
            .gap_1()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().colors().border.opacity(0.6))
            .px_1()
            .child(
                IconButton::new("icon-picker-pack-prev", IconName::ChevronLeft)
                    .shape(ui::IconButtonShape::Square)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Previous icon sets"))
                    .on_click(cx.listener(|panel, _, _, cx| {
                        panel.scroll_pack_tabs(-1.0, cx);
                    })),
            )
            .child(
                h_flex()
                    .id("icon-picker-pack-scroll")
                    .flex_1()
                    .h_full()
                    .overflow_x_scroll()
                    .overflow_y_hidden()
                    .track_scroll(&self.pack_scroll_handle)
                    .child(
                        h_flex()
                            .flex_none()
                            .gap_1()
                            .items_center()
                            .px_1()
                            .py_1()
                            .children(pack_buttons),
                    ),
            )
            .child(
                IconButton::new("icon-picker-pack-next", IconName::ChevronRight)
                    .shape(ui::IconButtonShape::Square)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Next icon sets"))
                    .on_click(cx.listener(|panel, _, _, cx| {
                        panel.scroll_pack_tabs(1.0, cx);
                    })),
            )
    }

    fn scroll_pack_tabs(&mut self, direction: f32, cx: &mut Context<Self>) {
        scroll_tab_handle(&self.pack_scroll_handle, direction);
        cx.notify();
    }

    fn render_pack_button(
        &self,
        id: &str,
        label: &str,
        tooltip: Option<SharedString>,
        count: usize,
        current: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = current == id;
        let id_string = id.to_string();
        let tooltip_label = tooltip.unwrap_or_else(|| label.to_string().into());
        div().flex_none().child(
            Button::new(format!("icon-picker-pack-{id}"), format!("{label} {count}"))
                .style(ButtonStyle::Subtle)
                .size(ButtonSize::Compact)
                .toggle_state(selected)
                .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                .tooltip(Tooltip::text(tooltip_label.to_string()))
                .on_click(cx.listener(move |panel, _, _, cx| {
                    panel.selected_pack = if id_string == "all" {
                        None
                    } else {
                        Some(id_string.clone().into())
                    };
                    panel.status = None;
                    cx.notify();
                })),
        )
    }

    fn render_icon_tile(&self, icon: PickerIcon, cx: &mut Context<Self>) -> impl IntoElement {
        let payload = self.payload_for_icon(&icon);
        let label = payload.label.clone();
        let selected = self
            .selected_icon
            .as_ref()
            .is_some_and(|selected| selected.id() == icon.id());
        let icon_preview = self.render_icon_preview(&icon, IconSize::Medium, cx);

        div()
            .id(format!("icon-picker-tile-{}", icon.id()))
            .min_w(px(0.))
            .h(px(44.))
            .p_0p5()
            .gap_1()
            .v_flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .border_1()
            .border_color(if selected {
                cx.theme().colors().border_focused
            } else {
                cx.theme().colors().border_variant
            })
            .bg(if selected {
                cx.theme().colors().element_selected
            } else {
                cx.theme().colors().element_background
            })
            .cursor_pointer()
            .hover(|style| style.bg(cx.theme().colors().element_hover))
            .tooltip(Tooltip::text(label.to_string()))
            .on_click(cx.listener({
                let icon = icon.clone();
                move |panel, _, window, cx| {
                    panel.insert_icon(icon.clone(), window, cx);
                }
            }))
            .on_drag(payload, |icon, position, _, cx| {
                cx.new(|_| IconDragPreview {
                    icon_name: icon.icon_name,
                    preview_path: icon.preview_path.clone(),
                    label: icon.label.clone(),
                    position,
                })
            })
            .child(icon_preview)
            .child(
                Label::new(label)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
    }

    fn render_icon_preview(
        &self,
        icon: &PickerIcon,
        size: IconSize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match icon {
            PickerIcon::Zed(icon_name) => Icon::new(*icon_name).size(size).into_any_element(),
            PickerIcon::External(icon) => self
                .cached_external_svg(icon)
                .map(|svg| {
                    Icon::from_external_svg(svg.preview_path)
                        .size(size)
                        .into_any_element()
                })
                .unwrap_or_else(|| external_pack_badge(icon, cx)),
        }
    }

    fn external_icon_total_count(&self) -> usize {
        if self.external_catalog_loaded {
            self.external_icons.len()
        } else {
            self.packs.iter().map(|pack| pack.total).sum()
        }
    }

    fn total_count_for_selection(&self, selected_pack: Option<&str>) -> usize {
        match selected_pack {
            Some("zed") => self.zed_icons.len(),
            Some(pack) => self
                .packs
                .iter()
                .find(|summary| summary.prefix.as_ref() == pack)
                .map(|summary| summary.total)
                .unwrap_or(0),
            None => self.zed_icons.len() + self.external_icon_total_count(),
        }
    }
}

impl Panel for IconPickerPanel {
    fn persistent_name() -> &'static str {
        "Icon Picker"
    }

    fn panel_key() -> &'static str {
        ICON_PICKER_PANEL_KEY
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
        Some("Icon Picker")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(ToggleFocus)
    }

    fn activation_priority(&self) -> u32 {
        8
    }
}

impl Focusable for IconPickerPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.filter_editor.focus_handle(cx)
    }
}

impl EventEmitter<PanelEvent> for IconPickerPanel {}

impl Render for IconPickerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_representative_external_previews_warmed(cx);
        let query = self.query(cx);
        self.ensure_icon_data_loaded_for_view(query.as_str(), cx);
        let (icons, total_matches, total_count) = self.filtered_icons(query.as_str());
        self.ensure_visible_external_previews_warmed(&icons, cx);
        let is_empty = icons.is_empty();
        let shown_count = icons.len();
        let count_label = self.status.clone().unwrap_or_else(|| {
            if self.loading_external_icons {
                "loading icons".into()
            } else if query.is_empty() {
                format!("{shown_count} / {total_count}").into()
            } else {
                format!("{total_matches} / {total_count}").into()
            }
        });
        let icon_tiles = icons
            .into_iter()
            .map(|icon| self.render_icon_tile(icon, cx).into_any_element())
            .collect::<Vec<_>>();

        v_flex()
            .id("icon-picker-panel")
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
                            .child(Label::new("Icons").size(LabelSize::Small))
                            .child(
                                Label::new(count_label)
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted)
                                    .truncate(),
                            ),
                    )
                    .child(self.filter_editor.clone()),
            )
            .child(self.render_pack_filters(cx))
            .child(
                div()
                    .image_cache(gpui::retain_all("icon-picker-icons"))
                    .id("icon-picker-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .p_2()
                    .when(is_empty, |this| {
                        this.child(
                            div().h_full().flex().items_center().justify_center().child(
                                Label::new(if self.loading_external_icons {
                                    "Loading icons"
                                } else {
                                    "No matching icons"
                                })
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                            ),
                        )
                    })
                    .when(!is_empty, |this| {
                        this.child(div().grid().grid_cols(10).gap_1().children(icon_tiles))
                    }),
            )
    }
}

fn scroll_tab_handle(handle: &ScrollHandle, direction: f32) {
    let current = handle.offset();
    let max = handle.max_offset();
    let mut next_x = current.x - px(direction * 180.0);
    let min_x = -max.x;
    if next_x < min_x {
        next_x = min_x;
    }
    if next_x > px(0.) {
        next_x = px(0.);
    }
    handle.set_offset(point(next_x, current.y));
}

fn icon_search_matches(searchable: &str, query_terms: &[&str]) -> bool {
    query_terms.iter().all(|term| searchable.contains(term))
}

struct IconDragPreview {
    icon_name: Option<IconName>,
    preview_path: Option<SharedString>,
    label: SharedString,
    position: gpui::Point<Pixels>,
}

impl Render for IconDragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let icon = self
            .preview_path
            .clone()
            .map(|path| {
                Icon::from_external_svg(path)
                    .size(IconSize::Small)
                    .into_any_element()
            })
            .or_else(|| {
                self.icon_name.map(|icon_name| {
                    Icon::new(icon_name)
                        .size(IconSize::Small)
                        .into_any_element()
                })
            })
            .unwrap_or_else(|| {
                Icon::new(IconName::SquareDot)
                    .size(IconSize::Small)
                    .into_any_element()
            });

        div()
            .absolute()
            .left(self.position.x - px(40.))
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
                    .child(icon)
                    .child(Label::new(self.label.clone()).size(LabelSize::XSmall)),
            )
    }
}

fn external_pack_badge(icon: &ExternalIcon, cx: &mut Context<IconPickerPanel>) -> AnyElement {
    div()
        .w(px(22.))
        .h(px(22.))
        .flex_none()
        .rounded_sm()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .bg(cx.theme().colors().elevated_surface_background)
        .flex()
        .items_center()
        .justify_center()
        .child(
            Label::new(external_pack_initials(icon.pack.as_ref()))
                .size(LabelSize::XSmall)
                .color(Color::Muted),
        )
        .into_any_element()
}

fn external_pack_initials(pack: &str) -> String {
    let mut initials = String::new();
    for part in pack.split(['-', '_', ' ']) {
        if let Some(character) = part
            .chars()
            .find(|character| character.is_ascii_alphanumeric())
        {
            initials.push(character.to_ascii_uppercase());
            if initials.len() == 2 {
                break;
            }
        }
    }

    if initials.is_empty() {
        "I".to_string()
    } else {
        initials
    }
}

#[derive(Clone)]
struct ExternalIconCatalog {
    icons: Vec<ExternalIcon>,
    icons_by_pack: HashMap<String, Vec<ExternalIcon>>,
    representative_icons: Vec<ExternalIcon>,
    packs: Vec<IconPackSummary>,
}

#[derive(Deserialize)]
struct IconifyPack {
    icons: HashMap<String, IconifyCatalogIcon>,
    #[serde(default)]
    aliases: HashMap<String, IconifyAlias>,
}

#[derive(Deserialize)]
struct IconifyCatalogIcon {
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}

#[derive(Deserialize)]
struct IconifyAlias {
    parent: String,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default, rename = "hFlip")]
    h_flip: bool,
    #[serde(default, rename = "vFlip")]
    v_flip: bool,
    #[serde(default)]
    rotate: Option<u8>,
}

#[derive(Clone)]
struct ExternalIconBody {
    body: String,
    width: Option<u32>,
    height: Option<u32>,
    h_flip: bool,
    v_flip: bool,
    rotate: Option<u8>,
}

#[derive(Deserialize)]
struct IconifyBodyPack {
    icons: HashMap<String, IconifyBody>,
    #[serde(default)]
    aliases: HashMap<String, IconifyAlias>,
}

#[derive(Deserialize)]
struct IconifyBody {
    body: String,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}

fn load_external_icon_catalog_cached() -> ExternalIconCatalog {
    EXTERNAL_ICON_CATALOG_CACHE
        .get_or_init(load_external_icon_catalog)
        .clone()
}

fn static_icon_pack_summaries() -> Vec<IconPackSummary> {
    ICON_PACK_INDEX
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut columns = line.split('\t');
            let prefix = columns.next()?;
            let name = columns.next()?;
            let total = columns.next()?.parse::<usize>().ok()?;
            let width = columns.next()?.parse::<u32>().ok()?.max(1);
            let height = columns.next()?.parse::<u32>().ok()?.max(1);
            let sample_names = columns
                .filter(|name| !name.is_empty())
                .map(SharedString::from)
                .collect::<Vec<_>>();
            Some(IconPackSummary {
                prefix: prefix.into(),
                name: name.into(),
                total,
                width,
                height,
                sample_names,
            })
        })
        .collect()
}

fn representative_icons_from_pack_summaries(packs: &[IconPackSummary]) -> Vec<ExternalIcon> {
    let mut icons = Vec::with_capacity(MAX_ICON_RESULTS.min(packs.len().saturating_mul(2)));
    for index in 0..2 {
        for pack in packs {
            let Some(name) = pack.sample_names.get(index) else {
                continue;
            };
            icons.push(external_icon_from_summary(
                pack,
                name.as_ref(),
                pack.width,
                pack.height,
            ));
            if icons.len() >= MAX_ICON_RESULTS {
                return icons;
            }
        }
    }
    icons
}

fn load_external_icon_catalog() -> ExternalIconCatalog {
    let data_dir = external_icon_data_dir();
    let packs = static_icon_pack_summaries();
    if !data_dir.is_dir() {
        return ExternalIconCatalog {
            icons: Vec::new(),
            icons_by_pack: HashMap::with_capacity(packs.len()),
            representative_icons: Vec::new(),
            packs,
        };
    }

    let total_icon_count = packs.iter().map(|pack| pack.total).sum::<usize>();
    let mut icons = Vec::with_capacity(total_icon_count);
    let mut icons_by_pack = HashMap::<String, Vec<ExternalIcon>>::with_capacity(packs.len());
    for pack_summary in &packs {
        let pack_icons = load_external_icon_pack_catalog(pack_summary);
        icons.extend(pack_icons.iter().cloned());
        icons_by_pack.insert(pack_summary.prefix.to_string(), pack_icons);
    }

    icons.sort_by(|left, right| {
        left.pack_name
            .as_ref()
            .cmp(right.pack_name.as_ref())
            .then_with(|| left.name.as_ref().cmp(right.name.as_ref()))
    });

    let mut representative_icons = Vec::with_capacity(MAX_ICON_RESULTS.min(packs.len()));
    for index in 0..2 {
        for pack in &packs {
            if let Some(icon) = icons_by_pack
                .get(pack.prefix.as_ref())
                .and_then(|icons| icons.get(index))
            {
                representative_icons.push(icon.clone());
                if representative_icons.len() >= MAX_ICON_RESULTS {
                    break;
                }
            }
        }
        if representative_icons.len() >= MAX_ICON_RESULTS {
            break;
        }
    }

    ExternalIconCatalog {
        icons,
        icons_by_pack,
        representative_icons,
        packs,
    }
}

fn load_external_icon_pack_catalog(pack_summary: &IconPackSummary) -> Vec<ExternalIcon> {
    let path = external_icon_data_dir().join(format!("{}.json", pack_summary.prefix.as_ref()));
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(pack) = serde_json::from_str::<IconifyPack>(&text) else {
        return Vec::new();
    };
    let IconifyPack { icons, aliases } = pack;

    let mut pack_icons = Vec::with_capacity(icons.len() + aliases.len());
    pack_icons.extend(icons.iter().map(|(name, icon_meta)| {
        external_icon_from_summary(
            pack_summary,
            name,
            icon_meta.width.unwrap_or(pack_summary.width),
            icon_meta.height.unwrap_or(pack_summary.height),
        )
    }));
    pack_icons.extend(aliases.into_iter().filter_map(|(name, alias)| {
        let parent = icons.get(alias.parent.as_str())?;
        Some(external_icon_from_summary(
            pack_summary,
            &name,
            alias.width.or(parent.width).unwrap_or(pack_summary.width),
            alias
                .height
                .or(parent.height)
                .unwrap_or(pack_summary.height),
        ))
    }));
    pack_icons.sort_by(|left, right| left.name.as_ref().cmp(right.name.as_ref()));
    pack_icons.dedup_by(|left, right| left.name.as_ref() == right.name.as_ref());
    pack_icons
}

fn external_icon_from_summary(
    pack_summary: &IconPackSummary,
    name: &str,
    width: u32,
    height: u32,
) -> ExternalIcon {
    let label = titleize_icon_name(name);
    let stem = format!("{}-{}", pack_summary.prefix.as_ref(), name);
    let search_text = format!(
        "{} {} {} {}",
        name.to_lowercase(),
        label.to_lowercase(),
        pack_summary.prefix.as_ref().to_lowercase(),
        pack_summary.name.as_ref().to_lowercase()
    );
    ExternalIcon {
        pack: pack_summary.prefix.clone(),
        pack_name: pack_summary.name.clone(),
        name: name.to_string().into(),
        label: label.into(),
        stem: stem.into(),
        width: width.max(1),
        height: height.max(1),
        search_text: search_text.into(),
    }
}

fn load_external_icon_bodies(pack: &str) -> anyhow::Result<HashMap<String, ExternalIconBody>> {
    let path = external_icon_data_dir().join(format!("{pack}.json"));
    let text = std::fs::read_to_string(path)?;
    let pack = serde_json::from_str::<IconifyBodyPack>(&text)?;
    let IconifyBodyPack { icons, aliases } = pack;
    let mut bodies = HashMap::with_capacity(icons.len() + aliases.len());
    bodies.extend(icons.into_iter().map(|(name, icon)| {
        (
            name,
            ExternalIconBody {
                body: icon.body,
                width: icon.width,
                height: icon.height,
                h_flip: false,
                v_flip: false,
                rotate: None,
            },
        )
    }));

    for (name, alias) in aliases {
        if bodies.contains_key(&name) {
            continue;
        }
        let Some(parent) = bodies.get(alias.parent.as_str()).cloned() else {
            continue;
        };
        bodies.insert(
            name,
            ExternalIconBody {
                body: parent.body,
                width: alias.width.or(parent.width),
                height: alias.height.or(parent.height),
                h_flip: alias.h_flip,
                v_flip: alias.v_flip,
                rotate: alias.rotate,
            },
        );
    }

    Ok(bodies)
}

fn warm_external_icon_previews(icons: Vec<ExternalIcon>) -> Vec<(String, Option<SharedString>)> {
    let mut pack_bodies = HashMap::<String, HashMap<String, ExternalIconBody>>::new();
    let mut previews = Vec::with_capacity(icons.len());

    for icon in icons {
        let key = icon.id();
        if let Some(preview_path) = existing_external_icon_preview(&icon) {
            previews.push((key, Some(preview_path.into())));
            continue;
        }

        let body = representative_icon_body(icon.pack.as_ref(), icon.name.as_ref()).or_else(|| {
            let pack = icon.pack.to_string();
            let bodies = pack_bodies
                .entry(pack.clone())
                .or_insert_with(|| load_external_icon_bodies(&pack).unwrap_or_default());
            bodies.get(icon.name.as_ref()).cloned()
        });
        let Some(body) = body else {
            previews.push((key, None));
            continue;
        };

        let width = body.width.unwrap_or(icon.width).max(1);
        let height = body.height.unwrap_or(icon.height).max(1);
        let icon_body = transform_iconify_alias_body(
            &body.body,
            width,
            height,
            body.h_flip,
            body.v_flip,
            body.rotate,
        );
        let svg = wrap_icon_body(&icon_body, width, height);
        let preview_path = write_external_icon_preview(&icon, &svg)
            .ok()
            .map(SharedString::from);
        previews.push((key, preview_path));
    }

    previews
}

fn representative_icon_body(pack: &str, name: &str) -> Option<ExternalIconBody> {
    representative_icon_bodies()
        .get(&format!("{pack}:{name}"))
        .cloned()
}

fn representative_icon_bodies() -> &'static HashMap<String, ExternalIconBody> {
    REPRESENTATIVE_ICON_BODY_CACHE.get_or_init(|| {
        let mut bodies = HashMap::new();
        for line in ICON_REPRESENTATIVE_BODIES.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }

            let mut columns = line.splitn(8, '\t');
            let Some(pack) = columns.next() else {
                continue;
            };
            let Some(name) = columns.next() else {
                continue;
            };
            let width = parse_optional_u32(columns.next());
            let height = parse_optional_u32(columns.next());
            let h_flip = columns.next() == Some("1");
            let v_flip = columns.next() == Some("1");
            let rotate = parse_optional_u8(columns.next());
            let Some(body_json) = columns.next() else {
                continue;
            };
            let Ok(body) = serde_json::from_str::<String>(body_json) else {
                continue;
            };

            bodies.insert(
                format!("{pack}:{name}"),
                ExternalIconBody {
                    body,
                    width,
                    height,
                    h_flip,
                    v_flip,
                    rotate,
                },
            );
        }
        bodies
    })
}

fn parse_optional_u32(value: Option<&str>) -> Option<u32> {
    value.and_then(|value| {
        (!value.is_empty())
            .then_some(value)
            .and_then(|value| value.parse().ok())
    })
}

fn parse_optional_u8(value: Option<&str>) -> Option<u8> {
    value.and_then(|value| {
        (!value.is_empty())
            .then_some(value)
            .and_then(|value| value.parse().ok())
    })
}

fn transform_iconify_alias_body(
    body: &str,
    width: u32,
    height: u32,
    h_flip: bool,
    v_flip: bool,
    rotate: Option<u8>,
) -> String {
    let mut transforms = Vec::new();

    if h_flip {
        transforms.push(format!("translate({width} 0) scale(-1 1)"));
    }
    if v_flip {
        transforms.push(format!("translate(0 {height}) scale(1 -1)"));
    }
    if let Some(rotate) = rotate
        .map(|rotate| rotate % 4)
        .filter(|rotate| *rotate != 0)
    {
        transforms.push(format!(
            "rotate({} {} {})",
            rotate as u16 * 90,
            width as f32 / 2.,
            height as f32 / 2.
        ));
    }

    if transforms.is_empty() {
        body.to_string()
    } else {
        format!(r#"<g transform="{}">{body}</g>"#, transforms.join(" "))
    }
}

fn icon_preview_batch_signature(icons: &[ExternalIcon]) -> String {
    let mut signature = format!("{}:{}:", EXTERNAL_ICON_PREVIEW_CACHE_VERSION, icons.len());
    for icon in icons.iter().take(24) {
        signature.push_str(icon.pack.as_ref());
        signature.push(':');
        signature.push_str(icon.name.as_ref());
        signature.push('|');
    }
    signature
}

fn external_icon_data_dir() -> PathBuf {
    std::env::var("DX_ICONS_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DX_ICON_DATA_DIR))
}

fn external_icon_preview_path(icon: &ExternalIcon) -> PathBuf {
    repo_root()
        .join("target")
        .join("icon-picker-icons")
        .join(EXTERNAL_ICON_PREVIEW_CACHE_VERSION)
        .join(sanitize_file_component(icon.pack.as_ref()))
        .join(format!(
            "{}.svg",
            sanitize_file_component(icon.name.as_ref())
        ))
}

fn existing_external_icon_preview(icon: &ExternalIcon) -> Option<String> {
    let path = external_icon_preview_path(icon);
    path.exists()
        .then(|| path.to_string_lossy().replace('\\', "/"))
}

fn write_external_icon_preview(icon: &ExternalIcon, svg: &str) -> anyhow::Result<String> {
    let path = external_icon_preview_path(icon);
    let Some(dir) = path.parent() else {
        anyhow::bail!("invalid icon preview path");
    };
    std::fs::create_dir_all(&dir)?;
    if !path.exists() {
        std::fs::write(&path, svg)?;
    }
    Ok(path.to_string_lossy().replace('\\', "/"))
}

fn wrap_icon_body(body: &str, width: u32, height: u32) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}"><g fill="currentColor">{body}</g></svg>"#
    )
}

fn repo_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("G:/Zed"))
}

fn sanitize_file_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn titleize_icon_name(name: &str) -> String {
    name.split(['-', '_', ':'])
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
