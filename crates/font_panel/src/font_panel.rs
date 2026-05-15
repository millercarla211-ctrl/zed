use db::kvp::KeyValueStore;
use editor::{Editor, EditorEvent};
use fs::Fs;
use gpui::{
    AnyElement, App, AppContext as _, AsyncWindowContext, ClipboardItem, Context, Entity,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, Pixels, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement, Subscription, WeakEntity, Window, actions, div,
    point, px,
};
use serde::{Deserialize, Serialize};
use settings::{FontFamilyName, Settings};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt::Write as _,
    fs as std_fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use theme::FontFamilyCache;
use theme_settings::ThemeSettings;
use ui::{TintColor, Tooltip, prelude::*};
use url::Url;
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
};

mod google_fonts;

#[cfg(target_os = "windows")]
use web_preview::web_preview_view::WebPreviewView;

actions!(
    font_panel,
    [
        /// Toggles the font panel.
        Toggle,
        /// Toggles focus on the font panel.
        ToggleFocus,
    ]
);

const FONT_PANEL_KEY: &str = "FontPanel";
const MAX_FONT_RESULTS: usize = 160;
const MAX_RECENT_FONT_ACTIONS: usize = 5;
const MAX_PINNED_FONT_ACTIONS: usize = 8;
const PINNED_FONT_ACTIONS_KEY: &str = "asset_panel_pinned_fonts_v1";
const PINNED_FONT_ACTIONS_STATE_VERSION: u32 = 1;

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace.register_action(|workspace, _: &ToggleFocus, window, cx| {
            workspace.toggle_panel_focus::<FontPanel>(window, cx);
        });
        workspace.register_action(|workspace, _: &Toggle, window, cx| {
            if !workspace.toggle_panel_focus::<FontPanel>(window, cx) {
                workspace.close_panel::<FontPanel>(window, cx);
            }
        });
    })
    .detach();
}

