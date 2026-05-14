use editor::{Editor, EditorEvent};
use gpui::{
    AnyElement, App, AppContext as _, AsyncWindowContext, Context, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, Pixels, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Subscription, WeakEntity, Window, actions, div, point, px,
};
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap, path::PathBuf, sync::OnceLock};
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
const MAX_ICON_RESULTS: usize = 360;
static EXTERNAL_ICON_CATALOG_CACHE: OnceLock<ExternalIconCatalog> = OnceLock::new();

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
    external_icons: Vec<ExternalIcon>,
    external_icons_by_pack: HashMap<String, Vec<ExternalIcon>>,
    representative_external_icons: Vec<ExternalIcon>,
    packs: Vec<IconPackSummary>,
    selected_pack: Option<SharedString>,
    selected_icon: Option<PickerIcon>,
    loading_external_icons: bool,
    external_catalog_loaded: bool,
    pack_svg_cache: RefCell<HashMap<String, HashMap<String, ExternalIconBody>>>,
    preview_cache: RefCell<HashMap<String, Option<ExternalSvg>>>,
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
                external_icons: Vec::new(),
                external_icons_by_pack: HashMap::default(),
                representative_external_icons,
                packs,
                selected_pack: None,
                selected_icon,
                loading_external_icons: false,
                external_catalog_loaded: false,
                pack_svg_cache: RefCell::default(),
                preview_cache: RefCell::default(),
                pack_scroll_handle: ScrollHandle::new(),
                status: None,
                _subscriptions: vec![filter_subscription],
            }
        })
    }

    fn ensure_icon_data_loaded_for_view(&mut self, cx: &mut Context<Self>) {
        let query = self.query(cx);
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

    fn filtered_icons(&self, cx: &App) -> (Vec<PickerIcon>, usize, usize) {
        let query = self.query(cx);
        let selected_pack = self.selected_pack.as_ref().map(|pack| pack.as_ref());
        let mut icons = Vec::new();
        let mut match_count = 0;
        let total_count = self.total_count_for_selection(selected_pack);

        if selected_pack.is_none() && query.is_empty() {
            icons.extend(
                self.representative_external_icons
                    .iter()
                    .cloned()
                    .map(PickerIcon::External),
            );
            if icons.len() < MAX_ICON_RESULTS {
                icons.extend(self.zed_icons.iter().copied().map(PickerIcon::Zed));
            }
            icons.truncate(MAX_ICON_RESULTS);
            return (icons, MAX_ICON_RESULTS.min(total_count), total_count);
        }

        if selected_pack.is_none() || selected_pack == Some("zed") {
            icons.extend(self.zed_icons.iter().copied().filter_map(|icon_name| {
                let payload = DraggedIconAsset::new(icon_name);
                let searchable = format!(
                    "{} {} zed",
                    payload.stem.as_ref(),
                    payload.label.as_ref().to_lowercase()
                );
                if !icon_search_matches(searchable.as_str(), query.as_str()) {
                    return None;
                }
                match_count += 1;
                Some(PickerIcon::Zed(icon_name))
            }));
        }

        if selected_pack != Some("zed") {
            if query.is_empty() && selected_pack.is_none() {
                icons.extend(
                    self.representative_external_icons
                        .iter()
                        .cloned()
                        .map(PickerIcon::External),
                );
            } else if let Some(selected_pack) = selected_pack {
                if let Some(pack_icons) = self.external_icons_by_pack.get(selected_pack) {
                    for icon in pack_icons {
                        if icon_search_matches(icon.search_text.as_ref(), query.as_str()) {
                            match_count += 1;
                            if icons.len() < MAX_ICON_RESULTS {
                                icons.push(PickerIcon::External(icon.clone()));
                            }
                        }
                    }
                }
            } else {
                for icon in &self.external_icons {
                    if icon_search_matches(icon.search_text.as_ref(), query.as_str()) {
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

    fn external_svg(&self, icon: &ExternalIcon) -> Option<ExternalSvg> {
        let key = icon.id();
        let cached_svg = self.preview_cache.borrow().get(&key).cloned();
        if let Some(svg) = cached_svg {
            return svg;
        }

        let body = self.external_icon_body(icon)?;
        let width = body.width.unwrap_or(icon.width).max(1);
        let height = body.height.unwrap_or(icon.height).max(1);
        let svg = wrap_icon_body(&body.body, width, height);
        let preview_path = write_external_icon_preview(icon, &svg).ok()?;
        let external_svg = ExternalSvg {
            preview_path: preview_path.into(),
        };
        self.preview_cache
            .borrow_mut()
            .insert(key, Some(external_svg.clone()));
        Some(external_svg)
    }

    fn external_icon_body(&self, icon: &ExternalIcon) -> Option<ExternalIconBody> {
        let pack_loaded = self
            .pack_svg_cache
            .borrow()
            .contains_key(icon.pack.as_ref());
        if !pack_loaded {
            let pack_icons = load_external_icon_bodies(icon.pack.as_ref()).unwrap_or_default();
            self.pack_svg_cache
                .borrow_mut()
                .insert(icon.pack.to_string(), pack_icons);
        }

        self.pack_svg_cache
            .borrow()
            .get(icon.pack.as_ref())
            .and_then(|icons| icons.get(icon.name.as_ref()).cloned())
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
        let load_external_svg = self.selected_pack.is_some() || !self.query(cx).is_empty();
        let icon_preview = self.render_icon_preview(&icon, IconSize::Medium, load_external_svg, cx);

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
        load_external_svg: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match icon {
            PickerIcon::Zed(icon_name) => Icon::new(*icon_name).size(size).into_any_element(),
            PickerIcon::External(icon) if load_external_svg => self
                .external_svg(icon)
                .map(|svg| {
                    Icon::from_external_svg(svg.preview_path)
                        .size(size)
                        .into_any_element()
                })
                .unwrap_or_else(|| Icon::new(IconName::SquareDot).size(size).into_any_element()),
            PickerIcon::External(icon) => external_pack_badge(icon, cx),
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
        self.ensure_icon_data_loaded_for_view(cx);
        let (icons, total_matches, total_count) = self.filtered_icons(cx);
        let is_empty = icons.is_empty();
        let shown_count = icons.len();
        let count_label = self.status.clone().unwrap_or_else(|| {
            if self.loading_external_icons {
                "loading icons".into()
            } else if self.query(cx).is_empty() {
                format!("{shown_count} / {total_count}").into()
            } else {
                format!("{total_matches} / {total_count}").into()
            }
        });
        let icon_tiles = icons
            .iter()
            .cloned()
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

fn icon_search_matches(searchable: &str, query: &str) -> bool {
    query
        .split_whitespace()
        .all(|term| searchable.contains(term))
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
}

#[derive(Deserialize)]
struct IconifyCatalogIcon {
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
}

#[derive(Clone)]
struct ExternalIconBody {
    body: String,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Deserialize)]
struct IconifyBodyPack {
    icons: HashMap<String, IconifyBody>,
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
    let mut icons = Vec::new();
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
            icons_by_pack: HashMap::default(),
            representative_icons: Vec::new(),
            packs,
        };
    }

    let mut icons = Vec::new();
    for pack_summary in &packs {
        icons.extend(load_external_icon_pack_catalog(pack_summary));
    }

    icons.sort_by(|left, right| {
        left.pack_name
            .as_ref()
            .cmp(right.pack_name.as_ref())
            .then_with(|| left.name.as_ref().cmp(right.name.as_ref()))
    });
    let mut icons_by_pack = HashMap::<String, Vec<ExternalIcon>>::new();
    for icon in &icons {
        icons_by_pack
            .entry(icon.pack.to_string())
            .or_default()
            .push(icon.clone());
    }

    let mut representative_icons = Vec::new();
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

    let mut icons = pack
        .icons
        .into_iter()
        .map(|(name, icon_meta)| {
            external_icon_from_summary(
                pack_summary,
                &name,
                icon_meta.width.unwrap_or(pack_summary.width),
                icon_meta.height.unwrap_or(pack_summary.height),
            )
        })
        .collect::<Vec<_>>();
    icons.sort_by(|left, right| left.name.as_ref().cmp(right.name.as_ref()));
    icons
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
    Ok(pack
        .icons
        .into_iter()
        .map(|(name, icon)| {
            (
                name,
                ExternalIconBody {
                    body: icon.body,
                    width: icon.width,
                    height: icon.height,
                },
            )
        })
        .collect())
}

fn external_icon_data_dir() -> PathBuf {
    std::env::var("DX_ICONS_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DX_ICON_DATA_DIR))
}

fn write_external_icon_preview(icon: &ExternalIcon, svg: &str) -> anyhow::Result<String> {
    let dir = repo_root()
        .join("target")
        .join("icon-picker-icons")
        .join(sanitize_file_component(icon.pack.as_ref()));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!(
        "{}.svg",
        sanitize_file_component(icon.name.as_ref())
    ));
    if !path.exists() {
        std::fs::write(&path, svg)?;
    }
    Ok(path.to_string_lossy().replace('\\', "/"))
}

fn wrap_icon_body(body: &str, width: u32, height: u32) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}">{body}</svg>"#
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