pub struct FontPanel {
    workspace: WeakEntity<Workspace>,
    fs: Arc<dyn Fs>,
    filter_editor: Entity<Editor>,
    fonts: Vec<SharedString>,
    font_search_text_cache: RefCell<HashMap<SharedString, SharedString>>,
    fonts_loaded: bool,
    loading_fonts: bool,
    source_filter: FontSourceFilter,
    source_scroll_handle: ScrollHandle,
    selected_font: Option<SharedString>,
    selected_source: FontSource,
    pinned_font_actions: VecDeque<RecentFontEntry>,
    recent_font_actions: VecDeque<RecentFontEntry>,
    status: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FontSource {
    System,
    Web,
}

impl FontSource {
    fn label(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Web => "web",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FontSourceFilter {
    All,
    System,
    Web,
}

impl FontSourceFilter {
    fn matches(self, source: FontSource) -> bool {
        match self {
            Self::All => true,
            Self::System => source == FontSource::System,
            Self::Web => source == FontSource::Web,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::System => "System",
            Self::Web => "Web",
        }
    }
}

#[derive(Clone, Copy)]
struct FontSourceCounts {
    system: usize,
    web: usize,
}

impl FontSourceCounts {
    fn from_panel(panel: &FontPanel) -> Self {
        Self {
            system: panel.fonts.len(),
            web: google_fonts::GOOGLE_FONT_FAMILIES.len(),
        }
    }

    fn count(self, filter: FontSourceFilter) -> usize {
        match filter {
            FontSourceFilter::All => self.system + self.web,
            FontSourceFilter::System => self.system,
            FontSourceFilter::Web => self.web,
        }
    }
}

#[derive(Clone)]
struct FontEntry {
    name: SharedString,
    source: FontSource,
}

#[derive(Clone)]
struct RecentFontEntry {
    font: FontEntry,
    action: RecentFontAction,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RecentFontAction {
    Previewed,
    CopiedCss,
    AppliedEditor,
    AppliedUi,
    AddedProject,
    Pinned,
}

#[derive(Serialize, Deserialize)]
struct SerializedPinnedFontActions {
    version: u32,
    entries: Vec<SerializedPinnedFontAction>,
}

#[derive(Serialize, Deserialize)]
struct SerializedPinnedFontAction {
    name: String,
    source: FontSource,
    action: RecentFontAction,
}

impl SerializedPinnedFontAction {
    fn from_entry(entry: &RecentFontEntry) -> Self {
        Self {
            name: entry.font.name.as_ref().to_string(),
            source: entry.font.source,
            action: entry.action,
        }
    }

    fn into_entry(self) -> RecentFontEntry {
        RecentFontEntry {
            font: FontEntry {
                name: self.name.into(),
                source: self.source,
            },
            action: self.action,
        }
    }
}

#[derive(Clone)]
struct WebFontSpec {
    name: String,
    family_query: String,
    variable: String,
}

impl FontPanel {
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
        let fs = workspace.app_state().fs.clone();
        let workspace_handle = cx.entity().downgrade();

        cx.new(|cx| {
            let filter_editor = cx.new(|cx| {
                let mut editor = Editor::single_line(window, cx);
                editor.set_placeholder_text("Search fonts...", window, cx);
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

            let selected_font = Some(Self::current_buffer_font(cx));
            let (fonts, fonts_loaded) = Self::cached_fonts(cx, selected_font.clone());
            let pinned_font_actions = load_pinned_font_actions(cx);

            Self {
                workspace: workspace_handle,
                fs,
                filter_editor,
                fonts,
                font_search_text_cache: RefCell::default(),
                fonts_loaded,
                loading_fonts: false,
                source_filter: FontSourceFilter::All,
                source_scroll_handle: ScrollHandle::new(),
                selected_font,
                selected_source: FontSource::System,
                pinned_font_actions,
                recent_font_actions: VecDeque::with_capacity(MAX_RECENT_FONT_ACTIONS),
                status: None,
                _subscriptions: vec![filter_subscription],
            }
        })
    }

    fn ensure_system_fonts_loading(&mut self, cx: &mut Context<Self>) {
        if self.fonts_loaded || self.loading_fonts {
            return;
        }

        self.loading_fonts = true;
        let font_family_cache = FontFamilyCache::global(cx);
        cx.spawn(async move |panel, cx| {
            font_family_cache.prefetch(cx).await;
            panel
                .update(cx, |panel, cx| {
                    if let Some(fonts) = FontFamilyCache::global(cx).try_list_font_families() {
                        panel.fonts = Self::sort_fonts(fonts);
                        panel.font_search_text_cache.borrow_mut().clear();
                        panel.fonts_loaded = true;
                    }
                    panel.loading_fonts = false;
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn current_buffer_font(cx: &App) -> SharedString {
        ThemeSettings::get_global(cx).buffer_font.family.clone()
    }

    fn cached_fonts(cx: &App, selected_font: Option<SharedString>) -> (Vec<SharedString>, bool) {
        match FontFamilyCache::global(cx).try_list_font_families() {
            Some(fonts) => (Self::sort_fonts(fonts), true),
            None => {
                let mut fonts = Vec::with_capacity(usize::from(selected_font.is_some()));
                fonts.extend(selected_font);
                (fonts, false)
            }
        }
    }

    fn refresh_fonts_if_needed(&mut self, cx: &App) {
        if self.fonts_loaded {
            return;
        }

        let Some(fonts) = FontFamilyCache::global(cx).try_list_font_families() else {
            return;
        };
        let fonts = Self::sort_fonts(fonts);
        if self.fonts != fonts {
            self.fonts = fonts;
            self.font_search_text_cache.borrow_mut().clear();
        }
        self.fonts_loaded = true;
    }

    fn sort_fonts(mut fonts: Vec<SharedString>) -> Vec<SharedString> {
        fonts.sort_by_cached_key(|font| lowercase_text(font.as_ref()));
        fonts.dedup();
        fonts
    }

    fn query(&self, cx: &App) -> String {
        lowercase_text(self.filter_editor.read(cx).text(cx).trim())
    }

    fn sample_text(&self, _cx: &App) -> SharedString {
        "The quick brown fox jumps over the lazy dog.".into()
    }

    fn matching_fonts(&self, query: &str, limit: usize) -> (Vec<FontEntry>, usize) {
        let source_filter = self.source_filter;
        if query.is_empty() {
            let total_count = FontSourceCounts::from_panel(self).count(source_filter);
            let mut visible_fonts = Vec::with_capacity(limit.min(total_count));

            if source_filter.matches(FontSource::System) {
                visible_fonts.extend(self.fonts.iter().take(limit).cloned().map(|name| {
                    FontEntry {
                        name,
                        source: FontSource::System,
                    }
                }));
            }

            if visible_fonts.len() < limit && source_filter.matches(FontSource::Web) {
                visible_fonts.extend(
                    google_fonts::GOOGLE_FONT_FAMILIES
                        .iter()
                        .take(limit - visible_fonts.len())
                        .map(|font_name| FontEntry {
                            name: (*font_name).into(),
                            source: FontSource::Web,
                        }),
                );
            }

            return (visible_fonts, total_count);
        }

        let query_term_count = query.split_whitespace().count();
        let mut query_terms = Vec::with_capacity(query_term_count);
        query_terms.extend(query.split_whitespace());
        let mut visible_fonts = Vec::with_capacity(limit);
        let mut match_count = 0;
        let mut exact_match = false;

        if source_filter.matches(FontSource::System) {
            for font in &self.fonts {
                exact_match |= font.as_ref().eq_ignore_ascii_case(query.as_str());
                if !self.font_matches(font.as_ref(), &query_terms) {
                    continue;
                }

                match_count += 1;
                if visible_fonts.len() < limit {
                    visible_fonts.push(FontEntry {
                        name: font.clone(),
                        source: FontSource::System,
                    });
                }
            }
        }

        if source_filter.matches(FontSource::Web) {
            for font_name in google_fonts::GOOGLE_FONT_FAMILIES {
                exact_match |= font_name.eq_ignore_ascii_case(query.as_str());
                if !self.font_matches(font_name, &query_terms) {
                    continue;
                }

                match_count += 1;
                if visible_fonts.len() < limit {
                    visible_fonts.push(FontEntry {
                        name: (*font_name).into(),
                        source: FontSource::Web,
                    });
                }
            }
        }

        if !query.is_empty()
            && source_filter.matches(FontSource::Web)
            && !exact_match
            && let Some(font_name) = custom_web_font_name(&query)
        {
            match_count += 1;
            if visible_fonts.len() < limit {
                visible_fonts.push(FontEntry {
                    name: font_name.into(),
                    source: FontSource::Web,
                });
            }
        }

        (visible_fonts, match_count)
    }

    fn font_matches(&self, font_name: &str, query_terms: &[&str]) -> bool {
        if let Some(matches) = {
            let search_text_cache = self.font_search_text_cache.borrow();
            search_text_cache
                .get(font_name)
                .map(|search_text| font_search_matches(search_text.as_ref(), query_terms))
        } {
            return matches;
        }

        let search_text: SharedString = lowercase_text(font_name).into();
        self.font_search_text_cache
            .borrow_mut()
            .insert(font_name.into(), search_text.clone());
        font_search_matches(search_text.as_ref(), query_terms)
    }

    fn render_source_filter_button(
        &self,
        filter: FontSourceFilter,
        count: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.source_filter == filter;
        let label = font_count_label(filter.label(), count);
        let button_id = font_element_id("font-source-filter-", filter.label());
        div().flex_none().child(
            Button::new(button_id, label)
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

    fn set_selected_font(&mut self, font: &FontEntry) {
        self.selected_font = Some(font.name.clone());
        self.selected_source = font.source;
    }

    fn select_font(&mut self, font: FontEntry, cx: &mut Context<Self>) {
        self.set_selected_font(&font);
        self.status = Some(font_status_label("Previewing ", font.name.as_ref()));
        cx.notify();
    }

    fn record_recent_font_action(&mut self, font: &FontEntry, action: RecentFontAction) {
        self.recent_font_actions
            .retain(|entry| entry.font.name.as_ref() != font.name.as_ref());
        self.recent_font_actions.push_front(RecentFontEntry {
            font: font.clone(),
            action,
        });
        self.recent_font_actions.truncate(MAX_RECENT_FONT_ACTIONS);
    }

    fn clear_recent_font_actions(&mut self, cx: &mut Context<Self>) {
        self.recent_font_actions.clear();
        self.status = Some("Cleared recent fonts".into());
        cx.notify();
    }

    fn pin_font_action(&mut self, entry: RecentFontEntry, cx: &mut Context<Self>) {
        self.pinned_font_actions
            .retain(|pinned| pinned.font.name.as_ref() != entry.font.name.as_ref());
        let name = entry.font.name.clone();
        self.pinned_font_actions.push_front(entry);
        self.pinned_font_actions.truncate(MAX_PINNED_FONT_ACTIONS);
        self.status = Some(font_status_label("Pinned ", name.as_ref()));
        self.persist_pinned_font_actions(cx);
        cx.notify();
    }

    fn unpin_font_action(&mut self, font: FontEntry, cx: &mut Context<Self>) {
        self.pinned_font_actions
            .retain(|pinned| pinned.font.name.as_ref() != font.name.as_ref());
        self.status = Some(font_status_label("Unpinned ", font.name.as_ref()));
        self.persist_pinned_font_actions(cx);
        cx.notify();
    }

    fn clear_pinned_font_actions(&mut self, cx: &mut Context<Self>) {
        self.pinned_font_actions.clear();
        self.status = Some("Cleared pinned fonts".into());
        self.persist_pinned_font_actions(cx);
        cx.notify();
    }

    fn persist_pinned_font_actions(&self, cx: &mut Context<Self>) {
        let entries = self
            .pinned_font_actions
            .iter()
            .take(MAX_PINNED_FONT_ACTIONS)
            .map(SerializedPinnedFontAction::from_entry)
            .collect();
        let Ok(json) = serde_json::to_string(&SerializedPinnedFontActions {
            version: PINNED_FONT_ACTIONS_STATE_VERSION,
            entries,
        }) else {
            return;
        };
        let kvp = KeyValueStore::global(cx);
        cx.background_spawn(async move {
            let _ = kvp
                .write_kvp(PINNED_FONT_ACTIONS_KEY.to_string(), json)
                .await;
        })
        .detach();
    }

    fn selected_font_entry(&self) -> Option<FontEntry> {
        Some(FontEntry {
            name: self.selected_font.clone()?,
            source: self.selected_source,
        })
    }

    fn apply_selected_to_buffer(&mut self, cx: &mut Context<Self>) {
        let Some(font) = self.selected_font.clone() else {
            self.status = Some("Select a font first".into());
            cx.notify();
            return;
        };
        if self.selected_source != FontSource::System {
            self.status = Some("Add web fonts to a project before using them in app code".into());
            cx.notify();
            return;
        }

        let font_name = font.to_string();
        settings::update_settings_file(self.fs.clone(), cx, move |settings, _| {
            settings.theme.buffer_font_family = Some(FontFamilyName(Arc::from(font_name.as_str())));
        });

        self.status = Some(font_status_label("Editor font set to ", font.as_ref()));
        if let Some(font) = self.selected_font_entry() {
            self.record_recent_font_action(&font, RecentFontAction::AppliedEditor);
        }
        cx.notify();
    }

    fn apply_selected_to_ui(&mut self, cx: &mut Context<Self>) {
        let Some(font) = self.selected_font.clone() else {
            self.status = Some("Select a font first".into());
            cx.notify();
            return;
        };
        if self.selected_source != FontSource::System {
            self.status = Some("Install the web font locally before using it for Zed UI".into());
            cx.notify();
            return;
        }

        let font_name = font.to_string();
        settings::update_settings_file(self.fs.clone(), cx, move |settings, _| {
            settings.theme.ui_font_family = Some(FontFamilyName(Arc::from(font_name.as_str())));
        });

        self.status = Some(font_status_label("UI font set to ", font.as_ref()));
        if let Some(font) = self.selected_font_entry() {
            self.record_recent_font_action(&font, RecentFontAction::AppliedUi);
        }
        cx.notify();
    }

    fn add_selected_to_project(&mut self, cx: &mut Context<Self>) {
        let Some(font) = self.selected_font.clone() else {
            self.status = Some("Select a web font first".into());
            cx.notify();
            return;
        };
        if self.selected_source != FontSource::Web {
            self.status = Some("Select a web font to add to the project".into());
            cx.notify();
            return;
        };
        let Some(web_font) = web_font_spec_by_name(font.as_ref()) else {
            self.status = Some("Select a web font to add to the project".into());
            cx.notify();
            return;
        };
        let Some(root) = self.primary_worktree_root(cx) else {
            self.status = Some("No project root found for font CSS".into());
            cx.notify();
            return;
        };

        match add_web_font_to_project(&root, &web_font) {
            Ok(path) => {
                self.status = Some(font_added_status(web_font.name.as_str(), &path));
                if let Some(font) = self.selected_font_entry() {
                    self.record_recent_font_action(&font, RecentFontAction::AddedProject);
                }
            }
            Err(error) => {
                self.status = Some(format!("Failed to add font CSS: {error}").into());
            }
        }
        cx.notify();
    }

    fn preview_selected_font(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(font) = self.selected_font.clone() else {
            self.status = Some("Select a font first".into());
            cx.notify();
            return;
        };
        let web_font = (self.selected_source == FontSource::Web)
            .then(|| web_font_spec_by_name(font.as_ref()))
            .flatten();
        let sample_text = self.sample_text(cx);
        let Some(preview_url) = local_font_preview_url(
            font.as_ref(),
            self.selected_source,
            web_font.as_ref(),
            sample_text.as_ref(),
        ) else {
            self.status = Some("Could not create font preview".into());
            cx.notify();
            return;
        };

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

        self.status = Some(font_status_label_with_suffix(
            "Previewing ",
            font.as_ref(),
            " in WebPreview",
        ));
        if let Some(font) = self.selected_font_entry() {
            self.record_recent_font_action(&font, RecentFontAction::Previewed);
        }
        cx.notify();
    }

    fn copy_selected_css(&mut self, cx: &mut Context<Self>) {
        let Some(font) = self.selected_font.clone() else {
            self.status = Some("Select a font first".into());
            cx.notify();
            return;
        };

        let css = if self.selected_source == FontSource::Web {
            match web_font_spec_by_name(font.as_ref()) {
                Some(web_font) => web_font_css_snippet(&web_font),
                None => system_font_css_snippet(font.as_ref()),
            }
        } else {
            system_font_css_snippet(font.as_ref())
        };

        cx.write_to_clipboard(ClipboardItem::new_string(css));
        self.status = Some(font_status_label("Copied CSS for ", font.as_ref()));
        if let Some(font) = self.selected_font_entry() {
            self.record_recent_font_action(&font, RecentFontAction::CopiedCss);
        }
        cx.notify();
    }

    fn primary_worktree_root(&self, cx: &App) -> Option<PathBuf> {
        let workspace = self.workspace.upgrade()?;
        let workspace = workspace.read(cx);
        let project = workspace.project().read(cx);
        project
            .visible_worktrees(cx)
            .next()
            .map(|worktree| worktree.read(cx).abs_path().to_path_buf())
    }

    fn render_font_row(&self, font: FontEntry, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self
            .selected_font
            .as_ref()
            .is_some_and(|selected| selected == &font.name);
        let id_font = font.name.clone();
        let source = font.source;
        let click_font = font.clone();
        let pin_font = font.clone();
        let is_system_font = source == FontSource::System;
        let row_id = font_element_id("font-panel-row-", id_font.as_ref());
        let pin_id = font_element_id("font-panel-pin-", id_font.as_ref());

        div()
            .id(row_id)
            .v_flex()
            .gap_1()
            .p_2()
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
            .tooltip(Tooltip::text(font.name.clone()))
            .on_click(cx.listener(move |panel, _, _, cx| {
                panel.select_font(click_font.clone(), cx);
            }))
            .child(
                h_flex()
                    .gap_2()
                    .justify_between()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_0p5()
                            .font_family(font.name.clone())
                            .child(
                                Label::new(font.name.clone())
                                    .size(LabelSize::Small)
                                    .truncate(),
                            )
                            .child(
                                Label::new("Aa Bb Cc 123")
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted)
                                    .truncate(),
                            ),
                    )
                    .child(Label::new(source.label()).size(LabelSize::XSmall).color(
                        if source == FontSource::Web {
                            Color::Accent
                        } else {
                            Color::Muted
                        },
                    )),
            )
            .when(selected, |this| {
                this.child(
                    h_flex()
                        .gap_1()
                        .flex_wrap()
                        .when(is_system_font, |this| {
                            this.child(
                                Button::new("font-panel-apply-editor", "Use in Editor")
                                    .style(ButtonStyle::Subtle)
                                    .size(ButtonSize::Compact)
                                    .on_click(cx.listener(|panel, _, _, cx| {
                                        panel.apply_selected_to_buffer(cx);
                                    })),
                            )
                        })
                        .child(
                            Button::new("font-panel-preview", "Preview")
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .on_click(cx.listener(|panel, _, window, cx| {
                                    panel.preview_selected_font(window, cx);
                                })),
                        )
                        .child(
                            Button::new("font-panel-copy-css", "Copy CSS")
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .on_click(cx.listener(|panel, _, _, cx| {
                                    panel.copy_selected_css(cx);
                                })),
                        )
                        .when(is_system_font, |this| {
                            this.child(
                                Button::new("font-panel-apply-ui", "Use in UI")
                                    .style(ButtonStyle::Subtle)
                                    .size(ButtonSize::Compact)
                                    .on_click(cx.listener(|panel, _, _, cx| {
                                        panel.apply_selected_to_ui(cx);
                                    })),
                            )
                        })
                        .when(!is_system_font, |this| {
                            this.child(
                                Button::new("font-panel-add-project", "Add to Project")
                                    .style(ButtonStyle::Subtle)
                                    .size(ButtonSize::Compact)
                                    .on_click(cx.listener(|panel, _, _, cx| {
                                        panel.add_selected_to_project(cx);
                                    })),
                            )
                        })
                        .child(
                            Button::new(pin_id, "Pin")
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .on_click(cx.listener(move |panel, _, _, cx| {
                                    panel.pin_font_action(
                                        RecentFontEntry {
                                            font: pin_font.clone(),
                                            action: RecentFontAction::Pinned,
                                        },
                                        cx,
                                    );
                                })),
                        ),
                )
            })
    }

    fn render_recent_font_section(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.recent_font_actions.is_empty() {
            return None;
        }

        let health_label = font_history_health_label(self.recent_font_actions.len());
        let mut rows =
            Vec::with_capacity(self.recent_font_actions.len().min(MAX_RECENT_FONT_ACTIONS));
        for (index, entry) in self
            .recent_font_actions
            .iter()
            .take(MAX_RECENT_FONT_ACTIONS)
            .cloned()
            .enumerate()
        {
            rows.push(self.render_font_history_row(entry, index, false, cx));
        }

        Some(
            v_flex()
                .id("font-panel-recent-actions-section")
                .gap_1()
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .child(Icon::new(IconName::Clock).size(IconSize::XSmall))
                                .child(
                                    Label::new("Recent")
                                        .size(LabelSize::XSmall)
                                        .color(Color::Muted),
                                )
                                .child(
                                    Label::new(health_label)
                                        .size(LabelSize::XSmall)
                                        .color(Color::Muted),
                                ),
                        )
                        .child(
                            Button::new("font-panel-clear-recent", "Clear")
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .on_click(cx.listener(|panel, _, _, cx| {
                                    panel.clear_recent_font_actions(cx);
                                })),
                        ),
                )
                .children(rows)
                .into_any_element(),
        )
    }

    fn render_pinned_font_section(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.pinned_font_actions.is_empty() {
            return None;
        }

        let health_label = font_history_health_label(self.pinned_font_actions.len());
        let mut rows =
            Vec::with_capacity(self.pinned_font_actions.len().min(MAX_PINNED_FONT_ACTIONS));
        for (index, entry) in self
            .pinned_font_actions
            .iter()
            .take(MAX_PINNED_FONT_ACTIONS)
            .cloned()
            .enumerate()
        {
            rows.push(self.render_font_history_row(entry, index, true, cx));
        }

        Some(
            v_flex()
                .id("font-panel-pinned-actions-section")
                .gap_1()
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .child(Icon::new(IconName::Star).size(IconSize::XSmall))
                                .child(
                                    Label::new("Pinned")
                                        .size(LabelSize::XSmall)
                                        .color(Color::Muted),
                                )
                                .child(
                                    Label::new(health_label)
                                        .size(LabelSize::XSmall)
                                        .color(Color::Muted),
                                ),
                        )
                        .child(
                            Button::new("font-panel-clear-pinned", "Clear")
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .on_click(cx.listener(|panel, _, _, cx| {
                                    panel.clear_pinned_font_actions(cx);
                                })),
                        ),
                )
                .children(rows)
                .into_any_element(),
        )
    }

    fn render_font_history_row(
        &self,
        entry: RecentFontEntry,
        index: usize,
        pinned: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id_suffix = index.to_string();
        let id_prefix = if pinned {
            "font-panel-pinned-"
        } else {
            "font-panel-recent-"
        };
        let row_id = font_element_id(id_prefix, id_suffix.as_str());
        let select_id = font_element_id(id_prefix, &format!("select-{id_suffix}"));
        let preview_id = font_element_id(id_prefix, &format!("preview-{id_suffix}"));
        let copy_id = font_element_id(id_prefix, &format!("copy-{id_suffix}"));
        let editor_id = font_element_id(id_prefix, &format!("editor-{id_suffix}"));
        let ui_id = font_element_id(id_prefix, &format!("ui-{id_suffix}"));
        let add_id = font_element_id(id_prefix, &format!("add-{id_suffix}"));
        let pin_id = font_element_id(id_prefix, &format!("pin-{id_suffix}"));
        let pin_entry = entry.clone();
        let font = entry.font;
        let pin_font = font.clone();
        let is_system_font = font.source == FontSource::System;
        let action_label = recent_font_action_label(entry.action);

        h_flex()
            .id(row_id)
            .gap_2()
            .items_center()
            .p_2()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().element_background)
            .child(Icon::new(IconName::Font).size(IconSize::Small))
            .child(
                v_flex()
                    .flex_1()
                    .gap_0p5()
                    .font_family(font.name.clone())
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Label::new(font.name.clone())
                                    .size(LabelSize::Small)
                                    .truncate(),
                            )
                            .child(
                                Label::new(action_label)
                                    .size(LabelSize::XSmall)
                                    .color(Color::Accent),
                            ),
                    )
                    .child(
                        Label::new(font.source.label())
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    ),
            )
            .child(
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .child(
                        Button::new(select_id, "Select")
                            .style(ButtonStyle::Subtle)
                            .size(ButtonSize::Compact)
                            .on_click(cx.listener({
                                let font = font.clone();
                                move |panel, _, _, cx| {
                                    panel.select_font(font.clone(), cx);
                                }
                            })),
                    )
                    .child(
                        Button::new(preview_id, "Preview")
                            .style(ButtonStyle::Subtle)
                            .size(ButtonSize::Compact)
                            .on_click(cx.listener({
                                let font = font.clone();
                                move |panel, _, window, cx| {
                                    panel.set_selected_font(&font);
                                    panel.preview_selected_font(window, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new(copy_id, "Copy CSS")
                            .style(ButtonStyle::Subtle)
                            .size(ButtonSize::Compact)
                            .on_click(cx.listener({
                                let font = font.clone();
                                move |panel, _, _, cx| {
                                    panel.set_selected_font(&font);
                                    panel.copy_selected_css(cx);
                                }
                            })),
                    )
                    .when(is_system_font, |this| {
                        this.child(
                            Button::new(editor_id, "Editor")
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .on_click(cx.listener({
                                    let font = font.clone();
                                    move |panel, _, _, cx| {
                                        panel.set_selected_font(&font);
                                        panel.apply_selected_to_buffer(cx);
                                    }
                                })),
                        )
                        .child(
                            Button::new(ui_id, "UI")
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .on_click(cx.listener({
                                    let font = font.clone();
                                    move |panel, _, _, cx| {
                                        panel.set_selected_font(&font);
                                        panel.apply_selected_to_ui(cx);
                                    }
                                })),
                        )
                    })
                    .when(!is_system_font, |this| {
                        this.child(
                            Button::new(add_id, "Add")
                                .style(ButtonStyle::Subtle)
                                .size(ButtonSize::Compact)
                                .on_click(cx.listener(move |panel, _, _, cx| {
                                    panel.set_selected_font(&font);
                                    panel.add_selected_to_project(cx);
                                })),
                        )
                    })
                    .child(
                        Button::new(pin_id, if pinned { "Unpin" } else { "Pin" })
                            .style(ButtonStyle::Subtle)
                            .size(ButtonSize::Compact)
                            .on_click(cx.listener(move |panel, _, _, cx| {
                                if pinned {
                                    panel.unpin_font_action(pin_font.clone(), cx);
                                } else {
                                    panel.pin_font_action(pin_entry.clone(), cx);
                                }
                            })),
                    ),
            )
            .into_any_element()
    }

    fn render_source_filters(
        &self,
        counts: FontSourceCounts,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .h(px(38.))
            .gap_1()
            .items_center()
            .child(
                IconButton::new("font-panel-source-prev", IconName::ChevronLeft)
                    .shape(ui::IconButtonShape::Square)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Previous font groups"))
                    .on_click(cx.listener(|panel, _, _, cx| {
                        panel.scroll_source_tabs(-1.0, cx);
                    })),
            )
            .child(
                h_flex()
                    .id("font-panel-source-filter-scroll")
                    .flex_1()
                    .h_full()
                    .overflow_x_scroll()
                    .overflow_y_hidden()
                    .track_scroll(&self.source_scroll_handle)
                    .child(
                        h_flex()
                            .flex_none()
                            .gap_1()
                            .items_center()
                            .child(self.render_source_filter_button(
                                FontSourceFilter::All,
                                counts.count(FontSourceFilter::All),
                                cx,
                            ))
                            .child(self.render_source_filter_button(
                                FontSourceFilter::System,
                                counts.count(FontSourceFilter::System),
                                cx,
                            ))
                            .child(self.render_source_filter_button(
                                FontSourceFilter::Web,
                                counts.count(FontSourceFilter::Web),
                                cx,
                            )),
                    ),
            )
            .child(
                IconButton::new("font-panel-source-next", IconName::ChevronRight)
                    .shape(ui::IconButtonShape::Square)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Next font groups"))
                    .on_click(cx.listener(|panel, _, _, cx| {
                        panel.scroll_source_tabs(1.0, cx);
                    })),
            )
    }

    fn scroll_source_tabs(&mut self, direction: f32, cx: &mut Context<Self>) {
        scroll_tab_handle(&self.source_scroll_handle, direction);
        cx.notify();
    }

    fn render_preview(
        &self,
        total_matches: usize,
        counts: FontSourceCounts,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let count_label = if let Some(status) = self.status.clone() {
            status
        } else if self.fonts_loaded || self.source_filter == FontSourceFilter::Web {
            font_fraction_label(total_matches, counts.count(self.source_filter))
        } else {
            "loading".into()
        };
        let working_set_label = font_working_set_label(
            self.pinned_font_actions.len(),
            self.recent_font_actions.len(),
        );
        let (readiness_label, readiness_color) = font_readiness_label(
            self.fonts_loaded || self.source_filter == FontSourceFilter::Web,
            counts.count(self.source_filter),
        );
        v_flex()
            .gap_2()
            .p_2()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(Label::new("Fonts").size(LabelSize::Small))
                            .child(
                                Label::new(readiness_label)
                                    .size(LabelSize::XSmall)
                                    .color(readiness_color)
                                    .truncate(),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .when_some(working_set_label, |this, working_set_label| {
                                this.child(
                                    Label::new(working_set_label)
                                        .size(LabelSize::XSmall)
                                        .color(Color::Muted)
                                        .truncate(),
                                )
                            })
                            .child(
                                Label::new(count_label)
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted)
                                    .truncate(),
                            ),
                    ),
            )
            .child(self.filter_editor.clone())
            .child(self.render_source_filters(counts, cx))
    }
}

impl Panel for FontPanel {
    fn persistent_name() -> &'static str {
        "Font Panel"
    }

    fn panel_key() -> &'static str {
        FONT_PANEL_KEY
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
        Some("Font Panel")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(ToggleFocus)
    }

    fn activation_priority(&self) -> u32 {
        9
    }
}

impl Focusable for FontPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.filter_editor.focus_handle(cx)
    }
}

impl EventEmitter<PanelEvent> for FontPanel {}

impl Render for FontPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_system_fonts_loading(cx);
        self.refresh_fonts_if_needed(cx);
        let query = self.query(cx);
        let (fonts, total_matches) = self.matching_fonts(query.as_str(), MAX_FONT_RESULTS);
        let source_counts = FontSourceCounts::from_panel(self);
        let mut font_rows = Vec::with_capacity(fonts.len());
        font_rows.extend(
            fonts
                .into_iter()
                .map(|font| self.render_font_row(font, cx).into_any_element()),
        );
        let recent_font_section = if query.is_empty() {
            self.render_recent_font_section(cx)
        } else {
            None
        };
        let pinned_font_section = if query.is_empty() {
            self.render_pinned_font_section(cx)
        } else {
            None
        };
        let mut content_rows = Vec::with_capacity(
            font_rows.len()
                + usize::from(pinned_font_section.is_some())
                + usize::from(recent_font_section.is_some()),
        );
        if let Some(pinned_font_section) = pinned_font_section {
            content_rows.push(pinned_font_section);
        }
        if let Some(recent_font_section) = recent_font_section {
            content_rows.push(recent_font_section);
        }
        content_rows.extend(font_rows);
        let is_empty = content_rows.is_empty() && total_matches == 0;

        v_flex()
            .id("font-panel")
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().colors().panel_background)
            .child(self.render_preview(total_matches, source_counts, cx))
            .child(
                div()
                    .id("font-panel-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .p_2()
                    .when(is_empty, |this| {
                        this.child(
                            div().h_full().flex().items_center().justify_center().child(
                                Label::new(if self.fonts_loaded {
                                    "No matching fonts"
                                } else {
                                    "Loading fonts"
                                })
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                            ),
                        )
                    })
                    .when(!content_rows.is_empty(), |this| {
                        this.child(v_flex().gap_2().children(content_rows))
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

fn font_search_matches(searchable: &str, query_terms: &[&str]) -> bool {
    query_terms.iter().all(|term| searchable.contains(term))
}

fn load_pinned_font_actions(cx: &App) -> VecDeque<RecentFontEntry> {
    let mut entries = VecDeque::with_capacity(MAX_PINNED_FONT_ACTIONS);
    let Some(json) = KeyValueStore::global(cx)
        .read_kvp(PINNED_FONT_ACTIONS_KEY)
        .ok()
        .flatten()
    else {
        return entries;
    };
    let Ok(state) = serde_json::from_str::<SerializedPinnedFontActions>(&json) else {
        return entries;
    };
    if state.version != PINNED_FONT_ACTIONS_STATE_VERSION {
        return entries;
    }

    entries.extend(
        state
            .entries
            .into_iter()
            .take(MAX_PINNED_FONT_ACTIONS)
            .map(SerializedPinnedFontAction::into_entry),
    );
    entries
}

fn recent_font_action_label(action: RecentFontAction) -> &'static str {
    match action {
        RecentFontAction::Previewed => "previewed",
        RecentFontAction::CopiedCss => "copied CSS",
        RecentFontAction::AppliedEditor => "editor",
        RecentFontAction::AppliedUi => "UI",
        RecentFontAction::AddedProject => "added",
        RecentFontAction::Pinned => "pinned",
    }
}

fn font_history_health_label(count: usize) -> SharedString {
    match count {
        1 => "1 ready".into(),
        _ => {
            let mut text = String::with_capacity(12);
            let _ = write!(text, "{count} ready");
            text.into()
        }
    }
}

fn font_working_set_label(pinned: usize, recent: usize) -> Option<SharedString> {
    if pinned == 0 && recent == 0 {
        return None;
    }

    Some(history_working_set_label(pinned, recent))
}

fn history_working_set_label(pinned: usize, recent: usize) -> SharedString {
    let mut text = String::with_capacity("pins ".len() + 6 + " / recent ".len() + 6);
    let _ = write!(text, "pins {pinned} / recent {recent}");
    text.into()
}

fn font_readiness_label(loaded: bool, total_count: usize) -> (&'static str, Color) {
    if !loaded {
        ("loading", Color::Accent)
    } else if total_count == 0 {
        ("empty", Color::Warning)
    } else {
        ("ready", Color::Success)
    }
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

fn font_element_id(prefix: &str, id: &str) -> String {
    let mut element_id = String::with_capacity(prefix.len() + id.len());
    element_id.push_str(prefix);
    element_id.push_str(id);
    element_id
}

fn font_count_label(label: &str, count: usize) -> String {
    let mut text = String::with_capacity(label.len() + 1 + 6);
    text.push_str(label);
    let _ = write!(text, " {count}");
    text
}

fn font_fraction_label(left: usize, right: usize) -> SharedString {
    let mut text = String::with_capacity(24);
    let _ = write!(text, "{left} / {right}");
    text.into()
}

fn font_status_label(prefix: &str, value: &str) -> SharedString {
    let mut text = String::with_capacity(prefix.len() + value.len());
    text.push_str(prefix);
    text.push_str(value);
    text.into()
}

fn font_status_label_with_suffix(prefix: &str, value: &str, suffix: &str) -> SharedString {
    let mut text = String::with_capacity(prefix.len() + value.len() + suffix.len());
    text.push_str(prefix);
    text.push_str(value);
    text.push_str(suffix);
    text.into()
}

fn font_added_status(font_name: &str, path: &Path) -> SharedString {
    let path = path.to_string_lossy();
    let mut text =
        String::with_capacity("Added ".len() + font_name.len() + " to ".len() + path.len());
    text.push_str("Added ");
    text.push_str(font_name);
    text.push_str(" to ");
    for ch in path.chars() {
        text.push(if ch == '\\' { '/' } else { ch });
    }
    text.into()
}

fn web_font_spec_by_name(name: &str) -> Option<WebFontSpec> {
    let name = google_fonts::GOOGLE_FONT_FAMILIES
        .iter()
        .find(|font_name| font_name.eq_ignore_ascii_case(name))
        .map(|font_name| (*font_name).to_string())
        .or_else(|| custom_web_font_name(name))?;
    Some(WebFontSpec {
        family_query: google_font_family_query(&name),
        variable: css_font_variable_name(&name),
        name,
    })
}

fn custom_web_font_name(query: &str) -> Option<String> {
    let mut name = String::with_capacity(query.len());
    for word in query
        .split(|character: char| !(character.is_alphanumeric() || character == ' '))
        .flat_map(|segment| segment.split_whitespace())
        .filter(|word| !word.is_empty())
        .take(6)
    {
        if !name.is_empty() {
            name.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            name.extend(first.to_uppercase());
            name.push_str(chars.as_str());
        }
    }

    (!name.is_empty()).then_some(name)
}

fn google_font_family_query(name: &str) -> String {
    let mut query = String::with_capacity(name.len());
    for word in name.split_whitespace() {
        if !query.is_empty() {
            query.push('+');
        }
        query.push_str(word);
    }
    query
}

fn css_font_variable_name(name: &str) -> String {
    let mut variable = String::with_capacity(name.len());
    let mut needs_separator = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if needs_separator && !variable.is_empty() {
                variable.push('-');
            }
            variable.push(character.to_ascii_lowercase());
            needs_separator = false;
        } else {
            needs_separator = !variable.is_empty();
        }
    }
    if variable.is_empty() {
        "web-font".to_string()
    } else {
        variable
    }
}

fn local_font_preview_url(
    font_name: &str,
    source: FontSource,
    web_font: Option<&WebFontSpec>,
    sample_text: &str,
) -> Option<String> {
    let preview_dir = repo_root().join("target").join("font-previews");
    std_fs::create_dir_all(&preview_dir).ok()?;
    let preview_path = preview_dir.join(format!("{}.html", font_preview_file_stem(font_name)));
    let html = font_preview_html(font_name, source, web_font, sample_text);
    std_fs::write(&preview_path, html).ok()?;
    Url::from_file_path(preview_path)
        .ok()
        .map(|url| url.to_string())
}

fn font_preview_file_stem(font_name: &str) -> String {
    font_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn font_preview_html(
    font_name: &str,
    source: FontSource,
    web_font: Option<&WebFontSpec>,
    sample_text: &str,
) -> String {
    let title = escape_html(font_name);
    let sample = escape_html(sample_text);
    let import = web_font
        .map(|font| {
            format!(
                r#"<link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family={}:wght@400;500;600;700&display=swap" rel="stylesheet">"#,
                font.family_query
            )
        })
        .unwrap_or_default();
    let source_label = source.label();

    format!(
        r#"<!doctype html>
<html lang="en" class="dark">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title} - Zed font preview</title>
  {import}
  <style>
    :root {{
      color-scheme: dark;
      --bg: #09090b;
      --panel: #101113;
      --border: #272a2f;
      --fg: #f4f4f5;
      --muted: #a1a1aa;
      --accent: #3fb950;
      --accent-soft: rgba(63, 185, 80, .16);
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      background: var(--bg);
      color: var(--fg);
      font-family: "{title}", Inter, ui-sans-serif, system-ui, sans-serif;
      padding: 32px;
    }}
    main {{
      max-width: 920px;
      margin: 0 auto;
      display: grid;
      gap: 18px;
    }}
    header, section {{
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--panel);
      padding: 20px;
    }}
    .pill {{
      display: inline-flex;
      width: max-content;
      border: 1px solid var(--border);
      border-radius: 999px;
      padding: 2px 8px;
      color: var(--accent);
      background: var(--accent-soft);
      font-size: 12px;
      margin-bottom: 12px;
    }}
    h1 {{ margin: 0; font-size: 48px; letter-spacing: 0; line-height: 1.05; }}
    p {{ margin: 0; color: var(--muted); font-size: 15px; }}
    .sample-xl {{ font-size: 64px; line-height: 1; font-weight: 700; letter-spacing: 0; }}
    .sample-lg {{ font-size: 28px; line-height: 1.25; font-weight: 600; }}
    .sample-body {{ font-size: 17px; line-height: 1.7; }}
    .grid {{ display: grid; gap: 12px; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); }}
    .card {{ border: 1px solid var(--border); border-radius: 8px; padding: 14px; }}
    .label {{ color: var(--muted); font-size: 12px; margin-bottom: 8px; font-family: Inter, ui-sans-serif, system-ui, sans-serif; }}
    code {{ color: var(--accent); font-family: ui-monospace, SFMono-Regular, Consolas, monospace; }}
  </style>
</head>
<body>
  <main>
    <header>
      <div class="pill">{source_label}</div>
      <h1>{title}</h1>
      <p>Browser-rendered specimen for checking scale, weight, code-adjacent text, and product UI copy.</p>
    </header>
    <section class="sample-xl">Aa Bb Cc 012345</section>
    <section class="sample-lg">{sample}</section>
    <section class="sample-body">The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs. Sphinx of black quartz, judge my vow.</section>
    <section class="grid">
      <div class="card"><div class="label">Regular</div><div style="font-weight:400">Dashboard, editor, panel, preview</div></div>
      <div class="card"><div class="label">Medium</div><div style="font-weight:500">Dashboard, editor, panel, preview</div></div>
      <div class="card"><div class="label">Semibold</div><div style="font-weight:600">Dashboard, editor, panel, preview</div></div>
      <div class="card"><div class="label">Bold</div><div style="font-weight:700">Dashboard, editor, panel, preview</div></div>
    </section>
    <section class="sample-body"><code>font-family: "{title}", system-ui, sans-serif;</code></section>
  </main>
</body>
</html>"#
    )
}

fn add_web_font_to_project(root: &Path, web_font: &WebFontSpec) -> std::io::Result<PathBuf> {
    let css_path = project_font_css_path(root);
    if let Some(parent) = css_path.parent() {
        std_fs::create_dir_all(parent)?;
    }

    let mut css = std_fs::read_to_string(&css_path).unwrap_or_default();
    let import = web_font_import(web_font);

    if !css.contains(&import) {
        css = if css.trim().is_empty() {
            format!("{import}\n")
        } else {
            format!("{import}\n{css}")
        };
    }

    let variable = format!("--font-{}", web_font.variable);
    if !css.contains(&variable) {
        css.push_str(&format!(
            "\n:root {{\n  {variable}: \"{}\", system-ui, sans-serif;\n}}\n",
            web_font.name
        ));
    }

    std_fs::write(&css_path, css)?;
    Ok(css_path)
}

fn web_font_import(web_font: &WebFontSpec) -> String {
    format!(
        "@import url(\"https://fonts.googleapis.com/css2?family={}:wght@400;500;600;700&display=swap\");",
        web_font.family_query
    )
}

fn web_font_css_snippet(web_font: &WebFontSpec) -> String {
    format!(
        "{}\n\n:root {{\n  --font-{}: \"{}\", system-ui, sans-serif;\n}}\n\n.font-{} {{\n  font-family: var(--font-{});\n}}",
        web_font_import(web_font),
        web_font.variable,
        web_font.name,
        web_font.variable,
        web_font.variable
    )
}

fn system_font_css_snippet(font_name: &str) -> String {
    format!("font-family: \"{font_name}\", system-ui, sans-serif;")
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

fn project_font_css_path(root: &Path) -> PathBuf {
    for candidate in [
        root.join("src").join("app").join("globals.css"),
        root.join("app").join("globals.css"),
        root.join("src").join("styles").join("globals.css"),
        root.join("styles").join("globals.css"),
        root.join("src").join("index.css"),
        root.join("index.css"),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }

    if root.join("src").join("app").is_dir() {
        root.join("src").join("app").join("globals.css")
    } else if root.join("app").is_dir() {
        root.join("app").join("globals.css")
    } else if root.join("src").is_dir() {
        root.join("src").join("styles").join("globals.css")
    } else {
        root.join("styles").join("globals.css")
    }
}
