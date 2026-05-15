use agent_client_protocol::schema as acp;
use agent_ui::AgentPanel;
use anyhow::{Context as _, Result, anyhow};
use base64::Engine as _;
use editor::Editor;
use gpui::{
    Action, Anchor, App, AppContext as _, Bounds, ClipboardItem, Context, Entity, EventEmitter,
    FocusHandle, Focusable, Image as GpuiImage, Pixels, Render, SharedString, Subscription, Task,
    WeakEntity, Window, canvas, point, size,
};
#[cfg(target_os = "windows")]
use gpui::{AsyncApp, EntityId, ImageFormat as GpuiImageFormat};
use menu::Confirm;
use paths::data_dir;
use project::{Project, ProjectEntryId, ProjectPath};
use serde_json::Value;
use std::{
    cell::{Cell, RefCell},
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[cfg(target_os = "windows")]
use std::{io::Cursor, num::NonZeroIsize};
use ui::{
    Color, ContextMenu, ContextMenuEntry, IconButton, IconName, IconSize, Label, LabelSize,
    PopoverMenu, Tooltip, prelude::*,
};
use workspace::item::{
    Item, ItemBufferKind, ItemEvent, PaneTabBarControls, ProjectItem as WorkspaceProjectItem,
    ProjectItemKind, TabContentParams, WorkspaceScreenKind,
};
use workspace::notifications::NotificationId;
use workspace::{NewWebPreview, Pane, Toast, Workspace, WorkspaceId};

#[cfg(target_os = "windows")]
use crate::windows_visual_webview::WindowsVisualWebView;
use crate::{OpenPreview, OpenPreviewToTheSide};

#[cfg(target_os = "windows")]
use gpui_windows::window_has_focused_webview;
#[cfg(target_os = "windows")]
use image::{ImageFormat as ExternalImageFormat, RgbaImage, imageops, imageops::FilterType};
#[cfg(target_os = "windows")]
use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, ClientToScreen,
        CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC,
        GetDIBits, HBITMAP, HGDIOBJ, ReleaseDC, SRCCOPY, SelectObject,
    },
};
const DEFAULT_WEB_PREVIEW_URL: &str = "https://www.google.com/";
const GOOGLE_SEARCH_URL: &str = "https://www.google.com/search";
const BOOKMARKS_FILE_NAME: &str = "bookmarks.json";

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreviewWorkspaceContext {
    workspace_id: Option<WorkspaceId>,
    root_path: Option<PathBuf>,
    root_name: SharedString,
    preview_key: SharedString,
    profile_dir: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DetectedExtension {
    browser: SharedString,
    id: SharedString,
    name: SharedString,
    path: PathBuf,
    icon_path: Option<PathBuf>,
    supports_chromium_loading: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreviewFileKind {
    Video,
    Audio,
    Document,
}

impl PreviewFileKind {
    fn label(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Document => "document",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Video => IconName::PlayOutlined,
            Self::Audio => IconName::AudioOn,
            Self::Document => IconName::FileDoc,
        }
    }
}

pub struct WebPreviewFileItem {
    project_path: ProjectPath,
    entry_id: Option<ProjectEntryId>,
    absolute_path: PathBuf,
    title: SharedString,
    kind: PreviewFileKind,
}

#[derive(Clone, Debug)]
enum PreviewLoadState {
    Loading,
    Ready,
    Error(SharedString),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreviewViewportMode {
    Full,
    Fixed {
        label: &'static str,
        width: u32,
        height: u32,
    },
}

impl PreviewViewportMode {
    const FULL: Self = Self::Full;
    const IPHONE_15: Self = Self::Fixed {
        label: "iPhone 15",
        width: 393,
        height: 852,
    };
    const IPAD_AIR: Self = Self::Fixed {
        label: "iPad Air",
        width: 820,
        height: 1180,
    };
    const LAPTOP: Self = Self::Fixed {
        label: "Laptop",
        width: 1280,
        height: 900,
    };

    fn label(self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::Fixed { label, .. } => label,
        }
    }

    fn dimensions(self) -> Option<(u32, u32)> {
        match self {
            Self::Full => None,
            Self::Fixed { width, height, .. } => Some((width, height)),
        }
    }

    fn rotated(self) -> Option<Self> {
        match self {
            Self::Full => None,
            Self::Fixed {
                label,
                width,
                height,
            } => Some(Self::Fixed {
                label,
                width: height,
                height: width,
            }),
        }
    }

    fn snapshot(self) -> Value {
        let (mode, width, height) = match self {
            Self::Full => ("full", None, None),
            Self::Fixed { width, height, .. } => ("fixed", Some(width), Some(height)),
        };

        serde_json::json!({
            "mode": mode,
            "label": self.label(),
            "width": width,
            "height": height,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AgentBrowserActionPermission {
    ReadOnly,
    Interactive,
}

impl AgentBrowserActionPermission {
    fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "Read-only",
            Self::Interactive => "Interactive allowed",
        }
    }

    fn interactive_enabled(self) -> bool {
        matches!(self, Self::Interactive)
    }

    fn snapshot(self) -> Value {
        serde_json::json!({
            "mode": match self {
                Self::ReadOnly => "read_only",
                Self::Interactive => "interactive",
            },
            "label": self.label(),
            "interactive_enabled": self.interactive_enabled(),
        })
    }
}

const READ_ONLY_AGENT_BROWSER_ACTIONS: &[&str] = &[
    "copy_session_info",
    "copy_session_json",
    "copy_workspace_session_inventory_json",
    "send_workspace_session_inventory_to_agent",
    "copy_page_diagnostics",
    "send_page_diagnostics_to_agent",
    "copy_runtime_events",
    "send_runtime_events_to_agent",
    "copy_dom_snapshot",
    "send_dom_snapshot_to_agent",
    "copy_action_targets",
    "send_action_targets_to_agent",
    "copy_readiness_probe",
    "send_readiness_probe_to_agent",
    "copy_wait_contract",
    "send_wait_contract_to_agent",
    "copy_interaction_plan",
    "send_interaction_plan_to_agent",
    "copy_interaction_preflight",
    "send_interaction_preflight_to_agent",
    "copy_interaction_receipt_template",
    "send_interaction_receipt_template_to_agent",
    "copy_interaction_action_request",
    "send_interaction_action_request_to_agent",
    "copy_blocked_interaction_receipt",
    "send_blocked_interaction_receipt_to_agent",
    "copy_successful_interaction_receipt",
    "send_successful_interaction_receipt_to_agent",
    "copy_agent_browser_status_packet",
    "send_agent_browser_status_packet_to_agent",
    "copy_agent_browser_executor_readiness",
    "send_agent_browser_executor_readiness_to_agent",
    "copy_agent_browser_noop_executor_attempt",
    "send_agent_browser_noop_executor_attempt_to_agent",
    "copy_agent_browser_qa_runbook",
    "send_agent_browser_qa_runbook_to_agent",
    "copy_agent_plugin_catalog",
    "send_agent_plugin_catalog_to_agent",
    "take_screenshot",
    "capture_selected_area_screenshot",
    "annotate_screenshot",
    "inspect_element",
    "open_devtools",
    "set_responsive_viewport",
];

const INTERACTIVE_AGENT_BROWSER_ACTIONS: &[&str] = &[
    "open_url",
    "reload",
    "go_back",
    "go_forward",
    "click",
    "type_text",
    "press_key",
    "scroll",
    "set_viewport",
    "clear_data",
    "clear_cache",
];

#[derive(Clone, Debug)]
pub(crate) enum BrowserEvent {
    UrlChanged(String),
    TitleChanged(String),
    NavigationStarted,
    NavigationCompleted,
    IpcMessage(String),
    MountFailed(String),
}

#[derive(Clone, Debug)]
enum BrowserAgentPayload {
    InspectElement {
        url: String,
        title: Option<String>,
        selector: Option<String>,
        tag: Option<String>,
        id: Option<String>,
        classes: Vec<String>,
        text: Option<String>,
        href: Option<String>,
        src: Option<String>,
        rect: Option<BrowserRect>,
        css: Option<String>,
        html: Option<String>,
    },
}

#[derive(Clone, Copy, Debug)]
struct BrowserRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[cfg(target_os = "windows")]
struct NativeWebPreview {
    webview: WindowsVisualWebView,
}

#[cfg(target_os = "windows")]
impl NativeWebPreview {
    fn load_url(&self, url: &str) -> Result<()> {
        self.webview.load_url(url)
    }

    fn reload(&self) -> Result<()> {
        self.webview.reload()
    }

    fn evaluate_script(&self, script: &str) -> Result<()> {
        self.webview.evaluate_script(script)
    }

    fn zoom(&self, zoom_factor: f64) -> Result<()> {
        self.webview.zoom(zoom_factor)
    }

    fn clear_all_browsing_data(&self) -> Result<()> {
        self.webview.clear_all_browsing_data()
    }

    fn open_devtools(&self) {
        self.webview.open_devtools()
    }

    fn focus_parent(&self) -> Result<()> {
        self.webview.focus_parent()
    }

    fn focus_page(&self) -> Result<()> {
        self.webview.focus_page()
    }

    fn set_visible(&mut self, visible: bool) -> Result<()> {
        self.webview.set_visible(visible)
    }

    fn sync_bounds(&mut self, bounds: Bounds<Pixels>, scale_factor: f32) -> Result<()> {
        self.webview
            .set_bounds(client_rect_for_bounds(bounds, scale_factor), scale_factor)
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
struct RawParentWindow {
    hwnd: NonZeroIsize,
}

#[cfg(target_os = "windows")]
impl RawParentWindow {
    fn from_window(window: &Window) -> Result<Self> {
        let handle = HasWindowHandle::window_handle(window)?;
        let RawWindowHandle::Win32(raw_handle) = handle.as_raw() else {
            return Err(anyhow!("Unsupported window handle for web preview"));
        };
        Ok(Self {
            hwnd: raw_handle.hwnd,
        })
    }

    fn as_hwnd(&self) -> HWND {
        HWND(self.hwnd.get() as *mut _)
    }
}

#[cfg(target_os = "windows")]
impl HasWindowHandle for RawParentWindow {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        let handle = Win32WindowHandle::new(self.hwnd);
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)) })
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct NativePreviewMountRequest {
    entity_id: EntityId,
    parent_window: RawParentWindow,
    profile_dir: PathBuf,
    initial_url: String,
    zoom_factor: f64,
    scale_factor: f32,
    initially_visible: bool,
    host_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    native_mount_requested: Rc<Cell<bool>>,
    native_preview: Rc<RefCell<Option<NativeWebPreview>>>,
}

pub struct WebPreviewView {
    workspace: WeakEntity<Workspace>,
    workspace_context: PreviewWorkspaceContext,
    session_id: SharedString,
    focus_handle: FocusHandle,
    project_item: Option<Entity<WebPreviewFileItem>>,
    url_editor: Entity<Editor>,
    url_editor_focus_handle: FocusHandle,
    url_editor_focus_requested: Rc<Cell<bool>>,
    page_title: Option<SharedString>,
    active_url: SharedString,
    bookmarks: Vec<String>,
    detected_extensions: Vec<DetectedExtension>,
    extensions_scanned: bool,
    load_state: PreviewLoadState,
    layout_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    host_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    #[cfg(target_os = "macos")]
    last_applied_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    native_mount_requested: Rc<Cell<bool>>,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    deferred_ipc_messages: Vec<String>,
    ipc_flush_scheduled: bool,
    latest_page_diagnostics: Option<Value>,
    latest_runtime_events: Option<Value>,
    latest_dom_snapshot: Option<Value>,
    latest_action_targets: Option<Value>,
    latest_readiness_probe: Option<Value>,
    latest_wait_contract: Option<Value>,
    latest_interaction_plan: Option<Value>,
    latest_interaction_preflight: Option<Value>,
    latest_interaction_receipt_template: Option<Value>,
    latest_interaction_action_request: Option<Value>,
    latest_blocked_interaction_receipt: Option<Value>,
    latest_successful_interaction_receipt: Option<Value>,
    latest_agent_browser_status_packet: Option<Value>,
    latest_agent_browser_executor_readiness: Option<Value>,
    latest_agent_browser_noop_executor_attempt: Option<Value>,
    latest_agent_browser_reload_executor_attempt: Option<Value>,
    latest_agent_browser_clear_data_executor_attempt: Option<Value>,
    latest_agent_browser_qa_runbook: Option<Value>,
    latest_agent_plugin_catalog: Option<Value>,
    latest_annotated_screenshot: Option<Value>,
    event_pump_task: Option<Task<()>>,
    native_mount_task: Option<Task<()>>,
    zoom_factor: f64,
    viewport_mode: PreviewViewportMode,
    agent_action_permission: AgentBrowserActionPermission,
    is_active_item: bool,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    native_preview: Rc<RefCell<Option<NativeWebPreview>>>,
    _subscriptions: Vec<Subscription>,
}

impl WebPreviewView {
    pub fn register(workspace: &mut Workspace, _window: &mut Window, _cx: &mut Context<Workspace>) {
        workspace.register_action(move |workspace, _: &NewWebPreview, window, cx| {
            Self::open_new_in_active_pane(workspace, window, cx);
        });

        workspace.register_action(move |workspace, _: &OpenPreview, window, cx| {
            Self::open_in_active_pane(workspace, window, cx);
        });

        workspace.register_action(move |workspace, _: &OpenPreviewToTheSide, window, cx| {
            Self::open_in_side_pane(workspace, window, cx);
        });
    }

    fn open_in_active_pane(
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let view = Self::open_or_create(workspace, window, cx);
        workspace.active_pane().update(cx, |pane, cx| {
            if let Some(existing_view_idx) = Self::find_existing_preview_item_idx(pane, &view, cx) {
                pane.activate_item(existing_view_idx, true, true, window, cx);
            } else {
                pane.add_item(Box::new(view.clone()), true, true, None, window, cx);
            }
        });
        cx.notify();
    }

    pub fn open_url_in_active_pane(
        workspace: &mut Workspace,
        url: &str,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let view = Self::open_or_create(workspace, window, cx);
        workspace.active_pane().update(cx, |pane, cx| {
            let preview = Self::find_existing_preview_item(pane, &view, cx).unwrap_or_else(|| {
                pane.add_item(Box::new(view.clone()), true, true, None, window, cx);
                view.clone()
            });
            preview.update(cx, |this, cx| {
                this.load_requested_url(url, window, cx);
            });
            if let Some(existing_view_idx) = pane.index_for_item(&preview) {
                pane.activate_item(existing_view_idx, true, true, window, cx);
            }
        });
        cx.notify();
    }

    pub fn ensure_startup_preview(
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let _ = (workspace, window, cx);
    }

    fn open_new_in_active_pane(
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let view = Self::open_or_create(workspace, window, cx);
        workspace.active_pane().update(cx, |pane, cx| {
            pane.add_item(Box::new(view.clone()), true, true, None, window, cx);
        });
        cx.notify();
    }

    fn open_in_side_pane(
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let view = Self::open_or_create(workspace, window, cx);
        let pane = workspace
            .find_pane_in_direction(workspace::SplitDirection::Right, cx)
            .unwrap_or_else(|| {
                workspace.split_pane(
                    workspace.active_pane().clone(),
                    workspace::SplitDirection::Right,
                    window,
                    cx,
                )
            });
        pane.update(cx, |pane, cx| {
            if let Some(existing_view_idx) = Self::find_existing_preview_item_idx(pane, &view, cx) {
                pane.activate_item(existing_view_idx, true, true, window, cx);
            } else {
                pane.add_item(Box::new(view.clone()), false, false, None, window, cx);
            }
        });
        cx.notify();
    }

    fn open_or_create(
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Entity<Self> {
        let workspace_context = Self::workspace_context(workspace, cx);
        let weak_workspace = workspace.weak_handle();

        cx.new(|cx| {
            Self::new_for_url(
                weak_workspace.clone(),
                workspace_context.clone(),
                DEFAULT_WEB_PREVIEW_URL.to_string(),
                None,
                None,
                window,
                cx,
            )
        })
    }

    fn new_for_url(
        workspace: WeakEntity<Workspace>,
        workspace_context: PreviewWorkspaceContext,
        current_url: String,
        title: Option<SharedString>,
        project_item: Option<Entity<WebPreviewFileItem>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let url_editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Search Google or enter a URL", window, cx);
            editor.set_text(current_url.as_str(), window, cx);
            editor
        });
        let url_editor_focus_handle = url_editor.focus_handle(cx);

        let browser_events = Arc::new(Mutex::new(Vec::new()));
        let mut this = Self {
            workspace,
            workspace_context: workspace_context.clone(),
            session_id: format!("web-preview-{}", cx.entity_id().as_non_zero_u64()).into(),
            focus_handle: cx.focus_handle(),
            project_item,
            url_editor,
            url_editor_focus_handle,
            url_editor_focus_requested: Rc::new(Cell::new(false)),
            page_title: title,
            active_url: current_url.into(),
            bookmarks: load_bookmarks(&workspace_context.profile_dir).unwrap_or_default(),
            detected_extensions: Vec::new(),
            extensions_scanned: false,
            load_state: PreviewLoadState::Loading,
            layout_bounds: Rc::new(RefCell::new(None)),
            host_bounds: Rc::new(RefCell::new(None)),
            #[cfg(target_os = "macos")]
            last_applied_bounds: Rc::new(RefCell::new(None)),
            native_mount_requested: Rc::new(Cell::new(false)),
            browser_events,
            deferred_ipc_messages: Vec::new(),
            ipc_flush_scheduled: false,
            latest_page_diagnostics: None,
            latest_runtime_events: None,
            latest_dom_snapshot: None,
            latest_action_targets: None,
            latest_readiness_probe: None,
            latest_wait_contract: None,
            latest_interaction_plan: None,
            latest_interaction_preflight: None,
            latest_interaction_receipt_template: None,
            latest_interaction_action_request: None,
            latest_blocked_interaction_receipt: None,
            latest_successful_interaction_receipt: None,
            latest_agent_browser_status_packet: None,
            latest_agent_browser_executor_readiness: None,
            latest_agent_browser_noop_executor_attempt: None,
            latest_agent_browser_reload_executor_attempt: None,
            latest_agent_browser_clear_data_executor_attempt: None,
            latest_agent_browser_qa_runbook: None,
            latest_agent_plugin_catalog: None,
            latest_annotated_screenshot: None,
            event_pump_task: None,
            native_mount_task: None,
            zoom_factor: 1.0,
            viewport_mode: PreviewViewportMode::FULL,
            agent_action_permission: AgentBrowserActionPermission::ReadOnly,
            is_active_item: false,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            native_preview: Rc::new(RefCell::new(None)),
            _subscriptions: vec![],
        };
        this.start_event_pump(window, cx);
        this
    }

    fn start_event_pump(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let browser_events = self.browser_events.clone();
        self.event_pump_task = Some(cx.spawn(async move |this, cx| {
            loop {
                if this.upgrade().is_none() {
                    break;
                }

                let has_events = browser_events
                    .lock()
                    .map(|events| !events.is_empty())
                    .unwrap_or(false);

                if has_events {
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(16))
                        .await;
                } else {
                    cx.background_executor()
                        .timer(Duration::from_millis(50))
                        .await;
                }
            }
        }));
    }

    fn find_existing_preview_item_idx(
        pane: &Pane,
        view: &Entity<WebPreviewView>,
        cx: &App,
    ) -> Option<usize> {
        Self::find_existing_preview_item(pane, view, cx)
            .and_then(|candidate| pane.index_for_item(&candidate))
    }

    fn find_existing_preview_item(
        pane: &Pane,
        view: &Entity<WebPreviewView>,
        cx: &App,
    ) -> Option<Entity<WebPreviewView>> {
        let preview_key = view.read(cx).workspace_context.preview_key.clone();
        pane.items_of_type::<WebPreviewView>()
            .find(|candidate| candidate.read(cx).workspace_context.preview_key == preview_key)
    }

    fn workspace_context(workspace: &Workspace, cx: &App) -> PreviewWorkspaceContext {
        let project = workspace.project().read(cx);
        let root = project.visible_worktrees(cx).next();
        let root_path = root
            .as_ref()
            .map(|worktree| worktree.read(cx).abs_path().as_ref().to_path_buf());
        let root_name = root
            .as_ref()
            .map(|worktree| worktree.read(cx).root_name().as_unix_str().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "Workspace".to_string());

        let preview_slug = root_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| slugify(&name.to_string_lossy()))
            .filter(|slug| !slug.is_empty())
            .or_else(|| {
                workspace
                    .database_id()
                    .map(|id| format!("workspace-{id:?}"))
            })
            .unwrap_or_else(|| "workspace".to_string());

        PreviewWorkspaceContext {
            workspace_id: workspace.database_id(),
            root_path,
            root_name: root_name.into(),
            preview_key: preview_slug.clone().into(),
            profile_dir: data_dir()
                .join("web_preview")
                .join("profiles")
                .join(preview_slug),
        }
    }

    fn current_url_text(&self, cx: &App) -> String {
        self.url_editor.read(cx).text(cx).trim().to_string()
    }

    fn current_tab_title(&self) -> SharedString {
        self.page_title
            .clone()
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| display_title_from_url(&self.active_url).into())
    }

    fn bookmarks_path(&self) -> PathBuf {
        self.workspace_context.profile_dir.join(BOOKMARKS_FILE_NAME)
    }

    fn is_active_url_bookmarked(&self) -> bool {
        self.bookmarks
            .iter()
            .any(|bookmark| bookmark == self.active_url.as_ref())
    }

    fn persist_bookmarks(&self) -> Result<()> {
        fs::create_dir_all(&self.workspace_context.profile_dir).with_context(|| {
            format!(
                "Failed to prepare {}",
                self.workspace_context.profile_dir.display()
            )
        })?;
        let data = serde_json::to_vec_pretty(&self.bookmarks)
            .with_context(|| "Failed to serialize web preview bookmarks")?;
        fs::write(self.bookmarks_path(), data)
            .with_context(|| "Failed to save web preview bookmarks")?;
        Ok(())
    }

    fn confirm_navigation(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        self.url_editor_focus_requested.set(false);
        self.navigate_to_input(window, cx);
        let preview_focus = self.focus_handle(cx);
        window.focus(&preview_focus, cx);
        cx.emit(ItemEvent::UpdateTab);
        cx.notify();
    }

    #[cfg(target_os = "windows")]
    fn release_native_preview_focus(&self) {
        let borrow = self.native_preview.borrow();
        if let Some(preview) = borrow.as_ref() {
            let _ = preview.focus_parent();
        }
    }

    #[cfg(target_os = "macos")]
    fn release_native_preview_focus(&self) {
        let borrow = self.native_preview.borrow();
        if let Some(preview) = borrow.as_ref() {
            let _ = preview.webview.focus_parent();
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn release_native_preview_focus(&self) {}

    #[cfg(target_os = "windows")]
    fn should_focus_native_preview_page(&self, window: &Window) -> bool {
        self.is_active_item
            && window.is_window_active()
            && self.focus_handle.is_focused(window)
            && !self.url_editor_focus_handle.is_focused(window)
            && !self.url_editor_focus_requested.get()
    }

    #[cfg(target_os = "windows")]
    fn native_preview_has_keyboard_focus(&self, window: &Window) -> bool {
        RawParentWindow::from_window(window)
            .map(|parent_window| window_has_focused_webview(parent_window.as_hwnd()))
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "windows"))]
    fn native_preview_has_keyboard_focus(&self, _window: &Window) -> bool {
        false
    }

    #[cfg(target_os = "windows")]
    fn focus_native_preview_page(&self) {
        let borrow = self.native_preview.borrow();
        if let Some(preview) = borrow.as_ref() {
            let _ = preview.focus_page();
        }
    }

    #[cfg(target_os = "windows")]
    fn sync_native_preview_window_activation(&mut self, window: &mut Window) {
        if self.native_preview.borrow().is_none() {
            return;
        }
        {
            let mut native_preview = self.native_preview.borrow_mut();
            let Some(preview) = native_preview.as_mut() else {
                return;
            };

            let should_be_visible = self.is_active_item;
            let _ = preview.set_visible(should_be_visible);
            if should_be_visible && let Some(bounds) = self.host_bounds.borrow().as_ref().copied() {
                let _ = preview.sync_bounds(bounds, window.scale_factor());
            }
        }

        if self.should_focus_native_preview_page(window)
            && !self.native_preview_has_keyboard_focus(window)
        {
            self.focus_native_preview_page();
        }
    }

    fn activate_url_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus_handle = self.url_editor_focus_handle.clone();
        self.url_editor_focus_requested.set(true);
        #[cfg(target_os = "windows")]
        let should_select_all = RawParentWindow::from_window(window)
            .map(|parent_window| window_has_focused_webview(parent_window.as_hwnd()))
            .unwrap_or_else(|_| !focus_handle.is_focused(window));
        #[cfg(not(target_os = "windows"))]
        let should_select_all = !focus_handle.is_focused(window);

        self.release_native_preview_focus();
        let preview_focus_handle = self.focus_handle(cx);
        window.focus(&preview_focus_handle, cx);
        cx.defer_in(window, move |this, window, cx| {
            window.focus(&focus_handle, cx);
            if should_select_all {
                this.url_editor.update(cx, |editor, cx| {
                    editor.select_all(&editor::actions::SelectAll, window, cx);
                });
            }
        });
    }

    fn navigate_to_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(url) = normalized_url(&self.current_url_text(cx)) else {
            self.load_state = PreviewLoadState::Error("Enter a valid URL or search query.".into());
            cx.notify();
            return;
        };

        self.active_url = url.to_string().into();
        self.page_title = None;
        if let Err(error) = self.load_url(url.as_str(), window, cx) {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        }
        cx.emit(ItemEvent::UpdateTab);
        cx.notify();
    }

    fn load_requested_url(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(url) = normalized_url(url) else {
            self.load_state = PreviewLoadState::Error("Enter a valid URL or search query.".into());
            cx.notify();
            return;
        };

        let url = url.to_string();
        self.url_editor.update(cx, |editor, cx| {
            editor.set_text(url.as_str(), window, cx);
        });
        self.active_url = url.clone().into();
        self.page_title = None;
        if let Err(error) = self.load_url(url.as_str(), window, cx) {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        }
        cx.emit(ItemEvent::UpdateTab);
        cx.notify();
    }

    #[allow(dead_code)]
    fn reset_native_preview(&mut self) {
        if let Some(mut preview) = self.native_preview.borrow_mut().take() {
            let _ = preview.set_visible(false);
        }
        self.native_mount_requested.set(false);
    }

    fn reload_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.load_state = PreviewLoadState::Loading;
        if let Err(error) = self.reload_webview(window, cx) {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        }
        cx.notify();
    }

    fn toggle_bookmark_for_active_url(&mut self, cx: &mut Context<Self>) {
        let url = self.active_url.to_string();
        if url.trim().is_empty() {
            return;
        }

        let message =
            if let Some(index) = self.bookmarks.iter().position(|bookmark| bookmark == &url) {
                self.bookmarks.remove(index);
                "Removed bookmark"
            } else {
                self.bookmarks.push(url);
                self.bookmarks.sort();
                self.bookmarks.dedup();
                "Bookmarked page"
            };

        if let Err(error) = self.persist_bookmarks() {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        } else {
            self.show_toast(message, cx);
        }
        cx.notify();
    }

    fn browser_session_info(&self, window: &Window) -> String {
        let load_state = match &self.load_state {
            PreviewLoadState::Loading => "loading".to_string(),
            PreviewLoadState::Ready => "ready".to_string(),
            PreviewLoadState::Error(error) => format!("error: {error}"),
        };
        let root_path = self
            .workspace_context
            .root_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string());
        let visible_bounds = self
            .host_bounds
            .borrow()
            .as_ref()
            .map(|bounds| {
                format!(
                    "{:.0}x{:.0} at {:.0},{:.0}",
                    bounds.size.width.as_f32(),
                    bounds.size.height.as_f32(),
                    bounds.origin.x.as_f32(),
                    bounds.origin.y.as_f32()
                )
            })
            .unwrap_or_else(|| "not mounted".to_string());

        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let native_preview_mounted = self.native_preview.borrow().is_some();
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let native_preview_mounted = false;

        #[cfg(target_os = "windows")]
        let native_backend = "webview2-composition";
        #[cfg(target_os = "macos")]
        let native_backend = "wkwebview";
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let native_backend = "unavailable";

        format!(
            concat!(
                "Web Preview Session\n",
                "session_id: {session_id}\n",
                "title: {title}\n",
                "url: {url}\n",
                "load_state: {load_state}\n",
                "native_backend: {native_backend}\n",
                "workspace: {workspace}\n",
                "workspace_key: {workspace_key}\n",
                "root_path: {root_path}\n",
                "profile_dir: {profile_dir}\n",
                "zoom: {zoom:.0}%\n",
                "active_item: {active_item}\n",
                "native_preview_mounted: {native_preview_mounted}\n",
                "native_keyboard_focus: {native_keyboard_focus}\n",
                "visible_bounds: {visible_bounds}\n",
                "viewport: {viewport}\n",
                "agent_action_policy: {agent_action_policy}\n",
                "bookmarks: {bookmark_count}\n",
                "project_file_preview: {project_file_preview}\n"
            ),
            session_id = self.session_id.as_ref(),
            title = self.current_tab_title(),
            url = self.active_url.as_ref(),
            load_state = load_state,
            native_backend = native_backend,
            workspace = self.workspace_context.root_name.as_ref(),
            workspace_key = self.workspace_context.preview_key.as_ref(),
            root_path = root_path,
            profile_dir = self.workspace_context.profile_dir.display(),
            zoom = self.zoom_factor * 100.0,
            active_item = self.is_active_item,
            native_preview_mounted = native_preview_mounted,
            native_keyboard_focus = self.native_preview_has_keyboard_focus(window),
            visible_bounds = visible_bounds,
            viewport = self.viewport_label(),
            agent_action_policy = self.agent_action_permission.label(),
            bookmark_count = self.bookmarks.len(),
            project_file_preview = self.project_item.is_some(),
        )
    }

    fn browser_session_snapshot(&self, window: &Window) -> Value {
        let (load_state, load_error) = match &self.load_state {
            PreviewLoadState::Loading => ("loading", None),
            PreviewLoadState::Ready => ("ready", None),
            PreviewLoadState::Error(error) => ("error", Some(error.to_string())),
        };
        let visible_bounds = self.host_bounds.borrow().as_ref().map(|bounds| {
            serde_json::json!({
                "x": bounds.origin.x.as_f32(),
                "y": bounds.origin.y.as_f32(),
                "width": bounds.size.width.as_f32(),
                "height": bounds.size.height.as_f32(),
            })
        });

        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let native_preview_mounted = self.native_preview.borrow().is_some();
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let native_preview_mounted = false;

        #[cfg(target_os = "windows")]
        let native_backend = "webview2-composition";
        #[cfg(target_os = "macos")]
        let native_backend = "wkwebview";
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let native_backend = "unavailable";

        serde_json::json!({
            "schema": "zed.web_preview.session.v1",
            "session_id": self.session_id.as_ref(),
            "title": self.current_tab_title().as_ref(),
            "url": self.active_url.as_ref(),
            "load_state": load_state,
            "load_error": load_error,
            "workspace": {
                "name": self.workspace_context.root_name.as_ref(),
                "key": self.workspace_context.preview_key.as_ref(),
                "database_id": self.workspace_context.workspace_id.as_ref().map(|id| format!("{id:?}")),
                "root_path": self.workspace_context.root_path.as_ref().map(|path| path.display().to_string()),
            },
            "profile_dir": self.workspace_context.profile_dir.display().to_string(),
            "zoom": self.zoom_factor,
            "viewport": self.viewport_mode.snapshot(),
            "active_item": self.is_active_item,
            "project_file_preview": self.project_item.is_some(),
            "agent_browser": self.agent_browser_policy_snapshot(),
            "bookmarks": {
                "count": self.bookmarks.len(),
                "active_url_bookmarked": self.is_active_url_bookmarked(),
            },
            "page_diagnostics": self.latest_page_diagnostics_summary(),
            "runtime_events": self.latest_runtime_events_summary(),
            "dom_snapshot": self.latest_dom_snapshot_summary(),
            "action_targets": self.latest_action_targets_summary(),
            "readiness_probe": self.latest_readiness_probe_summary(),
            "wait_contract": self.latest_wait_contract_summary(),
            "interaction_plan": self.latest_interaction_plan_summary(),
            "interaction_preflight": self.latest_interaction_preflight_summary(),
            "interaction_receipt_template": self.latest_interaction_receipt_template_summary(),
            "interaction_action_request": self.latest_interaction_action_request_summary(),
            "blocked_interaction_receipt": self.latest_blocked_interaction_receipt_summary(),
            "successful_interaction_receipt": self.latest_successful_interaction_receipt_summary(),
            "agent_browser_status_packet": self.latest_agent_browser_status_packet_summary(),
            "agent_browser_executor_readiness": self.latest_agent_browser_executor_readiness_summary(),
            "agent_browser_noop_executor_attempt": self.latest_agent_browser_noop_executor_attempt_summary(),
            "agent_browser_reload_executor_attempt": self.latest_agent_browser_reload_executor_attempt_summary(),
            "agent_browser_clear_data_executor_attempt": self.latest_agent_browser_clear_data_executor_attempt_summary(),
            "agent_browser_qa_runbook": self.latest_agent_browser_qa_runbook_summary(),
            "agent_plugin_catalog": self.latest_agent_plugin_catalog_summary(),
            "annotated_screenshot": self.latest_annotated_screenshot_summary(),
            "native_preview": {
                "backend": native_backend,
                "mounted": native_preview_mounted,
                "keyboard_focus": self.native_preview_has_keyboard_focus(window),
                "visible_bounds": visible_bounds,
            },
            "capabilities": {
                "copy_session_info": true,
                "copy_session_json": true,
                "send_session_to_agent": true,
                "copy_page_diagnostics": true,
                "send_page_diagnostics_to_agent": true,
                "copy_runtime_events": true,
                "send_runtime_events_to_agent": true,
                "copy_dom_snapshot": true,
                "send_dom_snapshot_to_agent": true,
                "copy_action_targets": true,
                "send_action_targets_to_agent": true,
                "copy_readiness_probe": true,
                "send_readiness_probe_to_agent": true,
                "copy_wait_contract": true,
                "send_wait_contract_to_agent": true,
                "copy_interaction_plan": true,
                "send_interaction_plan_to_agent": true,
                "copy_interaction_preflight": true,
                "send_interaction_preflight_to_agent": true,
                "copy_interaction_receipt_template": true,
                "send_interaction_receipt_template_to_agent": true,
                "copy_interaction_action_request": true,
                "send_interaction_action_request_to_agent": true,
                "copy_blocked_interaction_receipt": true,
                "send_blocked_interaction_receipt_to_agent": true,
                "copy_successful_interaction_receipt": true,
                "send_successful_interaction_receipt_to_agent": true,
                "copy_agent_browser_status_packet": true,
                "send_agent_browser_status_packet_to_agent": true,
                "copy_agent_browser_executor_readiness": true,
                "send_agent_browser_executor_readiness_to_agent": true,
                "copy_agent_browser_noop_executor_attempt": true,
                "send_agent_browser_noop_executor_attempt_to_agent": true,
                "run_permissioned_reload_executor": self.agent_action_permission.interactive_enabled(),
                "send_permissioned_reload_executor_to_agent": self.agent_action_permission.interactive_enabled(),
                "run_permissioned_clear_data_executor": self.agent_action_permission.interactive_enabled(),
                "send_permissioned_clear_data_executor_to_agent": self.agent_action_permission.interactive_enabled(),
                "copy_agent_browser_qa_runbook": true,
                "send_agent_browser_qa_runbook_to_agent": true,
                "copy_agent_plugin_catalog": true,
                "send_agent_plugin_catalog_to_agent": true,
                "copy_agent_browser_action_manifest": true,
                "send_agent_browser_action_manifest_to_agent": true,
                "interactive_browser_actions": self.agent_action_permission.interactive_enabled(),
                "screenshot": true,
                "capture_selected_area_screenshot": true,
                "annotate_screenshot": true,
                "inspect_element": true,
                "open_devtools": true,
                "responsive_viewport": true,
                "clear_data": true,
                "clear_cache": true,
            },
        })
    }

    fn agent_browser_policy_snapshot(&self) -> Value {
        serde_json::json!({
            "permission": self.agent_action_permission.snapshot(),
            "read_only_actions": READ_ONLY_AGENT_BROWSER_ACTIONS,
            "interactive_actions": INTERACTIVE_AGENT_BROWSER_ACTIONS,
            "interactive_actions_require_explicit_unlock": true,
        })
    }

    fn latest_page_diagnostics_summary(&self) -> Option<Value> {
        let diagnostics = self.latest_page_diagnostics.as_ref()?;
        Some(serde_json::json!({
            "captured_at": diagnostics.pointer("/page/timestamp").and_then(Value::as_str),
            "url": diagnostics.pointer("/page/url").and_then(Value::as_str),
            "title": diagnostics.pointer("/page/title").and_then(Value::as_str),
            "ready_state": diagnostics.pointer("/page/document/ready_state").and_then(Value::as_str),
            "counts": diagnostics.pointer("/page/counts").cloned(),
        }))
    }

    fn latest_runtime_events_summary(&self) -> Option<Value> {
        let events = self.latest_runtime_events.as_ref()?;
        Some(serde_json::json!({
            "captured_at": events.pointer("/events/timestamp").and_then(Value::as_str),
            "url": events.pointer("/events/url").and_then(Value::as_str),
            "title": events.pointer("/events/title").and_then(Value::as_str),
            "counts": events.pointer("/events/counts").cloned(),
        }))
    }

    fn latest_dom_snapshot_summary(&self) -> Option<Value> {
        let snapshot = self.latest_dom_snapshot.as_ref()?;
        Some(serde_json::json!({
            "captured_at": snapshot.pointer("/dom/timestamp").and_then(Value::as_str),
            "url": snapshot.pointer("/dom/url").and_then(Value::as_str),
            "title": snapshot.pointer("/dom/title").and_then(Value::as_str),
            "ready_state": snapshot.pointer("/dom/ready_state").and_then(Value::as_str),
            "counts": snapshot.pointer("/dom/counts").cloned(),
        }))
    }

    fn latest_action_targets_summary(&self) -> Option<Value> {
        let targets = self.latest_action_targets.as_ref()?;
        Some(serde_json::json!({
            "captured_at": targets.pointer("/targets/timestamp").and_then(Value::as_str),
            "url": targets.pointer("/targets/url").and_then(Value::as_str),
            "title": targets.pointer("/targets/title").and_then(Value::as_str),
            "ready_state": targets.pointer("/targets/ready_state").and_then(Value::as_str),
            "counts": targets.pointer("/targets/counts").cloned(),
        }))
    }

    fn latest_readiness_probe_summary(&self) -> Option<Value> {
        let probe = self.latest_readiness_probe.as_ref()?;
        Some(serde_json::json!({
            "captured_at": probe.pointer("/probe/timestamp").and_then(Value::as_str),
            "url": probe.pointer("/probe/url").and_then(Value::as_str),
            "title": probe.pointer("/probe/title").and_then(Value::as_str),
            "ready_state": probe.pointer("/probe/ready_state").and_then(Value::as_str),
            "readiness": probe.pointer("/probe/readiness").cloned(),
            "counts": probe.pointer("/probe/counts").cloned(),
        }))
    }

    fn latest_wait_contract_summary(&self) -> Option<Value> {
        let contract = self.latest_wait_contract.as_ref()?;
        Some(serde_json::json!({
            "captured_at": contract.pointer("/contract/timestamp").and_then(Value::as_str),
            "url": contract.pointer("/contract/url").and_then(Value::as_str),
            "title": contract.pointer("/contract/title").and_then(Value::as_str),
            "ready_state": contract.pointer("/contract/ready_state").and_then(Value::as_str),
            "recommended": contract.pointer("/contract/recommended").cloned(),
            "counts": contract.pointer("/contract/counts").cloned(),
        }))
    }

    fn latest_interaction_plan_summary(&self) -> Option<Value> {
        let plan = self.latest_interaction_plan.as_ref()?;
        Some(serde_json::json!({
            "captured_at": plan.pointer("/plan/timestamp").and_then(Value::as_str),
            "url": plan.pointer("/plan/url").and_then(Value::as_str),
            "title": plan.pointer("/plan/title").and_then(Value::as_str),
            "ready_state": plan.pointer("/plan/ready_state").and_then(Value::as_str),
            "dry_run_only": plan.pointer("/plan/dry_run_only").and_then(Value::as_bool),
            "counts": plan.pointer("/plan/counts").cloned(),
        }))
    }

    fn latest_interaction_preflight_summary(&self) -> Option<Value> {
        let preflight = self.latest_interaction_preflight.as_ref()?;
        Some(serde_json::json!({
            "captured_at": preflight.pointer("/preflight/timestamp").and_then(Value::as_str),
            "url": preflight.pointer("/preflight/url").and_then(Value::as_str),
            "title": preflight.pointer("/preflight/title").and_then(Value::as_str),
            "ready_state": preflight.pointer("/preflight/ready_state").and_then(Value::as_str),
            "permission": preflight.pointer("/preflight/permission").cloned(),
            "decision": preflight.pointer("/preflight/decision").cloned(),
            "counts": preflight.pointer("/preflight/counts").cloned(),
        }))
    }

    fn latest_interaction_receipt_template_summary(&self) -> Option<Value> {
        let template = self.latest_interaction_receipt_template.as_ref()?;
        Some(serde_json::json!({
            "captured_at": template.pointer("/template/timestamp").and_then(Value::as_str),
            "url": template.pointer("/template/url").and_then(Value::as_str),
            "title": template.pointer("/template/title").and_then(Value::as_str),
            "ready_state": template.pointer("/template/ready_state").and_then(Value::as_str),
            "permission": template.pointer("/template/permission").cloned(),
            "receipt_schema": template.pointer("/template/receipt/schema").and_then(Value::as_str),
            "counts": template.pointer("/template/counts").cloned(),
        }))
    }

    fn latest_interaction_action_request_summary(&self) -> Option<Value> {
        let request = self.latest_interaction_action_request.as_ref()?;
        Some(serde_json::json!({
            "captured_at": request.pointer("/request/timestamp").and_then(Value::as_str),
            "url": request.pointer("/request/url").and_then(Value::as_str),
            "title": request.pointer("/request/title").and_then(Value::as_str),
            "ready_state": request.pointer("/request/ready_state").and_then(Value::as_str),
            "permission": request.pointer("/request/permission").cloned(),
            "request_id": request.pointer("/request/envelope/request_id").and_then(Value::as_str),
            "status": request.pointer("/request/envelope/status").and_then(Value::as_str),
            "counts": request.pointer("/request/counts").cloned(),
        }))
    }

    fn latest_blocked_interaction_receipt_summary(&self) -> Option<Value> {
        let receipt = self.latest_blocked_interaction_receipt.as_ref()?;
        Some(serde_json::json!({
            "captured_at": receipt.pointer("/receipt/timestamp").and_then(Value::as_str),
            "url": receipt.pointer("/receipt/url").and_then(Value::as_str),
            "title": receipt.pointer("/receipt/title").and_then(Value::as_str),
            "ready_state": receipt.pointer("/receipt/ready_state").and_then(Value::as_str),
            "permission": receipt.pointer("/receipt/permission").cloned(),
            "outcome": receipt.pointer("/receipt/blocked_receipt/outcome").and_then(Value::as_str),
            "blockers": receipt.pointer("/receipt/blocked_receipt/error/blockers").cloned(),
            "counts": receipt.pointer("/receipt/counts").cloned(),
        }))
    }

    fn latest_successful_interaction_receipt_summary(&self) -> Option<Value> {
        let receipt = self.latest_successful_interaction_receipt.as_ref()?;
        Some(serde_json::json!({
            "captured_at": receipt.pointer("/receipt/timestamp").and_then(Value::as_str),
            "url": receipt.pointer("/receipt/url").and_then(Value::as_str),
            "title": receipt.pointer("/receipt/title").and_then(Value::as_str),
            "ready_state": receipt.pointer("/receipt/ready_state").and_then(Value::as_str),
            "permission": receipt.pointer("/receipt/permission").cloned(),
            "outcome": receipt.pointer("/receipt/success_receipt/outcome").and_then(Value::as_str),
            "sample_only": receipt.pointer("/receipt/success_receipt/sample_only").and_then(Value::as_bool),
            "counts": receipt.pointer("/receipt/counts").cloned(),
        }))
    }

    fn latest_agent_browser_status_packet_summary(&self) -> Option<Value> {
        let packet = self.latest_agent_browser_status_packet.as_ref()?;
        Some(serde_json::json!({
            "captured_at_ms": packet.pointer("/packet/captured_at_ms").and_then(Value::as_u64),
            "url": packet.pointer("/packet/url").and_then(Value::as_str),
            "title": packet.pointer("/packet/title").and_then(Value::as_str),
            "status": packet.pointer("/packet/status").and_then(Value::as_str),
            "context_ready": packet.pointer("/packet/readiness/context_ready").and_then(Value::as_bool),
            "audit_ready": packet.pointer("/packet/readiness/audit_ready").and_then(Value::as_bool),
            "interactive_unlocked": packet.pointer("/packet/readiness/interactive_unlocked").and_then(Value::as_bool),
            "next_step": packet.pointer("/packet/next_step").and_then(Value::as_str),
        }))
    }

    fn latest_agent_browser_executor_readiness_summary(&self) -> Option<Value> {
        let readiness = self.latest_agent_browser_executor_readiness.as_ref()?;
        Some(serde_json::json!({
            "captured_at_ms": readiness.pointer("/readiness/captured_at_ms").and_then(Value::as_u64),
            "url": readiness.pointer("/readiness/url").and_then(Value::as_str),
            "title": readiness.pointer("/readiness/title").and_then(Value::as_str),
            "status": readiness.pointer("/readiness/status").and_then(Value::as_str),
            "gate_ready_for_executor": readiness.pointer("/readiness/gate_ready_for_executor").and_then(Value::as_bool),
            "can_dispatch_now": readiness.pointer("/readiness/can_dispatch_now").and_then(Value::as_bool),
            "blocker_count": readiness.pointer("/readiness/blockers").and_then(Value::as_array).map(Vec::len),
            "next_step": readiness.pointer("/readiness/next_step").and_then(Value::as_str),
        }))
    }

    fn latest_agent_browser_noop_executor_attempt_summary(&self) -> Option<Value> {
        let attempt = self.latest_agent_browser_noop_executor_attempt.as_ref()?;
        Some(serde_json::json!({
            "captured_at_ms": attempt.pointer("/attempt/captured_at_ms").and_then(Value::as_u64),
            "url": attempt.pointer("/attempt/url").and_then(Value::as_str),
            "title": attempt.pointer("/attempt/title").and_then(Value::as_str),
            "mode": attempt.pointer("/attempt/mode").and_then(Value::as_str),
            "outcome": attempt.pointer("/attempt/outcome").and_then(Value::as_str),
            "attempted_action_count": attempt.pointer("/attempt/attempted_actions").and_then(Value::as_array).map(Vec::len),
            "blocked_receipt_outcome": attempt.pointer("/attempt/blocked_receipt/outcome").and_then(Value::as_str),
            "blocker_count": attempt.pointer("/attempt/blockers").and_then(Value::as_array).map(Vec::len),
        }))
    }

    fn latest_agent_browser_reload_executor_attempt_summary(&self) -> Option<Value> {
        let attempt = self.latest_agent_browser_reload_executor_attempt.as_ref()?;
        Some(serde_json::json!({
            "captured_at_ms": attempt.pointer("/attempt/captured_at_ms").and_then(Value::as_u64),
            "url": attempt.pointer("/attempt/url").and_then(Value::as_str),
            "title": attempt.pointer("/attempt/title").and_then(Value::as_str),
            "action": attempt.pointer("/attempt/action").and_then(Value::as_str),
            "outcome": attempt.pointer("/attempt/outcome").and_then(Value::as_str),
            "browser_command_dispatched": attempt.pointer("/attempt/browser_command_dispatched").and_then(Value::as_bool),
            "receipt_outcome": attempt.pointer("/attempt/receipt/outcome").and_then(Value::as_str),
            "blocker_count": attempt.pointer("/attempt/blockers").and_then(Value::as_array).map(Vec::len),
        }))
    }

    fn latest_agent_browser_clear_data_executor_attempt_summary(&self) -> Option<Value> {
        let attempt = self
            .latest_agent_browser_clear_data_executor_attempt
            .as_ref()?;
        Some(serde_json::json!({
            "captured_at_ms": attempt.pointer("/attempt/captured_at_ms").and_then(Value::as_u64),
            "url": attempt.pointer("/attempt/url").and_then(Value::as_str),
            "title": attempt.pointer("/attempt/title").and_then(Value::as_str),
            "action": attempt.pointer("/attempt/action").and_then(Value::as_str),
            "outcome": attempt.pointer("/attempt/outcome").and_then(Value::as_str),
            "browser_command_dispatched": attempt.pointer("/attempt/browser_command_dispatched").and_then(Value::as_bool),
            "receipt_outcome": attempt.pointer("/attempt/receipt/outcome").and_then(Value::as_str),
            "blocker_count": attempt.pointer("/attempt/blockers").and_then(Value::as_array).map(Vec::len),
        }))
    }

    fn latest_agent_browser_qa_runbook_summary(&self) -> Option<Value> {
        let runbook = self.latest_agent_browser_qa_runbook.as_ref()?;
        Some(serde_json::json!({
            "captured_at_ms": runbook.pointer("/runbook/captured_at_ms").and_then(Value::as_u64),
            "url": runbook.pointer("/runbook/url").and_then(Value::as_str),
            "title": runbook.pointer("/runbook/title").and_then(Value::as_str),
            "status": runbook.pointer("/runbook/status").and_then(Value::as_str),
            "manual_gate_count": runbook.pointer("/runbook/manual_gates").and_then(Value::as_array).map(Vec::len),
            "known_limit_count": runbook.pointer("/runbook/known_limits").and_then(Value::as_array).map(Vec::len),
            "next_feature_set": runbook.pointer("/runbook/next_feature_set/name").and_then(Value::as_str),
        }))
    }

    fn latest_agent_plugin_catalog_summary(&self) -> Option<Value> {
        let catalog = self.latest_agent_plugin_catalog.as_ref()?;
        Some(serde_json::json!({
            "generated_at_ms": catalog.pointer("/catalog/generated_at_ms").and_then(Value::as_u64),
            "status": catalog.pointer("/catalog/status").and_then(Value::as_str),
            "plugin_count": catalog.pointer("/catalog/plugins").and_then(Value::as_array).map(Vec::len),
            "default_enabled_plugins": catalog.pointer("/catalog/default_enabled_plugins").cloned(),
            "available_to": catalog.pointer("/catalog/available_to").cloned(),
        }))
    }

    fn latest_annotated_screenshot_summary(&self) -> Option<Value> {
        let screenshot = self.latest_annotated_screenshot.as_ref()?;
        Some(serde_json::json!({
            "captured_at_ms": screenshot.pointer("/capture/captured_at_ms").and_then(Value::as_u64),
            "url": screenshot.pointer("/annotation/url").and_then(Value::as_str),
            "title": screenshot.pointer("/annotation/title").and_then(Value::as_str),
            "annotation_count": screenshot.pointer("/annotation/counts/annotations").and_then(Value::as_u64),
            "viewport": screenshot.pointer("/annotation/viewport").cloned(),
        }))
    }

    fn current_epoch_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
            .unwrap_or_default()
    }

    fn load_state_name(&self) -> &'static str {
        match &self.load_state {
            PreviewLoadState::Loading => "loading",
            PreviewLoadState::Ready => "ready",
            PreviewLoadState::Error(_) => "error",
        }
    }

    fn viewport_label(&self) -> String {
        match self.viewport_mode.dimensions() {
            Some((width, height)) => {
                format!("{} ({}x{})", self.viewport_mode.label(), width, height)
            }
            None => self.viewport_mode.label().to_string(),
        }
    }

    fn browser_session_json(&self, window: &Window) -> String {
        serde_json::to_string_pretty(&self.browser_session_snapshot(window))
            .unwrap_or_else(|_| "{}".to_string())
    }

    fn workspace_session_snapshots(&self, window: &Window, cx: &App) -> Vec<Value> {
        let Some(workspace) = self.workspace.upgrade() else {
            return vec![self.browser_session_snapshot(window)];
        };

        let mut snapshots = workspace.read_with(cx, |workspace, cx| {
            workspace
                .items(cx)
                .filter_map(|item| {
                    let preview = item.downcast::<WebPreviewView>()?;
                    Some(preview.read(cx).browser_session_snapshot(window))
                })
                .collect::<Vec<_>>()
        });

        if snapshots.is_empty() {
            snapshots.push(self.browser_session_snapshot(window));
        }

        snapshots
    }

    fn workspace_session_inventory_json(&self, window: &Window, cx: &App) -> String {
        let sessions = self.workspace_session_snapshots(window, cx);
        let inventory = serde_json::json!({
            "schema": "zed.web_preview.session_inventory.v1",
            "active_session_id": self.session_id.as_ref(),
            "count": sessions.len(),
            "sessions": sessions,
        });

        serde_json::to_string_pretty(&inventory).unwrap_or_else(|_| "{}".to_string())
    }

    fn copy_browser_session_info(&mut self, window: &Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(self.browser_session_info(window)));
        self.show_toast("Copied web preview session info", cx);
    }

    fn copy_browser_session_json(&mut self, window: &Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(self.browser_session_json(window)));
        self.show_toast("Copied web preview session JSON", cx);
    }

    fn copy_workspace_session_inventory_json(&mut self, window: &Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(
            self.workspace_session_inventory_json(window, cx),
        ));
        self.show_toast("Copied web preview session inventory JSON", cx);
    }

    fn send_workspace_session_inventory_to_agent(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let blocks = vec![acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview session inventory:\n\n```json\n{}\n```",
            self.workspace_session_inventory_json(window, cx)
        )))];
        self.append_content_blocks_to_agent_panel(blocks, window, cx);
        self.show_toast("Sent web preview session inventory to the agent panel", cx);
    }

    fn send_browser_session_info_to_agent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut blocks = Vec::new();
        if let Some(url_block) = self.current_url_attachment_block() {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview session context:\n\n```json\n{}\n```",
            self.browser_session_json(window)
        ))));
        self.append_content_blocks_to_agent_panel(blocks, window, cx);
        self.show_toast("Sent web preview session info to the agent panel", cx);
    }

    fn page_diagnostics_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut page = payload.clone();
        if let Some(page) = page.as_object_mut() {
            page.remove("kind");
            page.remove("action");
        }

        serde_json::json!({
            "schema": "zed.web_preview.page_diagnostics.v1",
            "session": self.browser_session_snapshot(window),
            "page": page,
        })
    }

    fn page_diagnostics_json(diagnostics: &Value) -> String {
        serde_json::to_string_pretty(diagnostics).unwrap_or_else(|_| "{}".to_string())
    }

    fn page_diagnostics_agent_blocks(&self, diagnostics: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = diagnostics.pointer("/page/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview page diagnostics:\n\n```json\n{}\n```",
            Self::page_diagnostics_json(diagnostics)
        ))));
        blocks
    }

    fn request_page_diagnostics(&mut self, action: &'static str, cx: &mut Context<Self>) {
        let script = format!(
            "window.__zedWebPreview && window.__zedWebPreview.collectPageDiagnostics('{action}');"
        );
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(&script))) {
            Ok(Ok(())) => {
                self.show_toast("Collecting web preview page diagnostics", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Page diagnostics are unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic("Page diagnostics crashed before collection", cx);
            }
        }
    }

    fn copy_page_diagnostics(&mut self, cx: &mut Context<Self>) {
        self.request_page_diagnostics("copy", cx);
    }

    fn send_page_diagnostics_to_agent(&mut self, cx: &mut Context<Self>) {
        self.request_page_diagnostics("agent", cx);
    }

    fn runtime_events_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut events = payload.clone();
        if let Some(events) = events.as_object_mut() {
            events.remove("kind");
            events.remove("action");
        }

        serde_json::json!({
            "schema": "zed.web_preview.runtime_events.v1",
            "session": self.browser_session_snapshot(window),
            "events": events,
        })
    }

    fn runtime_events_json(events: &Value) -> String {
        serde_json::to_string_pretty(events).unwrap_or_else(|_| "{}".to_string())
    }

    fn runtime_events_agent_blocks(&self, events: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = events.pointer("/events/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview runtime events:\n\n```json\n{}\n```",
            Self::runtime_events_json(events)
        ))));
        blocks
    }

    fn request_runtime_events(&mut self, action: &'static str, cx: &mut Context<Self>) {
        let script = format!(
            "window.__zedWebPreview && window.__zedWebPreview.collectRuntimeEvents('{action}');"
        );
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(&script))) {
            Ok(Ok(())) => {
                self.show_toast("Collecting web preview runtime events", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Runtime events are unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic("Runtime events crashed before collection", cx);
            }
        }
    }

    fn copy_runtime_events(&mut self, cx: &mut Context<Self>) {
        self.request_runtime_events("copy", cx);
    }

    fn send_runtime_events_to_agent(&mut self, cx: &mut Context<Self>) {
        self.request_runtime_events("agent", cx);
    }

    fn dom_snapshot_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut dom = payload.clone();
        if let Some(dom) = dom.as_object_mut() {
            dom.remove("kind");
            dom.remove("action");
        }

        serde_json::json!({
            "schema": "zed.web_preview.dom_snapshot.v1",
            "session": self.browser_session_snapshot(window),
            "dom": dom,
        })
    }

    fn dom_snapshot_json(snapshot: &Value) -> String {
        serde_json::to_string_pretty(snapshot).unwrap_or_else(|_| "{}".to_string())
    }

    fn dom_snapshot_agent_blocks(&self, snapshot: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = snapshot.pointer("/dom/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview DOM snapshot:\n\n```json\n{}\n```",
            Self::dom_snapshot_json(snapshot)
        ))));
        blocks
    }

    fn request_dom_snapshot(&mut self, action: &'static str, cx: &mut Context<Self>) {
        let script = format!(
            "window.__zedWebPreview && window.__zedWebPreview.collectDomSnapshot('{action}');"
        );
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(&script))) {
            Ok(Ok(())) => {
                self.show_toast("Collecting web preview DOM snapshot", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("DOM snapshot is unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic("DOM snapshot crashed before collection", cx);
            }
        }
    }

    fn copy_dom_snapshot(&mut self, cx: &mut Context<Self>) {
        self.request_dom_snapshot("copy", cx);
    }

    fn send_dom_snapshot_to_agent(&mut self, cx: &mut Context<Self>) {
        self.request_dom_snapshot("agent", cx);
    }

    fn action_targets_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut targets = payload.clone();
        if let Some(targets) = targets.as_object_mut() {
            targets.remove("kind");
            targets.remove("action");
        }

        serde_json::json!({
            "schema": "zed.web_preview.action_targets.v1",
            "session": self.browser_session_snapshot(window),
            "targets": targets,
        })
    }

    fn action_targets_json(targets: &Value) -> String {
        serde_json::to_string_pretty(targets).unwrap_or_else(|_| "{}".to_string())
    }

    fn action_targets_agent_blocks(&self, targets: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = targets.pointer("/targets/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview action targets:\n\n```json\n{}\n```",
            Self::action_targets_json(targets)
        ))));
        blocks
    }

    fn request_action_targets(&mut self, action: &'static str, cx: &mut Context<Self>) {
        let script = format!(
            "window.__zedWebPreview && window.__zedWebPreview.collectActionTargets('{action}');"
        );
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(&script))) {
            Ok(Ok(())) => {
                self.show_toast("Collecting web preview action targets", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Action targets are unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic("Action targets crashed before collection", cx);
            }
        }
    }

    fn copy_action_targets(&mut self, cx: &mut Context<Self>) {
        self.request_action_targets("copy", cx);
    }

    fn send_action_targets_to_agent(&mut self, cx: &mut Context<Self>) {
        self.request_action_targets("agent", cx);
    }

    fn readiness_probe_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut probe = payload.clone();
        if let Some(probe) = probe.as_object_mut() {
            probe.remove("kind");
            probe.remove("action");
        }

        serde_json::json!({
            "schema": "zed.web_preview.readiness_probe.v1",
            "session": self.browser_session_snapshot(window),
            "probe": probe,
        })
    }

    fn readiness_probe_json(probe: &Value) -> String {
        serde_json::to_string_pretty(probe).unwrap_or_else(|_| "{}".to_string())
    }

    fn readiness_probe_agent_blocks(&self, probe: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = probe.pointer("/probe/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview readiness probe:\n\n```json\n{}\n```",
            Self::readiness_probe_json(probe)
        ))));
        blocks
    }

    fn request_readiness_probe(&mut self, action: &'static str, cx: &mut Context<Self>) {
        let script = format!(
            "window.__zedWebPreview && window.__zedWebPreview.collectReadinessProbe('{action}');"
        );
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(&script))) {
            Ok(Ok(())) => {
                self.show_toast("Collecting web preview readiness probe", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Readiness probe is unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic("Readiness probe crashed before collection", cx);
            }
        }
    }

    fn copy_readiness_probe(&mut self, cx: &mut Context<Self>) {
        self.request_readiness_probe("copy", cx);
    }

    fn send_readiness_probe_to_agent(&mut self, cx: &mut Context<Self>) {
        self.request_readiness_probe("agent", cx);
    }

    fn wait_contract_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut contract = payload.clone();
        if let Some(contract) = contract.as_object_mut() {
            contract.remove("kind");
            contract.remove("action");
        }

        serde_json::json!({
            "schema": "zed.web_preview.wait_contract.v1",
            "session": self.browser_session_snapshot(window),
            "contract": contract,
        })
    }

    fn wait_contract_json(contract: &Value) -> String {
        serde_json::to_string_pretty(contract).unwrap_or_else(|_| "{}".to_string())
    }

    fn wait_contract_agent_blocks(&self, contract: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = contract.pointer("/contract/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview wait contract:\n\n```json\n{}\n```",
            Self::wait_contract_json(contract)
        ))));
        blocks
    }

    fn request_wait_contract(&mut self, action: &'static str, cx: &mut Context<Self>) {
        let script = format!(
            "window.__zedWebPreview && window.__zedWebPreview.collectWaitContract('{action}');"
        );
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(&script))) {
            Ok(Ok(())) => {
                self.show_toast("Collecting web preview wait contract", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Wait contract is unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic("Wait contract crashed before collection", cx);
            }
        }
    }

    fn copy_wait_contract(&mut self, cx: &mut Context<Self>) {
        self.request_wait_contract("copy", cx);
    }

    fn send_wait_contract_to_agent(&mut self, cx: &mut Context<Self>) {
        self.request_wait_contract("agent", cx);
    }

    fn interaction_plan_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut plan = payload.clone();
        if let Some(plan) = plan.as_object_mut() {
            plan.remove("kind");
            plan.remove("action");
        }

        serde_json::json!({
            "schema": "zed.web_preview.interaction_plan.v1",
            "session": self.browser_session_snapshot(window),
            "plan": plan,
        })
    }

    fn interaction_plan_json(plan: &Value) -> String {
        serde_json::to_string_pretty(plan).unwrap_or_else(|_| "{}".to_string())
    }

    fn interaction_plan_agent_blocks(&self, plan: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = plan.pointer("/plan/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview interaction plan:\n\n```json\n{}\n```",
            Self::interaction_plan_json(plan)
        ))));
        blocks
    }

    fn request_interaction_plan(&mut self, action: &'static str, cx: &mut Context<Self>) {
        let script = format!(
            "window.__zedWebPreview && window.__zedWebPreview.collectInteractionPlan('{action}');"
        );
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(&script))) {
            Ok(Ok(())) => {
                self.show_toast("Collecting web preview interaction plan", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Interaction plan is unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic("Interaction plan crashed before collection", cx);
            }
        }
    }

    fn copy_interaction_plan(&mut self, cx: &mut Context<Self>) {
        self.request_interaction_plan("copy", cx);
    }

    fn send_interaction_plan_to_agent(&mut self, cx: &mut Context<Self>) {
        self.request_interaction_plan("agent", cx);
    }

    fn interaction_preflight_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut preflight = payload.clone();
        if let Some(preflight) = preflight.as_object_mut() {
            preflight.remove("kind");
            preflight.remove("action");
            preflight.insert(
                "permission".to_string(),
                self.agent_action_permission.snapshot(),
            );
            preflight.insert(
                "permission_gate".to_string(),
                serde_json::json!({
                    "interactive_unlocked": self.agent_action_permission.interactive_enabled(),
                    "status": if self.agent_action_permission.interactive_enabled() {
                        "unlocked"
                    } else {
                        "locked"
                    },
                    "message": if self.agent_action_permission.interactive_enabled() {
                        "Interactive actions are currently unlocked for this WebPreview session."
                    } else {
                        "Interactive actions are locked; this preflight can only be used for planning."
                    },
                }),
            );
        }

        serde_json::json!({
            "schema": "zed.web_preview.interaction_preflight.v1",
            "session": self.browser_session_snapshot(window),
            "preflight": preflight,
        })
    }

    fn interaction_preflight_json(preflight: &Value) -> String {
        serde_json::to_string_pretty(preflight).unwrap_or_else(|_| "{}".to_string())
    }

    fn interaction_preflight_agent_blocks(&self, preflight: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = preflight.pointer("/preflight/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview interaction preflight:\n\n```json\n{}\n```",
            Self::interaction_preflight_json(preflight)
        ))));
        blocks
    }

    fn request_interaction_preflight(&mut self, action: &'static str, cx: &mut Context<Self>) {
        let script = format!(
            "window.__zedWebPreview && window.__zedWebPreview.collectInteractionPreflight('{action}');"
        );
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(&script))) {
            Ok(Ok(())) => {
                self.show_toast("Collecting web preview interaction preflight", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Interaction preflight is unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic("Interaction preflight crashed before collection", cx);
            }
        }
    }

    fn copy_interaction_preflight(&mut self, cx: &mut Context<Self>) {
        self.request_interaction_preflight("copy", cx);
    }

    fn send_interaction_preflight_to_agent(&mut self, cx: &mut Context<Self>) {
        self.request_interaction_preflight("agent", cx);
    }

    fn interaction_receipt_template_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut template = payload.clone();
        if let Some(template) = template.as_object_mut() {
            template.remove("kind");
            template.remove("action");
            template.insert(
                "permission".to_string(),
                self.agent_action_permission.snapshot(),
            );
            template.insert(
                "permission_gate".to_string(),
                serde_json::json!({
                    "interactive_unlocked": self.agent_action_permission.interactive_enabled(),
                    "status": if self.agent_action_permission.interactive_enabled() {
                        "unlocked"
                    } else {
                        "locked"
                    },
                    "message": if self.agent_action_permission.interactive_enabled() {
                        "Future interactive actions may proceed only after a fresh preflight and user-approved target."
                    } else {
                        "Interactive actions are locked; use this receipt template for planning only."
                    },
                }),
            );
        }

        serde_json::json!({
            "schema": "zed.web_preview.interaction_receipt_template.v1",
            "session": self.browser_session_snapshot(window),
            "template": template,
        })
    }

    fn interaction_receipt_template_json(template: &Value) -> String {
        serde_json::to_string_pretty(template).unwrap_or_else(|_| "{}".to_string())
    }

    fn interaction_receipt_template_agent_blocks(
        &self,
        template: &Value,
    ) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = template.pointer("/template/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview interaction receipt template:\n\n```json\n{}\n```",
            Self::interaction_receipt_template_json(template)
        ))));
        blocks
    }

    fn request_interaction_receipt_template(
        &mut self,
        action: &'static str,
        cx: &mut Context<Self>,
    ) {
        let script = format!(
            "window.__zedWebPreview && window.__zedWebPreview.collectInteractionReceiptTemplate('{action}');"
        );
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(&script))) {
            Ok(Ok(())) => {
                self.show_toast("Collecting web preview interaction receipt template", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Interaction receipt template is unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic(
                    "Interaction receipt template crashed before collection",
                    cx,
                );
            }
        }
    }

    fn copy_interaction_receipt_template(&mut self, cx: &mut Context<Self>) {
        self.request_interaction_receipt_template("copy", cx);
    }

    fn send_interaction_receipt_template_to_agent(&mut self, cx: &mut Context<Self>) {
        self.request_interaction_receipt_template("agent", cx);
    }

    fn interaction_action_request_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut request = payload.clone();
        if let Some(request) = request.as_object_mut() {
            request.remove("kind");
            request.remove("action");
            request.insert(
                "permission".to_string(),
                self.agent_action_permission.snapshot(),
            );
            request.insert(
                "permission_gate".to_string(),
                serde_json::json!({
                    "interactive_unlocked": self.agent_action_permission.interactive_enabled(),
                    "status": if self.agent_action_permission.interactive_enabled() {
                        "unlocked"
                    } else {
                        "locked"
                    },
                    "message": if self.agent_action_permission.interactive_enabled() {
                        "This request envelope can be used only after a fresh preflight confirms the selected target."
                    } else {
                        "Interactive actions are locked; this request envelope is planning-only."
                    },
                }),
            );
        }

        serde_json::json!({
            "schema": "zed.web_preview.interaction_action_request.v1",
            "session": self.browser_session_snapshot(window),
            "request": request,
        })
    }

    fn interaction_action_request_json(request: &Value) -> String {
        serde_json::to_string_pretty(request).unwrap_or_else(|_| "{}".to_string())
    }

    fn interaction_action_request_agent_blocks(&self, request: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = request.pointer("/request/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview interaction action request:\n\n```json\n{}\n```",
            Self::interaction_action_request_json(request)
        ))));
        blocks
    }

    fn request_interaction_action_request(&mut self, action: &'static str, cx: &mut Context<Self>) {
        let script = format!(
            "window.__zedWebPreview && window.__zedWebPreview.collectInteractionActionRequest('{action}');"
        );
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(&script))) {
            Ok(Ok(())) => {
                self.show_toast("Collecting web preview action request", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Interaction action request is unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic(
                    "Interaction action request crashed before collection",
                    cx,
                );
            }
        }
    }

    fn copy_interaction_action_request(&mut self, cx: &mut Context<Self>) {
        self.request_interaction_action_request("copy", cx);
    }

    fn send_interaction_action_request_to_agent(&mut self, cx: &mut Context<Self>) {
        self.request_interaction_action_request("agent", cx);
    }

    fn blocked_interaction_receipt_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut receipt = payload.clone();
        if let Some(receipt) = receipt.as_object_mut() {
            receipt.remove("kind");
            receipt.remove("action");
            let interactive_enabled = self.agent_action_permission.interactive_enabled();
            receipt.insert(
                "permission".to_string(),
                self.agent_action_permission.snapshot(),
            );
            receipt.insert(
                "permission_gate".to_string(),
                serde_json::json!({
                    "interactive_unlocked": interactive_enabled,
                    "status": if interactive_enabled {
                        "unlocked"
                    } else {
                        "locked"
                    },
                    "message": if interactive_enabled {
                        "The Zed permission gate is unlocked; page preflight blockers may still block execution."
                    } else {
                        "Interactive actions are locked, so the attempted action must be recorded as blocked."
                    },
                }),
            );
            if let Some(blocked_receipt) = receipt
                .get_mut("blocked_receipt")
                .and_then(Value::as_object_mut)
            {
                blocked_receipt.insert(
                    "permission_mode".to_string(),
                    serde_json::json!(if interactive_enabled {
                        "interactive"
                    } else {
                        "read_only"
                    }),
                );
                if !interactive_enabled {
                    blocked_receipt.insert("outcome".to_string(), serde_json::json!("blocked"));
                    let permission_blocker = serde_json::json!({
                        "code": "interactive_actions_locked",
                        "message": "Zed interactive browser actions are locked for this WebPreview session.",
                    });
                    if let Some(error) = blocked_receipt
                        .get_mut("error")
                        .and_then(Value::as_object_mut)
                    {
                        error.insert(
                            "code".to_string(),
                            serde_json::json!("interactive_actions_locked"),
                        );
                        error.insert(
                            "message".to_string(),
                            serde_json::json!(
                                "The browser action was blocked by Zed's interactive permission gate."
                            ),
                        );
                        match error.get_mut("blockers").and_then(Value::as_array_mut) {
                            Some(blockers) => blockers.push(permission_blocker),
                            None => {
                                error.insert(
                                    "blockers".to_string(),
                                    serde_json::json!([permission_blocker]),
                                );
                            }
                        }
                    }
                }
            }
            if !interactive_enabled {
                receipt.insert(
                    "zed_blocker".to_string(),
                    serde_json::json!({
                        "code": "interactive_actions_locked",
                        "message": "Zed interactive browser actions are locked for this WebPreview session.",
                    }),
                );
            }
        }

        serde_json::json!({
            "schema": "zed.web_preview.blocked_interaction_receipt.v1",
            "session": self.browser_session_snapshot(window),
            "receipt": receipt,
        })
    }

    fn blocked_interaction_receipt_json(receipt: &Value) -> String {
        serde_json::to_string_pretty(receipt).unwrap_or_else(|_| "{}".to_string())
    }

    fn blocked_interaction_receipt_agent_blocks(&self, receipt: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = receipt.pointer("/receipt/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview blocked interaction receipt:\n\n```json\n{}\n```",
            Self::blocked_interaction_receipt_json(receipt)
        ))));
        blocks
    }

    fn request_blocked_interaction_receipt(
        &mut self,
        action: &'static str,
        cx: &mut Context<Self>,
    ) {
        let script = format!(
            "window.__zedWebPreview && window.__zedWebPreview.collectBlockedInteractionReceipt('{action}');"
        );
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(&script))) {
            Ok(Ok(())) => {
                self.show_toast("Collecting blocked interaction receipt", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Blocked interaction receipt is unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic(
                    "Blocked interaction receipt crashed before collection",
                    cx,
                );
            }
        }
    }

    fn copy_blocked_interaction_receipt(&mut self, cx: &mut Context<Self>) {
        self.request_blocked_interaction_receipt("copy", cx);
    }

    fn send_blocked_interaction_receipt_to_agent(&mut self, cx: &mut Context<Self>) {
        self.request_blocked_interaction_receipt("agent", cx);
    }

    fn successful_interaction_receipt_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut receipt = payload.clone();
        if let Some(receipt) = receipt.as_object_mut() {
            receipt.remove("kind");
            receipt.remove("action");
            let interactive_enabled = self.agent_action_permission.interactive_enabled();
            receipt.insert(
                "permission".to_string(),
                self.agent_action_permission.snapshot(),
            );
            receipt.insert(
                "permission_gate".to_string(),
                serde_json::json!({
                    "interactive_unlocked": interactive_enabled,
                    "status": if interactive_enabled {
                        "unlocked"
                    } else {
                        "locked"
                    },
                    "message": if interactive_enabled {
                        "A real successful receipt still requires a fresh post-action snapshot from the executor."
                    } else {
                        "Interactive actions are locked; this success receipt remains a sample-only audit template."
                    },
                }),
            );
            if let Some(success_receipt) = receipt
                .get_mut("success_receipt")
                .and_then(Value::as_object_mut)
            {
                success_receipt.insert(
                    "permission_mode".to_string(),
                    serde_json::json!(if interactive_enabled {
                        "interactive"
                    } else {
                        "read_only"
                    }),
                );
                success_receipt.insert("sample_only".to_string(), serde_json::json!(true));
            }
        }

        serde_json::json!({
            "schema": "zed.web_preview.successful_interaction_receipt.v1",
            "session": self.browser_session_snapshot(window),
            "receipt": receipt,
        })
    }

    fn successful_interaction_receipt_json(receipt: &Value) -> String {
        serde_json::to_string_pretty(receipt).unwrap_or_else(|_| "{}".to_string())
    }

    fn successful_interaction_receipt_agent_blocks(
        &self,
        receipt: &Value,
    ) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = receipt.pointer("/receipt/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview successful interaction receipt template:\n\n```json\n{}\n```",
            Self::successful_interaction_receipt_json(receipt)
        ))));
        blocks
    }

    fn request_successful_interaction_receipt(
        &mut self,
        action: &'static str,
        cx: &mut Context<Self>,
    ) {
        let script = format!(
            "window.__zedWebPreview && window.__zedWebPreview.collectSuccessfulInteractionReceipt('{action}');"
        );
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(&script))) {
            Ok(Ok(())) => {
                self.show_toast("Collecting successful interaction receipt template", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error(
                    "Successful interaction receipt is unavailable",
                    error,
                    cx,
                );
            }
            Err(_) => {
                self.report_action_panic(
                    "Successful interaction receipt crashed before collection",
                    cx,
                );
            }
        }
    }

    fn copy_successful_interaction_receipt(&mut self, cx: &mut Context<Self>) {
        self.request_successful_interaction_receipt("copy", cx);
    }

    fn send_successful_interaction_receipt_to_agent(&mut self, cx: &mut Context<Self>) {
        self.request_successful_interaction_receipt("agent", cx);
    }

    fn agent_browser_status_packet(&self, window: &Window) -> Value {
        let diagnostics_ready = self.latest_page_diagnostics.is_some();
        let runtime_ready = self.latest_runtime_events.is_some();
        let dom_ready = self.latest_dom_snapshot.is_some();
        let targets_ready = self.latest_action_targets.is_some();
        let readiness_ready = self.latest_readiness_probe.is_some();
        let wait_ready = self.latest_wait_contract.is_some();
        let plan_ready = self.latest_interaction_plan.is_some();
        let preflight_ready = self.latest_interaction_preflight.is_some();
        let receipt_template_ready = self.latest_interaction_receipt_template.is_some();
        let action_request_ready = self.latest_interaction_action_request.is_some();
        let blocked_receipt_ready = self.latest_blocked_interaction_receipt.is_some();
        let success_receipt_ready = self.latest_successful_interaction_receipt.is_some();
        let context_ready =
            diagnostics_ready || runtime_ready || dom_ready || targets_ready || readiness_ready;
        let audit_ready = plan_ready
            && preflight_ready
            && receipt_template_ready
            && action_request_ready
            && blocked_receipt_ready
            && success_receipt_ready;
        let interactive_unlocked = self.agent_action_permission.interactive_enabled();
        let status = if context_ready && audit_ready {
            "ready_for_permissioned_executor"
        } else if context_ready {
            "context_ready_audit_incomplete"
        } else {
            "needs_read_only_context_collection"
        };
        let next_step = if !context_ready {
            "Collect page diagnostics, DOM, action targets, readiness, and runtime events before attempting browser actions."
        } else if !audit_ready {
            "Collect the interaction plan, preflight, action request, blocked receipt, and success receipt template to complete the audit packet."
        } else if !interactive_unlocked {
            "Keep actions read-only until the user explicitly unlocks interactive browser actions for this WebPreview session."
        } else {
            "Interactive actions are unlocked; the executor must still perform a fresh preflight and emit a receipt for every action."
        };

        serde_json::json!({
            "schema": "zed.web_preview.agent_browser_status_packet.v1",
            "session": self.browser_session_snapshot(window),
            "policy": self.agent_browser_policy_snapshot(),
            "packet": {
                "captured_at_ms": Self::current_epoch_millis(),
                "session_id": self.session_id.as_ref(),
                "title": self.current_tab_title().as_ref(),
                "url": self.active_url.as_ref(),
                "status": status,
                "next_step": next_step,
                "readiness": {
                    "context_ready": context_ready,
                    "audit_ready": audit_ready,
                    "interactive_unlocked": interactive_unlocked,
                    "page_diagnostics_ready": diagnostics_ready,
                    "runtime_events_ready": runtime_ready,
                    "dom_snapshot_ready": dom_ready,
                    "action_targets_ready": targets_ready,
                    "readiness_probe_ready": readiness_ready,
                    "wait_contract_ready": wait_ready,
                    "interaction_plan_ready": plan_ready,
                    "interaction_preflight_ready": preflight_ready,
                    "receipt_template_ready": receipt_template_ready,
                    "action_request_ready": action_request_ready,
                    "blocked_receipt_ready": blocked_receipt_ready,
                    "successful_receipt_template_ready": success_receipt_ready,
                },
                "latest": {
                    "page_diagnostics": self.latest_page_diagnostics_summary(),
                    "runtime_events": self.latest_runtime_events_summary(),
                    "dom_snapshot": self.latest_dom_snapshot_summary(),
                    "action_targets": self.latest_action_targets_summary(),
                    "readiness_probe": self.latest_readiness_probe_summary(),
                    "wait_contract": self.latest_wait_contract_summary(),
                    "interaction_plan": self.latest_interaction_plan_summary(),
                    "interaction_preflight": self.latest_interaction_preflight_summary(),
                    "interaction_receipt_template": self.latest_interaction_receipt_template_summary(),
                    "interaction_action_request": self.latest_interaction_action_request_summary(),
                    "blocked_interaction_receipt": self.latest_blocked_interaction_receipt_summary(),
                    "successful_interaction_receipt": self.latest_successful_interaction_receipt_summary(),
                    "agent_browser_executor_readiness": self.latest_agent_browser_executor_readiness_summary(),
                    "agent_browser_noop_executor_attempt": self.latest_agent_browser_noop_executor_attempt_summary(),
                    "agent_browser_reload_executor_attempt": self.latest_agent_browser_reload_executor_attempt_summary(),
                    "agent_browser_clear_data_executor_attempt": self.latest_agent_browser_clear_data_executor_attempt_summary(),
                    "annotated_screenshot": self.latest_annotated_screenshot_summary(),
                },
                "handoff": {
                    "read_only_only": !interactive_unlocked,
                    "requires_fresh_preflight_before_input": true,
                    "requires_receipt_after_every_input": true,
                    "executor_wired": false,
                    "safe_to_send_to_agent_panel": true,
                },
            },
            "notes": [
                "This packet is a read-only state handoff for the Agent Browser Command Center.",
                "It does not execute click, type, key, scroll, navigation, viewport, cache, or other mutating browser actions.",
                "A future executor should treat this packet as context, then revalidate preflight immediately before every permissioned action."
            ],
        })
    }

    fn agent_browser_status_packet_json(packet: &Value) -> String {
        serde_json::to_string_pretty(packet).unwrap_or_else(|_| "{}".to_string())
    }

    fn agent_browser_status_packet_agent_blocks(&self, packet: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = packet.pointer("/packet/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview agent browser status packet:\n\n```json\n{}\n```",
            Self::agent_browser_status_packet_json(packet)
        ))));
        blocks
    }

    fn copy_agent_browser_status_packet(&mut self, window: &Window, cx: &mut Context<Self>) {
        let packet = self.agent_browser_status_packet(window);
        cx.write_to_clipboard(ClipboardItem::new_string(
            Self::agent_browser_status_packet_json(&packet),
        ));
        self.latest_agent_browser_status_packet = Some(packet);
        self.show_toast("Copied agent browser status packet", cx);
        cx.notify();
    }

    fn send_agent_browser_status_packet_to_agent(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let packet = self.agent_browser_status_packet(window);
        let blocks = self.agent_browser_status_packet_agent_blocks(&packet);
        self.latest_agent_browser_status_packet = Some(packet);
        self.append_content_blocks_to_agent_panel(blocks, window, cx);
        self.show_toast("Sent agent browser status packet to the agent panel", cx);
        cx.notify();
    }

    fn agent_browser_executor_readiness(&self, window: &Window) -> Value {
        let interactive_unlocked = self.agent_action_permission.interactive_enabled();
        let context_ready = self.latest_page_diagnostics.is_some()
            && self.latest_dom_snapshot.is_some()
            && self.latest_action_targets.is_some()
            && self.latest_readiness_probe.is_some();
        let observability_ready = self.latest_runtime_events.is_some();
        let wait_contract_ready = self.latest_wait_contract.is_some();
        let plan_ready = self.latest_interaction_plan.is_some();
        let preflight_ready = self.latest_interaction_preflight.is_some();
        let receipt_template_ready = self.latest_interaction_receipt_template.is_some();
        let action_request_ready = self.latest_interaction_action_request.is_some();
        let blocked_receipt_ready = self.latest_blocked_interaction_receipt.is_some();
        let success_receipt_ready = self.latest_successful_interaction_receipt.is_some();
        let audit_ready = wait_contract_ready
            && plan_ready
            && preflight_ready
            && receipt_template_ready
            && action_request_ready
            && blocked_receipt_ready
            && success_receipt_ready;
        let gate_ready_for_executor = interactive_unlocked && context_ready && audit_ready;

        let mut blockers = Vec::new();
        if !interactive_unlocked {
            blockers.push(serde_json::json!({
                "code": "interactive_actions_locked",
                "message": "The user has not unlocked interactive Agent Browser actions for this WebPreview session.",
                "required_action": "Use Allow Interactive Agent Actions only after the user explicitly approves browser control.",
            }));
        }
        if !context_ready {
            blockers.push(serde_json::json!({
                "code": "context_not_collected",
                "message": "Fresh page diagnostics, DOM, action targets, and readiness probe context are required before dispatch.",
                "missing": {
                    "page_diagnostics": self.latest_page_diagnostics.is_none(),
                    "dom_snapshot": self.latest_dom_snapshot.is_none(),
                    "action_targets": self.latest_action_targets.is_none(),
                    "readiness_probe": self.latest_readiness_probe.is_none(),
                },
            }));
        }
        if !audit_ready {
            blockers.push(serde_json::json!({
                "code": "audit_contract_incomplete",
                "message": "The executor must have the wait contract, plan, preflight, request envelope, and both receipt shapes before input dispatch.",
                "missing": {
                    "wait_contract": !wait_contract_ready,
                    "interaction_plan": !plan_ready,
                    "interaction_preflight": !preflight_ready,
                    "receipt_template": !receipt_template_ready,
                    "action_request": !action_request_ready,
                    "blocked_receipt": !blocked_receipt_ready,
                    "successful_receipt_template": !success_receipt_ready,
                },
            }));
        }

        blockers.push(serde_json::json!({
            "code": "executor_not_wired",
            "message": "This build exposes the readiness contract only; real click, type, key, scroll, navigation, viewport, and cache dispatch are still intentionally disabled.",
            "required_action": "Wire each executor action behind this readiness gate and emit a receipt for every attempted action.",
        }));

        let interactive_actions = INTERACTIVE_AGENT_BROWSER_ACTIONS
            .iter()
            .map(|action| {
                serde_json::json!({
                    "action": action,
                    "can_dispatch_now": false,
                    "gate_ready_for_executor": gate_ready_for_executor,
                    "requires_fresh_preflight": true,
                    "requires_receipt": true,
                    "executor_wired": false,
                })
            })
            .collect::<Vec<_>>();
        let status = if gate_ready_for_executor {
            "gate_ready_executor_not_wired"
        } else {
            "blocked_until_ready"
        };
        let next_step = if !context_ready {
            "Collect fresh diagnostics, DOM, action targets, and readiness context from the current page."
        } else if !audit_ready {
            "Collect the remaining wait, plan, preflight, request, and receipt artifacts before wiring execution."
        } else if !interactive_unlocked {
            "Ask the user to explicitly unlock interactive browser actions for this session before any future executor dispatch."
        } else {
            "Wire the first executor action behind this gate, then require fresh preflight and a receipt per dispatch."
        };

        serde_json::json!({
            "schema": "zed.web_preview.agent_browser_executor_readiness.v1",
            "session": self.browser_session_snapshot(window),
            "policy": self.agent_browser_policy_snapshot(),
            "readiness": {
                "captured_at_ms": Self::current_epoch_millis(),
                "session_id": self.session_id.as_ref(),
                "title": self.current_tab_title().as_ref(),
                "url": self.active_url.as_ref(),
                "status": status,
                "next_step": next_step,
                "can_dispatch_now": false,
                "gate_ready_for_executor": gate_ready_for_executor,
                "interactive_unlocked": interactive_unlocked,
                "context_ready": context_ready,
                "observability_ready": observability_ready,
                "audit_ready": audit_ready,
                "executor_wired": false,
                "requires_user_permission": true,
                "requires_fresh_preflight_before_every_action": true,
                "requires_receipt_after_every_action": true,
                "blockers": blockers,
                "interactive_actions": interactive_actions,
            },
            "latest": {
                "status_packet": self.latest_agent_browser_status_packet_summary(),
                "page_diagnostics": self.latest_page_diagnostics_summary(),
                "runtime_events": self.latest_runtime_events_summary(),
                "dom_snapshot": self.latest_dom_snapshot_summary(),
                "action_targets": self.latest_action_targets_summary(),
                "readiness_probe": self.latest_readiness_probe_summary(),
                "wait_contract": self.latest_wait_contract_summary(),
                "interaction_plan": self.latest_interaction_plan_summary(),
                "interaction_preflight": self.latest_interaction_preflight_summary(),
                "interaction_receipt_template": self.latest_interaction_receipt_template_summary(),
                "interaction_action_request": self.latest_interaction_action_request_summary(),
                "blocked_interaction_receipt": self.latest_blocked_interaction_receipt_summary(),
                "successful_interaction_receipt": self.latest_successful_interaction_receipt_summary(),
                "noop_executor_attempt": self.latest_agent_browser_noop_executor_attempt_summary(),
                "reload_executor_attempt": self.latest_agent_browser_reload_executor_attempt_summary(),
                "clear_data_executor_attempt": self.latest_agent_browser_clear_data_executor_attempt_summary(),
            },
            "notes": [
                "This readiness contract is read-only and does not dispatch browser input.",
                "Executors must stay disabled until they are wired behind this gate.",
                "Every future executor action must run a fresh preflight immediately before dispatch and emit either a blocked or successful receipt."
            ],
        })
    }

    fn agent_browser_executor_readiness_json(readiness: &Value) -> String {
        serde_json::to_string_pretty(readiness).unwrap_or_else(|_| "{}".to_string())
    }

    fn agent_browser_executor_readiness_agent_blocks(
        &self,
        readiness: &Value,
    ) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = readiness.pointer("/readiness/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview agent browser executor readiness:\n\n```json\n{}\n```",
            Self::agent_browser_executor_readiness_json(readiness)
        ))));
        blocks
    }

    fn copy_agent_browser_executor_readiness(&mut self, window: &Window, cx: &mut Context<Self>) {
        let readiness = self.agent_browser_executor_readiness(window);
        cx.write_to_clipboard(ClipboardItem::new_string(
            Self::agent_browser_executor_readiness_json(&readiness),
        ));
        self.latest_agent_browser_executor_readiness = Some(readiness);
        self.show_toast("Copied agent browser executor readiness", cx);
        cx.notify();
    }

    fn send_agent_browser_executor_readiness_to_agent(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let readiness = self.agent_browser_executor_readiness(window);
        let blocks = self.agent_browser_executor_readiness_agent_blocks(&readiness);
        self.latest_agent_browser_executor_readiness = Some(readiness);
        self.append_content_blocks_to_agent_panel(blocks, window, cx);
        self.show_toast(
            "Sent agent browser executor readiness to the agent panel",
            cx,
        );
        cx.notify();
    }

    fn agent_browser_noop_executor_attempt(&self, window: &Window) -> Value {
        let interactive_unlocked = self.agent_action_permission.interactive_enabled();
        let context_ready = self.latest_page_diagnostics.is_some()
            && self.latest_dom_snapshot.is_some()
            && self.latest_action_targets.is_some()
            && self.latest_readiness_probe.is_some();
        let audit_ready = self.latest_wait_contract.is_some()
            && self.latest_interaction_plan.is_some()
            && self.latest_interaction_preflight.is_some()
            && self.latest_interaction_receipt_template.is_some()
            && self.latest_interaction_action_request.is_some()
            && self.latest_blocked_interaction_receipt.is_some()
            && self.latest_successful_interaction_receipt.is_some();
        let gate_ready_for_executor = interactive_unlocked && context_ready && audit_ready;

        let mut blockers = Vec::new();
        if !interactive_unlocked {
            blockers.push(serde_json::json!({
                "code": "interactive_actions_locked",
                "message": "Interactive Agent Browser actions are locked for this WebPreview session.",
            }));
        }
        if !context_ready {
            blockers.push(serde_json::json!({
                "code": "context_not_collected",
                "message": "Fresh diagnostics, DOM, action targets, and readiness context are required before dispatch.",
            }));
        }
        if !audit_ready {
            blockers.push(serde_json::json!({
                "code": "audit_contract_incomplete",
                "message": "Wait, plan, preflight, request, and receipt artifacts must exist before dispatch.",
            }));
        }
        blockers.push(serde_json::json!({
            "code": "noop_executor_harness",
            "message": "This executor harness records the attempt and emits a blocked receipt without dispatching native or page input.",
        }));

        let blocker_codes = blockers
            .iter()
            .filter_map(|blocker| blocker.get("code").and_then(Value::as_str))
            .map(str::to_string)
            .collect::<Vec<_>>();
        let attempted_actions = INTERACTIVE_AGENT_BROWSER_ACTIONS
            .iter()
            .map(|action| {
                serde_json::json!({
                    "action": action,
                    "attempt_id": format!("noop-{action}"),
                    "mode": "no_op",
                    "dispatch_status": "blocked",
                    "would_pass_gate_if_executor_were_wired": gate_ready_for_executor,
                    "native_input_dispatched": false,
                    "page_script_dispatched": false,
                    "blocker_codes": &blocker_codes,
                    "receipt_required": true,
                })
            })
            .collect::<Vec<_>>();
        let receipt = serde_json::json!({
            "schema": "zed.web_preview.noop_executor_blocked_receipt.v1",
            "timestamp_ms": Self::current_epoch_millis(),
            "outcome": "blocked",
            "reason": "noop_executor_harness",
            "session_id": self.session_id.as_ref(),
            "url": self.active_url.as_ref(),
            "title": self.current_tab_title().as_ref(),
            "permission": self.agent_action_permission.snapshot(),
            "gate": {
                "interactive_unlocked": interactive_unlocked,
                "context_ready": context_ready,
                "audit_ready": audit_ready,
                "gate_ready_for_executor": gate_ready_for_executor,
            },
            "blockers": blockers,
            "attempted_action_count": attempted_actions.len(),
            "native_input_dispatched": false,
            "page_script_dispatched": false,
        });
        let receipt_blockers = receipt.pointer("/blockers").cloned();

        serde_json::json!({
            "schema": "zed.web_preview.agent_browser_noop_executor_attempt.v1",
            "session": self.browser_session_snapshot(window),
            "policy": self.agent_browser_policy_snapshot(),
            "attempt": {
                "captured_at_ms": Self::current_epoch_millis(),
                "session_id": self.session_id.as_ref(),
                "title": self.current_tab_title().as_ref(),
                "url": self.active_url.as_ref(),
                "mode": "no_op",
                "outcome": "blocked",
                "executor_wired": true,
                "native_dispatch_enabled": false,
                "page_script_dispatch_enabled": false,
                "gate_ready_for_executor": gate_ready_for_executor,
                "blockers": receipt_blockers,
                "attempted_actions": attempted_actions,
                "blocked_receipt": receipt,
                "latest_executor_readiness": self.latest_agent_browser_executor_readiness_summary(),
            },
            "notes": [
                "This harness proves the executor path, readiness check, and receipt shape without touching WebPreview native input.",
                "Every interactive action family is represented as blocked with native_input_dispatched=false.",
                "Real dispatch must remain disabled until a later slice wires one action behind fresh preflight and receipt emission."
            ],
        })
    }

    fn agent_browser_noop_executor_attempt_json(attempt: &Value) -> String {
        serde_json::to_string_pretty(attempt).unwrap_or_else(|_| "{}".to_string())
    }

    fn agent_browser_noop_executor_attempt_agent_blocks(
        &self,
        attempt: &Value,
    ) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = attempt.pointer("/attempt/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview no-op executor attempt:\n\n```json\n{}\n```",
            Self::agent_browser_noop_executor_attempt_json(attempt)
        ))));
        blocks
    }

    fn copy_agent_browser_noop_executor_attempt(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let attempt = self.agent_browser_noop_executor_attempt(window);
        cx.write_to_clipboard(ClipboardItem::new_string(
            Self::agent_browser_noop_executor_attempt_json(&attempt),
        ));
        self.latest_agent_browser_noop_executor_attempt = Some(attempt);
        self.show_toast("Copied no-op executor attempt", cx);
        cx.notify();
    }

    fn send_agent_browser_noop_executor_attempt_to_agent(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let attempt = self.agent_browser_noop_executor_attempt(window);
        let blocks = self.agent_browser_noop_executor_attempt_agent_blocks(&attempt);
        self.latest_agent_browser_noop_executor_attempt = Some(attempt);
        self.append_content_blocks_to_agent_panel(blocks, window, cx);
        self.show_toast("Sent no-op executor attempt to the agent panel", cx);
        cx.notify();
    }

    fn permissioned_reload_executor_attempt(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        send_to_agent: bool,
    ) {
        let captured_at_ms = Self::current_epoch_millis();
        let before = serde_json::json!({
            "url": self.active_url.as_ref(),
            "title": self.current_tab_title().as_ref(),
            "load_state": self.load_state_name(),
        });
        let interactive_unlocked = self.agent_action_permission.interactive_enabled();
        let context_ready = self.latest_page_diagnostics.is_some()
            && self.latest_dom_snapshot.is_some()
            && self.latest_action_targets.is_some()
            && self.latest_readiness_probe.is_some();
        let audit_ready = self.latest_wait_contract.is_some()
            && self.latest_interaction_plan.is_some()
            && self.latest_interaction_preflight.is_some()
            && self.latest_interaction_receipt_template.is_some()
            && self.latest_interaction_action_request.is_some()
            && self.latest_blocked_interaction_receipt.is_some()
            && self.latest_successful_interaction_receipt.is_some();
        let gate_ready_for_executor = interactive_unlocked && context_ready && audit_ready;

        let mut blockers = Vec::new();
        if !interactive_unlocked {
            blockers.push(serde_json::json!({
                "code": "interactive_actions_locked",
                "message": "Interactive Agent Browser actions are locked for this WebPreview session.",
            }));
        }
        if !context_ready {
            blockers.push(serde_json::json!({
                "code": "context_not_collected",
                "message": "Fresh page diagnostics, DOM, action targets, and readiness probe context are required before reload dispatch.",
            }));
        }
        if !audit_ready {
            blockers.push(serde_json::json!({
                "code": "audit_contract_incomplete",
                "message": "Reload dispatch requires the wait contract, interaction plan, preflight, request envelope, and receipt artifacts.",
            }));
        }

        let mut browser_command_dispatched = false;
        let mut dispatch_error = None;
        if gate_ready_for_executor {
            match self.reload_webview(window, cx) {
                Ok(()) => {
                    browser_command_dispatched = true;
                }
                Err(error) => {
                    let message = error.to_string();
                    self.load_state = PreviewLoadState::Error(message.clone().into());
                    dispatch_error = Some(message.clone());
                    blockers.push(serde_json::json!({
                        "code": "reload_dispatch_failed",
                        "message": message,
                    }));
                }
            }
        }

        let outcome = if browser_command_dispatched {
            "dispatched"
        } else {
            "blocked"
        };
        let after = serde_json::json!({
            "url": self.active_url.as_ref(),
            "title": self.current_tab_title().as_ref(),
            "load_state": self.load_state_name(),
        });
        let receipt = serde_json::json!({
            "schema": "zed.web_preview.permissioned_reload_executor_receipt.v1",
            "timestamp_ms": Self::current_epoch_millis(),
            "action": "reload",
            "outcome": outcome,
            "session_id": self.session_id.as_ref(),
            "url": self.active_url.as_ref(),
            "title": self.current_tab_title().as_ref(),
            "permission": self.agent_action_permission.snapshot(),
            "gate": {
                "interactive_unlocked": interactive_unlocked,
                "context_ready": context_ready,
                "audit_ready": audit_ready,
                "gate_ready_for_executor": gate_ready_for_executor,
            },
            "before": before,
            "after": after,
            "blockers": blockers,
            "dispatch_error": dispatch_error,
            "browser_command_dispatched": browser_command_dispatched,
            "native_input_dispatched": false,
            "page_script_dispatched": false,
        });
        let receipt_blockers = receipt.pointer("/blockers").cloned();

        let attempt = serde_json::json!({
            "schema": "zed.web_preview.permissioned_reload_executor_attempt.v1",
            "session": self.browser_session_snapshot(window),
            "policy": self.agent_browser_policy_snapshot(),
            "attempt": {
                "captured_at_ms": captured_at_ms,
                "session_id": self.session_id.as_ref(),
                "title": self.current_tab_title().as_ref(),
                "url": self.active_url.as_ref(),
                "action": "reload",
                "outcome": outcome,
                "gate_ready_for_executor": gate_ready_for_executor,
                "browser_command_dispatched": browser_command_dispatched,
                "native_input_dispatched": false,
                "page_script_dispatched": false,
                "blockers": receipt_blockers,
                "receipt": receipt,
                "latest_executor_readiness": self.latest_agent_browser_executor_readiness_summary(),
            },
            "notes": [
                "This is the first permission-gated executor shell.",
                "It only dispatches the existing native WebView reload command when permission, context, and audit gates are ready.",
                "It never sends click, type, key, wheel, pointer, or page-script input."
            ],
        });
        let blocks = self.permissioned_reload_executor_agent_blocks(&attempt);
        self.latest_agent_browser_reload_executor_attempt = Some(attempt.clone());

        if send_to_agent {
            self.append_content_blocks_to_agent_panel(blocks, window, cx);
            self.show_toast(
                "Sent permissioned reload executor receipt to the agent panel",
                cx,
            );
        } else {
            cx.write_to_clipboard(ClipboardItem::new_string(
                Self::permissioned_reload_executor_json(&attempt),
            ));
            self.show_toast("Copied permissioned reload executor receipt", cx);
        }
        cx.notify();
    }

    fn permissioned_reload_executor_json(attempt: &Value) -> String {
        serde_json::to_string_pretty(attempt).unwrap_or_else(|_| "{}".to_string())
    }

    fn permissioned_reload_executor_agent_blocks(&self, attempt: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = attempt.pointer("/attempt/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview permissioned reload executor attempt:\n\n```json\n{}\n```",
            Self::permissioned_reload_executor_json(attempt)
        ))));
        blocks
    }

    fn copy_permissioned_reload_executor_attempt(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.permissioned_reload_executor_attempt(window, cx, false);
    }

    fn send_permissioned_reload_executor_attempt_to_agent(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.permissioned_reload_executor_attempt(window, cx, true);
    }

    fn permissioned_clear_data_executor_attempt(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        send_to_agent: bool,
    ) {
        let captured_at_ms = Self::current_epoch_millis();
        let before = serde_json::json!({
            "url": self.active_url.as_ref(),
            "title": self.current_tab_title().as_ref(),
            "load_state": self.load_state_name(),
        });
        let interactive_unlocked = self.agent_action_permission.interactive_enabled();
        let context_ready = self.latest_page_diagnostics.is_some()
            && self.latest_dom_snapshot.is_some()
            && self.latest_action_targets.is_some()
            && self.latest_readiness_probe.is_some();
        let audit_ready = self.latest_wait_contract.is_some()
            && self.latest_interaction_plan.is_some()
            && self.latest_interaction_preflight.is_some()
            && self.latest_interaction_receipt_template.is_some()
            && self.latest_interaction_action_request.is_some()
            && self.latest_blocked_interaction_receipt.is_some()
            && self.latest_successful_interaction_receipt.is_some();
        let gate_ready_for_executor = interactive_unlocked && context_ready && audit_ready;

        let mut blockers = Vec::new();
        if !interactive_unlocked {
            blockers.push(serde_json::json!({
                "code": "interactive_actions_locked",
                "message": "Interactive Agent Browser actions are locked for this WebPreview session.",
            }));
        }
        if !context_ready {
            blockers.push(serde_json::json!({
                "code": "context_not_collected",
                "message": "Fresh page diagnostics, DOM, action targets, and readiness probe context are required before clearing browsing data.",
            }));
        }
        if !audit_ready {
            blockers.push(serde_json::json!({
                "code": "audit_contract_incomplete",
                "message": "Clear-data dispatch requires the wait contract, interaction plan, preflight, request envelope, and receipt artifacts.",
            }));
        }

        let mut browser_command_dispatched = false;
        let mut dispatch_error = None;
        if gate_ready_for_executor {
            match self.clear_all_browsing_data() {
                Ok(()) => {
                    browser_command_dispatched = true;
                }
                Err(error) => {
                    let message = error.to_string();
                    self.load_state = PreviewLoadState::Error(message.clone().into());
                    dispatch_error = Some(message.clone());
                    blockers.push(serde_json::json!({
                        "code": "clear_data_dispatch_failed",
                        "message": message,
                    }));
                }
            }
        }

        let outcome = if browser_command_dispatched {
            "dispatched"
        } else {
            "blocked"
        };
        let after = serde_json::json!({
            "url": self.active_url.as_ref(),
            "title": self.current_tab_title().as_ref(),
            "load_state": self.load_state_name(),
        });
        let receipt = serde_json::json!({
            "schema": "zed.web_preview.permissioned_clear_data_executor_receipt.v1",
            "timestamp_ms": Self::current_epoch_millis(),
            "action": "clear_data",
            "outcome": outcome,
            "session_id": self.session_id.as_ref(),
            "url": self.active_url.as_ref(),
            "title": self.current_tab_title().as_ref(),
            "permission": self.agent_action_permission.snapshot(),
            "gate": {
                "interactive_unlocked": interactive_unlocked,
                "context_ready": context_ready,
                "audit_ready": audit_ready,
                "gate_ready_for_executor": gate_ready_for_executor,
            },
            "scope": {
                "cache": true,
                "cookies_and_site_data": true,
                "storage_and_service_workers": true,
                "profile_reset": false,
            },
            "before": before,
            "after": after,
            "blockers": blockers,
            "dispatch_error": dispatch_error,
            "browser_command_dispatched": browser_command_dispatched,
            "native_input_dispatched": false,
            "page_script_dispatched": false,
        });
        let receipt_blockers = receipt.pointer("/blockers").cloned();

        let attempt = serde_json::json!({
            "schema": "zed.web_preview.permissioned_clear_data_executor_attempt.v1",
            "session": self.browser_session_snapshot(window),
            "policy": self.agent_browser_policy_snapshot(),
            "attempt": {
                "captured_at_ms": captured_at_ms,
                "session_id": self.session_id.as_ref(),
                "title": self.current_tab_title().as_ref(),
                "url": self.active_url.as_ref(),
                "action": "clear_data",
                "outcome": outcome,
                "gate_ready_for_executor": gate_ready_for_executor,
                "browser_command_dispatched": browser_command_dispatched,
                "native_input_dispatched": false,
                "page_script_dispatched": false,
                "blockers": receipt_blockers,
                "receipt": receipt,
                "latest_executor_readiness": self.latest_agent_browser_executor_readiness_summary(),
            },
            "notes": [
                "This permission-gated executor clears WebPreview browsing data through the native browser backend.",
                "It never sends click, type, key, wheel, pointer, or page-script input.",
                "Profile reset and imported/external browser profile cleanup remain out of scope for this action."
            ],
        });
        let blocks = self.permissioned_clear_data_executor_agent_blocks(&attempt);
        self.latest_agent_browser_clear_data_executor_attempt = Some(attempt.clone());

        if send_to_agent {
            self.append_content_blocks_to_agent_panel(blocks, window, cx);
            self.show_toast(
                "Sent permissioned clear-data executor receipt to the agent panel",
                cx,
            );
        } else {
            cx.write_to_clipboard(ClipboardItem::new_string(
                Self::permissioned_clear_data_executor_json(&attempt),
            ));
            self.show_toast("Copied permissioned clear-data executor receipt", cx);
        }
        cx.notify();
    }

    fn permissioned_clear_data_executor_json(attempt: &Value) -> String {
        serde_json::to_string_pretty(attempt).unwrap_or_else(|_| "{}".to_string())
    }

    fn permissioned_clear_data_executor_agent_blocks(
        &self,
        attempt: &Value,
    ) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = attempt.pointer("/attempt/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview permissioned clear-data executor attempt:\n\n```json\n{}\n```",
            Self::permissioned_clear_data_executor_json(attempt)
        ))));
        blocks
    }

    fn copy_permissioned_clear_data_executor_attempt(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.permissioned_clear_data_executor_attempt(window, cx, false);
    }

    fn send_permissioned_clear_data_executor_attempt_to_agent(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.permissioned_clear_data_executor_attempt(window, cx, true);
    }

    fn agent_browser_qa_runbook(&self, window: &Window) -> Value {
        let interactive_unlocked = self.agent_action_permission.interactive_enabled();
        let context_ready = self.latest_page_diagnostics.is_some()
            && self.latest_dom_snapshot.is_some()
            && self.latest_action_targets.is_some()
            && self.latest_readiness_probe.is_some();
        let audit_ready = self.latest_wait_contract.is_some()
            && self.latest_interaction_plan.is_some()
            && self.latest_interaction_preflight.is_some()
            && self.latest_interaction_receipt_template.is_some()
            && self.latest_interaction_action_request.is_some()
            && self.latest_blocked_interaction_receipt.is_some()
            && self.latest_successful_interaction_receipt.is_some();

        serde_json::json!({
            "schema": "zed.web_preview.agent_browser_qa_runbook.v1",
            "session": self.browser_session_snapshot(window),
            "policy": self.agent_browser_policy_snapshot(),
            "runbook": {
                "captured_at_ms": Self::current_epoch_millis(),
                "session_id": self.session_id.as_ref(),
                "title": self.current_tab_title().as_ref(),
                "url": self.active_url.as_ref(),
                "status": "agent_browser_command_center_complete_manual_qa_required",
                "completion_claim": {
                    "feature_set": "Agent Browser Command Center",
                    "score": 100,
                    "scope": "Read-only browser context, diagnostics, action planning, audit packets, and executor readiness gates are wired.",
                    "out_of_scope": "Real click, type, key, scroll, navigation, and viewport dispatch belongs to the next Permissioned Agent Browser Executor feature set."
                },
                "readiness_snapshot": {
                    "context_ready": context_ready,
                    "audit_ready": audit_ready,
                    "interactive_unlocked": interactive_unlocked,
                    "executor_wired": false,
                    "latest_status_packet": self.latest_agent_browser_status_packet_summary(),
                    "latest_executor_readiness": self.latest_agent_browser_executor_readiness_summary(),
                    "latest_noop_executor_attempt": self.latest_agent_browser_noop_executor_attempt_summary(),
                    "latest_reload_executor_attempt": self.latest_agent_browser_reload_executor_attempt_summary(),
                    "latest_clear_data_executor_attempt": self.latest_agent_browser_clear_data_executor_attempt_summary(),
                },
                "manual_gates": [
                    {
                        "name": "Editor regression guard",
                        "checks": [
                            "Type quickly in a normal Rust or text file; inserted text should appear immediately.",
                            "Caret should remain visible while typing and after focus switches.",
                            "Right-side panel copy/send commands must not steal editor text input."
                        ]
                    },
                    {
                        "name": "WebPreview interaction guard",
                        "checks": [
                            "Hover, click, right-click, mouse wheel, and keyboard input inside page fields continue to work.",
                            "Switch WebPreview to editor and back; stale focus must not route keys to the wrong surface.",
                            "Use the More menu Agent Browser commands; they should collect/copy/send context without reloading the page."
                        ]
                    },
                    {
                        "name": "Agent Panel handoff",
                        "checks": [
                            "Send session, diagnostics, DOM, action targets, readiness, wait contract, preflight, receipts, status packet, executor readiness, and this runbook to the Agent Panel.",
                            "Each handoff should include the URL attachment when a valid page URL is active.",
                            "JSON schemas should be readable and bounded for model context."
                        ]
                    },
                    {
                        "name": "Permission boundary",
                        "checks": [
                            "Interactive actions are locked by default.",
                            "Executor readiness still reports can_dispatch_now=false because no real executor is wired.",
                            "Unlocking the session changes policy/readiness state but does not execute browser input by itself."
                        ]
                    }
                ],
                "known_limits": [
                    "The Command Center currently prepares context and audit contracts only.",
                    "Real browser input dispatch is intentionally deferred to the next feature set.",
                    "Cross-platform behavior is represented by shared state and capability contracts; Windows remains the local manual QA platform."
                ],
                "next_feature_set": {
                    "name": "Permissioned Agent Browser Executor",
                    "target_score": 100,
                    "goal": "Wire real browser actions behind the readiness gate, preserving editor speed and WebPreview focus.",
                    "first_slices": [
                        "Add a no-op executor harness that records attempted actions and emits blocked receipts.",
                        "Wire one low-risk action behind explicit permission, fresh preflight, and receipt emission.",
                        "Expand action coverage only after manual QA confirms no editor or WebPreview input regression."
                    ]
                }
            },
            "notes": [
                "This runbook is a read-only completion handoff.",
                "It is safe to copy/send while editing because it does not evaluate page JavaScript or dispatch native input.",
                "Use this as the starting checklist before moving into the next executor feature set."
            ],
        })
    }

    fn agent_browser_qa_runbook_json(runbook: &Value) -> String {
        serde_json::to_string_pretty(runbook).unwrap_or_else(|_| "{}".to_string())
    }

    fn agent_browser_qa_runbook_agent_blocks(&self, runbook: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url) = runbook.pointer("/runbook/url").and_then(Value::as_str)
            && let Some(url_block) = self.url_attachment_block(url)
        {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview Agent Browser QA runbook:\n\n```json\n{}\n```",
            Self::agent_browser_qa_runbook_json(runbook)
        ))));
        blocks
    }

    fn copy_agent_browser_qa_runbook(&mut self, window: &Window, cx: &mut Context<Self>) {
        let runbook = self.agent_browser_qa_runbook(window);
        cx.write_to_clipboard(ClipboardItem::new_string(
            Self::agent_browser_qa_runbook_json(&runbook),
        ));
        self.latest_agent_browser_qa_runbook = Some(runbook);
        self.show_toast("Copied agent browser QA runbook", cx);
        cx.notify();
    }

    fn send_agent_browser_qa_runbook_to_agent(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let runbook = self.agent_browser_qa_runbook(window);
        let blocks = self.agent_browser_qa_runbook_agent_blocks(&runbook);
        self.latest_agent_browser_qa_runbook = Some(runbook);
        self.append_content_blocks_to_agent_panel(blocks, window, cx);
        self.show_toast("Sent agent browser QA runbook to the agent panel", cx);
        cx.notify();
    }

    fn agent_browser_action_manifest(&self, window: &Window) -> Value {
        serde_json::json!({
            "schema": "zed.web_preview.agent_browser_actions.v1",
            "session": self.browser_session_snapshot(window),
            "policy": self.agent_browser_policy_snapshot(),
            "notes": [
                "Read-only actions are always available for context gathering.",
                "Interactive actions must remain locked until the user explicitly allows them for this WebPreview session.",
                "Automation callers should re-read this manifest before running click, type, key, scroll, navigation, viewport, cache, or other mutating browser actions."
            ],
        })
    }

    fn agent_browser_action_manifest_json(&self, window: &Window) -> String {
        serde_json::to_string_pretty(&self.agent_browser_action_manifest(window))
            .unwrap_or_else(|_| "{}".to_string())
    }

    fn copy_agent_browser_action_manifest(&mut self, window: &Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(
            self.agent_browser_action_manifest_json(window),
        ));
        self.show_toast("Copied agent browser action manifest", cx);
    }

    fn send_agent_browser_action_manifest_to_agent(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let blocks = vec![acp::ContentBlock::Text(acp::TextContent::new(format!(
            "Web preview agent browser action manifest:\n\n```json\n{}\n```",
            self.agent_browser_action_manifest_json(window)
        )))];
        self.append_content_blocks_to_agent_panel(blocks, window, cx);
        self.show_toast("Sent agent browser action manifest to the agent panel", cx);
    }

    fn agent_plugin_catalog(&self, window: &Window) -> Value {
        let workspace_tools_root = self
            .workspace_context
            .root_path
            .as_ref()
            .map(|root| root.join("tools"));
        let workspace_plugin_root = workspace_tools_root
            .as_ref()
            .map(|root| root.join("agent-plugins"));
        let zed_plugin_root = data_dir().join("agent-plugins");

        serde_json::json!({
            "schema": "zed.agent_plugins.catalog.v1",
            "session": self.browser_session_snapshot(window),
            "catalog": {
                "generated_at_ms": Self::current_epoch_millis(),
                "name": "DX Agent Plugin Runtime",
                "status": "discovery_layer_available",
                "tool_name": "list_agent_plugins",
                "default_enabled_plugins": ["zed.browser", "zed.chrome", "zed.pc_use"],
                "available_to": [
                    "agent_panel",
                    "subagents",
                    "acp_threads",
                    "web_preview_agent_handoff"
                ],
                "bootstrap_plan": {
                    "default_download": true,
                    "download_policy": "download_or_update_on_first_use",
                    "zed_data_plugin_root": zed_plugin_root.display().to_string(),
                    "workspace_plugin_root": workspace_plugin_root
                        .as_ref()
                        .map(|path| path.display().to_string()),
                    "workspace_tools_root": workspace_tools_root
                        .as_ref()
                        .map(|path| path.display().to_string()),
                    "dx_chrome_extension": {
                        "install_policy": "download_or_update_on_first_use",
                        "preferred_root": workspace_plugin_root
                            .as_ref()
                            .map(|root| root.join("dx-chrome-extension"))
                            .unwrap_or_else(|| zed_plugin_root.join("dx-chrome-extension"))
                            .display()
                            .to_string(),
                        "load_mode": "unpacked_extension",
                        "never_write_to_user_browser_profiles": true
                    },
                    "playwright": {
                        "install_policy": "download_or_update_on_first_use",
                        "preferred_root": workspace_tools_root
                            .as_ref()
                            .map(|root| root.join("playwright"))
                            .unwrap_or_else(|| zed_plugin_root.join("playwright"))
                            .display()
                            .to_string(),
                        "managed_by": "DX Code Editor"
                    }
                },
                "permission_model": {
                    "read_only_discovery_without_prompt": true,
                    "browser_interactions_require_explicit_session_unlock": true,
                    "external_chrome_and_pc_use_require_user_visible_permission": true,
                    "receipts_required_for_every_mutating_action": true,
                    "fresh_preflight_required_before_input": true
                },
                "plugins": [
                    {
                        "id": "zed.browser",
                        "name": "Browser",
                        "kind": "built_in",
                        "status": "available",
                        "default_enabled": true,
                        "scope": "in_app_web_preview",
                        "runtime": {
                            "backend": "web_preview",
                            "requires_external_process": false,
                            "native_backend": self.browser_native_backend_name()
                        },
                        "entrypoints": [
                            "WebPreview More menu",
                            "Agent Panel content handoff",
                            "list_agent_plugins tool"
                        ],
                        "capabilities": [
                            {"id": "browser.sessions.list", "state": "available", "description": "List open WebPreview sessions and workspace inventory."},
                            {"id": "browser.session.snapshot", "state": "available", "description": "Read the active WebPreview session metadata, bounds, profile, URL, and policy."},
                            {"id": "browser.page.diagnostics", "state": "available", "description": "Collect ready state, title, URL, DOM counts, and page metadata."},
                            {"id": "browser.dom.snapshot", "state": "available", "description": "Collect a bounded DOM tree snapshot for model context."},
                            {"id": "browser.runtime.events", "state": "available", "description": "Read bounded console, page-error, fetch, and XHR event buffers."},
                            {"id": "browser.screenshot.capture", "state": "available", "description": "Capture WebPreview screenshots for Agent Panel attachments."},
                            {"id": "browser.screenshot.area", "state": "available", "description": "Capture a selected WebPreview rectangle for Agent Panel attachments."},
                            {"id": "browser.screenshot.annotate", "state": "available", "description": "Draw page annotations and capture the marked WebPreview screenshot with metadata."},
                            {"id": "browser.element.inspect", "state": "available", "description": "Pick a page element and send selector, HTML, computed styles, rect, and screenshot context to the Agent Panel."},
                            {"id": "browser.devtools.open", "state": "available", "description": "Open the native browser DevTools for the active WebPreview backend."},
                            {"id": "browser.viewport.responsive", "state": "available", "description": "Switch the active WebPreview between full, phone, tablet, laptop, and rotated responsive viewports."},
                            {"id": "browser.action.reload", "state": "available_when_unlocked", "description": "Reload through the permissioned WebPreview executor shell."},
                            {"id": "browser.action.clear_data", "state": "available_when_unlocked", "description": "Clear WebPreview browsing data through the permissioned executor shell."},
                            {"id": "browser.action.click", "state": "planned_executor", "description": "Click visible page targets after unlock, fresh preflight, and receipt logging."},
                            {"id": "browser.action.type", "state": "planned_executor", "description": "Type into page inputs after unlock, fresh preflight, and receipt logging."},
                            {"id": "browser.action.key", "state": "planned_executor", "description": "Send key presses after unlock, fresh preflight, and receipt logging."},
                            {"id": "browser.action.scroll", "state": "planned_executor", "description": "Scroll page or element targets after unlock, fresh preflight, and receipt logging."}
                        ],
                        "safety": {
                            "interactive_locked_by_default": true,
                            "uses_current_webpreview_profile": true,
                            "does_not_mutate_external_browser_profiles": true,
                            "requires_receipts": true
                        }
                    },
                    {
                        "id": "zed.chrome",
                        "name": "Chrome",
                        "kind": "built_in",
                        "status": "requires_bootstrap",
                        "default_enabled": true,
                        "scope": "external_chrome_playwright_dx_extension",
                        "runtime": {
                            "backend": "playwright",
                            "requires_node": true,
                            "requires_managed_chrome": true,
                            "requires_dx_chrome_extension": true,
                            "profile_policy": "managed_profile_only"
                        },
                        "capabilities": [
                            {"id": "chrome.session.launch", "state": "requires_bootstrap", "description": "Launch or attach to a managed Chrome profile."},
                            {"id": "chrome.page.open_url", "state": "requires_bootstrap", "description": "Open URLs in managed Chrome tabs."},
                            {"id": "chrome.page.click", "state": "requires_permission", "description": "Click elements through Playwright locators or extension targets."},
                            {"id": "chrome.page.type", "state": "requires_permission", "description": "Type into focused inputs through Playwright or extension bridge."},
                            {"id": "chrome.page.press_key", "state": "requires_permission", "description": "Press keyboard shortcuts in managed Chrome."},
                            {"id": "chrome.page.scroll", "state": "requires_permission", "description": "Scroll pages and containers in managed Chrome."},
                            {"id": "chrome.page.screenshot", "state": "requires_bootstrap", "description": "Capture full-page or viewport screenshots."},
                            {"id": "chrome.page.dom_snapshot", "state": "requires_bootstrap", "description": "Read DOM/accessibility snapshots."},
                            {"id": "chrome.runtime.console", "state": "requires_bootstrap", "description": "Read console, page errors, and network events."},
                            {"id": "chrome.extension.bridge", "state": "requires_bootstrap", "description": "Use the DX Chrome extension bridge for pages where DevTools-only control is insufficient."}
                        ],
                        "safety": {
                            "managed_profile_only": true,
                            "explicit_permission_required_for_input": true,
                            "receipts_required": true,
                            "os_wide_control": false
                        }
                    },
                    {
                        "id": "zed.pc_use",
                        "name": "PC Use",
                        "kind": "built_in",
                        "status": "planned_permission_gate",
                        "default_enabled": true,
                        "scope": "zed_window_and_permissioned_desktop",
                        "runtime": {
                            "backend": "zed_window_runtime",
                            "os_wide_automation": "requires_separate_explicit_permission"
                        },
                        "capabilities": [
                            {"id": "pc.zed_window.screenshot", "state": "planned", "description": "Capture Zed-window screenshots for agent context."},
                            {"id": "pc.zed_window.focus", "state": "planned", "description": "Focus Zed panes, panels, and tabs by safe editor-native handles."},
                            {"id": "pc.zed_window.click", "state": "planned_permission_gate", "description": "Click within Zed surfaces only after permission and target preflight."},
                            {"id": "pc.zed_window.type", "state": "planned_permission_gate", "description": "Type within Zed surfaces only after permission and target preflight."},
                            {"id": "pc.zed_window.inspect_ui", "state": "planned", "description": "Read safe UI metadata for currently visible Zed surfaces."},
                            {"id": "pc.desktop.os_wide", "state": "blocked_by_default", "description": "OS-wide desktop automation remains unavailable until the user explicitly enables it."}
                        ],
                        "safety": {
                            "zed_window_first": true,
                            "os_wide_actions_blocked_by_default": true,
                            "explicit_permission_required_for_input": true,
                            "receipts_required": true
                        }
                    }
                ]
            },
            "notes": [
                "The Browser plugin is available now through WebPreview context and permissioned reload.",
                "Chrome and PC Use are default-enabled plugin manifests with bootstrap and permission gates defined before executor wiring.",
                "The Agent can also call the read-only list_agent_plugins tool to discover this catalog from any Agent Panel."
            ]
        })
    }

    fn browser_native_backend_name(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        {
            "webview2_composition"
        }
        #[cfg(target_os = "macos")]
        {
            "wkwebview"
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            "unavailable"
        }
    }

    fn agent_plugin_catalog_json(catalog: &Value) -> String {
        serde_json::to_string_pretty(catalog).unwrap_or_else(|_| "{}".to_string())
    }

    fn agent_plugin_catalog_agent_blocks(&self, catalog: &Value) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        if let Some(url_block) = self.current_url_attachment_block() {
            blocks.push(url_block);
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
            "DX/Zed agent plugin catalog:\n\n```json\n{}\n```",
            Self::agent_plugin_catalog_json(catalog)
        ))));
        blocks
    }

    fn copy_agent_plugin_catalog(&mut self, window: &Window, cx: &mut Context<Self>) {
        let catalog = self.agent_plugin_catalog(window);
        cx.write_to_clipboard(ClipboardItem::new_string(Self::agent_plugin_catalog_json(
            &catalog,
        )));
        self.latest_agent_plugin_catalog = Some(catalog);
        self.show_toast("Copied agent plugin catalog", cx);
        cx.notify();
    }

    fn send_agent_plugin_catalog_to_agent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let catalog = self.agent_plugin_catalog(window);
        let blocks = self.agent_plugin_catalog_agent_blocks(&catalog);
        self.latest_agent_plugin_catalog = Some(catalog);
        self.append_content_blocks_to_agent_panel(blocks, window, cx);
        self.show_toast("Sent agent plugin catalog to the agent panel", cx);
        cx.notify();
    }

    fn set_agent_action_permission(
        &mut self,
        permission: AgentBrowserActionPermission,
        cx: &mut Context<Self>,
    ) {
        self.agent_action_permission = permission;
        self.show_toast(
            format!(
                "Agent browser actions: {}",
                self.agent_action_permission.label()
            ),
            cx,
        );
        cx.notify();
    }

    fn go_back_in_history(&mut self, cx: &mut Context<Self>) {
        let _ = self.evaluate_script("history.back();");
        cx.notify();
    }

    fn go_forward_in_history(&mut self, cx: &mut Context<Self>) {
        let _ = self.evaluate_script("history.forward();");
        cx.notify();
    }

    fn zoom_in_step(&mut self, cx: &mut Context<Self>) {
        self.zoom_factor = (self.zoom_factor + 0.1).min(3.0);
        let _ = self.apply_zoom();
        cx.notify();
    }

    fn zoom_out_step(&mut self, cx: &mut Context<Self>) {
        self.zoom_factor = (self.zoom_factor - 0.1).max(0.25);
        let _ = self.apply_zoom();
        cx.notify();
    }

    fn set_viewport_mode(
        &mut self,
        mode: PreviewViewportMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.viewport_mode = mode;
        if let Err(error) = self.sync_current_viewport_bounds(window) {
            self.report_action_error("Viewport change failed", error, cx);
        } else {
            self.show_toast(format!("Viewport: {}", self.viewport_label()), cx);
        }
        cx.notify();
    }

    fn rotate_viewport(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(rotated) = self.viewport_mode.rotated() else {
            self.show_toast("Select a fixed viewport before rotating", cx);
            return;
        };

        self.set_viewport_mode(rotated, window, cx);
    }

    fn open_devtools(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            let opened = {
                let borrow = self.native_preview.borrow();
                if let Some(preview) = borrow.as_ref() {
                    preview.open_devtools();
                    true
                } else {
                    false
                }
            };
            if opened {
                self.show_toast("Opened DevTools", cx);
            }
        }
    }

    fn inspect_element(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let script = "window.__zedWebPreview && window.__zedWebPreview.inspectNextElement();";
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(script))) {
            Ok(Ok(())) => {
                self.show_toast("Click an element in the page to send it to the agent.", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Element selector is unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic("Element selector crashed before it could start", cx);
            }
        }
    }

    fn capture_selected_area_screenshot(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let script = "window.__zedWebPreview && window.__zedWebPreview.captureAreaScreenshot();";
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(script))) {
            Ok(Ok(())) => {
                self.show_toast("Drag a page area to capture it for the agent.", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Selected-area capture is unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic("Selected-area capture crashed before it could start", cx);
            }
        }
    }

    fn annotate_screenshot(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let script = "window.__zedWebPreview && window.__zedWebPreview.startAnnotationMode();";
        match catch_unwind(AssertUnwindSafe(|| self.evaluate_script(script))) {
            Ok(Ok(())) => {
                self.show_toast("Drag to annotate. Enter captures; Escape cancels.", cx);
            }
            Ok(Err(error)) => {
                self.report_action_error("Annotated screenshot mode is unavailable", error, cx);
            }
            Err(_) => {
                self.report_action_panic(
                    "Annotated screenshot mode crashed before it could start",
                    cx,
                );
            }
        }
    }

    fn clear_cache(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = self.clear_all_browsing_data() {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        } else {
            self.show_toast("Cleared browser cache", cx);
        }
        cx.notify();
    }

    fn open_extension_location(
        &mut self,
        extension_name: SharedString,
        extension_path: PathBuf,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let url = if extension_path.is_dir() {
            url::Url::from_directory_path(&extension_path)
        } else {
            url::Url::from_file_path(&extension_path)
        };

        match url {
            Ok(url) => {
                cx.open_url(url.as_str());
                self.show_toast(format!("Opened {} extension folder", extension_name), cx);
            }
            Err(()) => {
                self.load_state = PreviewLoadState::Error(
                    format!("Failed to open extension path {}", extension_path.display()).into(),
                );
            }
        }
    }

    fn take_screenshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match catch_unwind(AssertUnwindSafe(|| {
            self.capture_screenshot_payload(None, window)
        })) {
            Ok(Ok((_path, image, blocks))) => {
                cx.write_to_clipboard(ClipboardItem::new_image(&image));
                self.append_content_blocks_to_agent_panel(blocks, window, cx);
                self.show_toast(
                    "Captured web preview screenshot to clipboard and AI input",
                    cx,
                );
            }
            Ok(Err(error)) => {
                self.report_action_error("Web preview screenshot failed", error, cx);
            }
            Err(_) => {
                self.report_action_panic("Web preview screenshot crashed", cx);
            }
        }
        cx.notify();
    }

    fn annotated_screenshot_snapshot(&self, payload: &Value, window: &Window) -> Value {
        let mut annotation = payload.clone();
        if let Some(object) = annotation.as_object_mut() {
            object.remove("kind");
        }

        serde_json::json!({
            "schema": "zed.web_preview.annotated_screenshot.v1",
            "capture": {
                "captured_at_ms": Self::current_epoch_millis(),
                "source": "web_preview_annotation_overlay",
            },
            "session": self.browser_session_snapshot(window),
            "annotation": annotation,
        })
    }

    fn annotated_screenshot_json(screenshot: &Value) -> String {
        serde_json::to_string_pretty(screenshot).unwrap_or_else(|_| "{}".to_string())
    }

    fn report_action_error(&mut self, prefix: &str, error: anyhow::Error, cx: &mut Context<Self>) {
        let message = format!("{prefix}: {error}");
        self.load_state = PreviewLoadState::Error(message.clone().into());
        self.show_toast(message, cx);
    }

    fn report_action_panic(&mut self, message: &str, cx: &mut Context<Self>) {
        let message = message.to_string();
        self.load_state = PreviewLoadState::Error(message.clone().into());
        self.show_toast(message, cx);
    }

    fn apply_browser_events(
        &mut self,
        events: Vec<BrowserEvent>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut tab_updated = false;
        let mut refocus_after_navigation = false;

        for event in events {
            match event {
                BrowserEvent::UrlChanged(url) => {
                    let previous_url = self.active_url.to_string();
                    self.active_url = url.clone().into();
                    let editor_text = self.current_url_text(cx);
                    let should_sync_editor = !self.url_editor_focus_handle.is_focused(window)
                        || editor_text.is_empty()
                        || editor_text == previous_url;

                    if should_sync_editor {
                        self.url_editor.update(cx, |editor, cx| {
                            editor.set_text(url.as_str(), window, cx);
                        });
                    }
                }
                BrowserEvent::TitleChanged(title) => {
                    self.page_title = Some(title.into());
                    tab_updated = true;
                }
                BrowserEvent::NavigationStarted => {
                    self.load_state = PreviewLoadState::Loading;
                    self.page_title = None;
                    tab_updated = true;
                }
                BrowserEvent::NavigationCompleted => {
                    self.load_state = PreviewLoadState::Ready;
                    refocus_after_navigation = true;
                }
                BrowserEvent::IpcMessage(message) => {
                    self.deferred_ipc_messages.push(message);
                }
                BrowserEvent::MountFailed(error) => {
                    self.native_mount_requested.set(false);
                    self.load_state = PreviewLoadState::Error(error.into());
                }
            }
        }

        self.flush_deferred_ipc(window, cx);

        #[cfg(target_os = "windows")]
        if refocus_after_navigation && self.should_focus_native_preview_page(window) {
            self.focus_native_preview_page();
        }

        if tab_updated {
            cx.emit(ItemEvent::UpdateTab);
        }
        cx.notify();
    }

    fn flush_deferred_ipc(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.deferred_ipc_messages.is_empty() || self.ipc_flush_scheduled {
            return;
        }

        self.ipc_flush_scheduled = true;
        cx.on_next_frame(window, |this, window, cx| {
            this.ipc_flush_scheduled = false;
            let pending = std::mem::take(&mut this.deferred_ipc_messages);
            for message in pending {
                if let Err(error) = this.handle_ipc_message(&message, window, cx) {
                    this.load_state = PreviewLoadState::Error(error.to_string().into());
                }
            }
            cx.notify();
        });
    }

    fn handle_ipc_message(
        &mut self,
        message: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let payload: Value =
            serde_json::from_str(message).with_context(|| "Invalid Web Preview bridge payload")?;
        let kind = payload
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Browser bridge payload is missing kind"))?;

        match kind {
            "inspect-element" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let raw_rect = payload.get("rect").and_then(parse_browser_rect);
                    let scale = payload
                        .get("scale")
                        .and_then(Value::as_f64)
                        .filter(|value| *value > 0.0)
                        .unwrap_or(1.0);
                    let data = BrowserAgentPayload::InspectElement {
                        url: payload
                            .get("url")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        title: payload
                            .get("title")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        selector: payload
                            .get("selector")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        tag: payload
                            .get("tag")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        id: payload
                            .get("id")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        classes: payload
                            .get("classes")
                            .and_then(Value::as_array)
                            .map(|classes| {
                                classes
                                    .iter()
                                    .filter_map(Value::as_str)
                                    .map(ToOwned::to_owned)
                                    .collect()
                            })
                            .unwrap_or_default(),
                        text: payload
                            .get("text")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        href: payload
                            .get("href")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        src: payload
                            .get("src")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        rect: raw_rect,
                        css: payload
                            .get("css")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        html: payload
                            .get("html")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                    };
                    let screenshot_png = raw_rect
                        .map(|rect| BrowserRect {
                            x: rect.x * scale,
                            y: rect.y * scale,
                            width: rect.width * scale,
                            height: rect.height * scale,
                        })
                        .map(|crop| self.capture_screenshot_png_bytes(Some(crop), window))
                        .transpose()
                        .ok()
                        .flatten()
                        .and_then(|png_bytes| self.prepare_agent_png_bytes(png_bytes).ok());
                    self.append_content_blocks_to_agent_panel(
                        self.inspect_element_agent_blocks(&data, screenshot_png.as_deref()),
                        window,
                        cx,
                    );
                    Ok(())
                })) {
                    Ok(Ok(())) => {
                        self.show_toast("Sent inspected element to the agent panel", cx);
                    }
                    Ok(Err(error)) => {
                        self.report_action_error("Element selector failed to send data", error, cx);
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Element selector crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "page-diagnostics" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("copy");
                    let diagnostics = self.page_diagnostics_snapshot(&payload, window);
                    self.latest_page_diagnostics = Some(diagnostics.clone());

                    match action {
                        "agent" => {
                            self.append_content_blocks_to_agent_panel(
                                self.page_diagnostics_agent_blocks(&diagnostics),
                                window,
                                cx,
                            );
                            self.show_toast("Sent page diagnostics to the agent panel", cx);
                        }
                        _ => {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                Self::page_diagnostics_json(&diagnostics),
                            ));
                            self.show_toast("Copied web preview page diagnostics JSON", cx);
                        }
                    }

                    Ok(())
                })) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        self.report_action_error("Page diagnostics failed", error, cx);
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Page diagnostics crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "runtime-events" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("copy");
                    let events = self.runtime_events_snapshot(&payload, window);
                    self.latest_runtime_events = Some(events.clone());

                    match action {
                        "agent" => {
                            self.append_content_blocks_to_agent_panel(
                                self.runtime_events_agent_blocks(&events),
                                window,
                                cx,
                            );
                            self.show_toast("Sent runtime events to the agent panel", cx);
                        }
                        _ => {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                Self::runtime_events_json(&events),
                            ));
                            self.show_toast("Copied web preview runtime events JSON", cx);
                        }
                    }

                    Ok(())
                })) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        self.report_action_error("Runtime event collection failed", error, cx);
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Runtime events crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "dom-snapshot" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("copy");
                    let snapshot = self.dom_snapshot_snapshot(&payload, window);
                    self.latest_dom_snapshot = Some(snapshot.clone());

                    match action {
                        "agent" => {
                            self.append_content_blocks_to_agent_panel(
                                self.dom_snapshot_agent_blocks(&snapshot),
                                window,
                                cx,
                            );
                            self.show_toast("Sent DOM snapshot to the agent panel", cx);
                        }
                        _ => {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                Self::dom_snapshot_json(&snapshot),
                            ));
                            self.show_toast("Copied web preview DOM snapshot JSON", cx);
                        }
                    }

                    Ok(())
                })) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        self.report_action_error("DOM snapshot collection failed", error, cx);
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "DOM snapshot crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "action-targets" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("copy");
                    let targets = self.action_targets_snapshot(&payload, window);
                    self.latest_action_targets = Some(targets.clone());

                    match action {
                        "agent" => {
                            self.append_content_blocks_to_agent_panel(
                                self.action_targets_agent_blocks(&targets),
                                window,
                                cx,
                            );
                            self.show_toast("Sent action targets to the agent panel", cx);
                        }
                        _ => {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                Self::action_targets_json(&targets),
                            ));
                            self.show_toast("Copied web preview action targets JSON", cx);
                        }
                    }

                    Ok(())
                })) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        self.report_action_error("Action target collection failed", error, cx);
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Action targets crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "readiness-probe" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("copy");
                    let probe = self.readiness_probe_snapshot(&payload, window);
                    self.latest_readiness_probe = Some(probe.clone());

                    match action {
                        "agent" => {
                            self.append_content_blocks_to_agent_panel(
                                self.readiness_probe_agent_blocks(&probe),
                                window,
                                cx,
                            );
                            self.show_toast("Sent readiness probe to the agent panel", cx);
                        }
                        _ => {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                Self::readiness_probe_json(&probe),
                            ));
                            self.show_toast("Copied web preview readiness probe JSON", cx);
                        }
                    }

                    Ok(())
                })) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        self.report_action_error("Readiness probe collection failed", error, cx);
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Readiness probe crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "wait-contract" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("copy");
                    let contract = self.wait_contract_snapshot(&payload, window);
                    self.latest_wait_contract = Some(contract.clone());

                    match action {
                        "agent" => {
                            self.append_content_blocks_to_agent_panel(
                                self.wait_contract_agent_blocks(&contract),
                                window,
                                cx,
                            );
                            self.show_toast("Sent wait contract to the agent panel", cx);
                        }
                        _ => {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                Self::wait_contract_json(&contract),
                            ));
                            self.show_toast("Copied web preview wait contract JSON", cx);
                        }
                    }

                    Ok(())
                })) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        self.report_action_error("Wait contract collection failed", error, cx);
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Wait contract crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "interaction-plan" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("copy");
                    let plan = self.interaction_plan_snapshot(&payload, window);
                    self.latest_interaction_plan = Some(plan.clone());

                    match action {
                        "agent" => {
                            self.append_content_blocks_to_agent_panel(
                                self.interaction_plan_agent_blocks(&plan),
                                window,
                                cx,
                            );
                            self.show_toast("Sent interaction plan to the agent panel", cx);
                        }
                        _ => {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                Self::interaction_plan_json(&plan),
                            ));
                            self.show_toast("Copied web preview interaction plan JSON", cx);
                        }
                    }

                    Ok(())
                })) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        self.report_action_error("Interaction plan collection failed", error, cx);
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Interaction plan crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "interaction-preflight" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("copy");
                    let preflight = self.interaction_preflight_snapshot(&payload, window);
                    self.latest_interaction_preflight = Some(preflight.clone());

                    match action {
                        "agent" => {
                            self.append_content_blocks_to_agent_panel(
                                self.interaction_preflight_agent_blocks(&preflight),
                                window,
                                cx,
                            );
                            self.show_toast("Sent interaction preflight to the agent panel", cx);
                        }
                        _ => {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                Self::interaction_preflight_json(&preflight),
                            ));
                            self.show_toast("Copied web preview interaction preflight JSON", cx);
                        }
                    }

                    Ok(())
                })) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        self.report_action_error(
                            "Interaction preflight collection failed",
                            error,
                            cx,
                        );
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Interaction preflight crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "interaction-receipt-template" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("copy");
                    let template = self.interaction_receipt_template_snapshot(&payload, window);
                    self.latest_interaction_receipt_template = Some(template.clone());

                    match action {
                        "agent" => {
                            self.append_content_blocks_to_agent_panel(
                                self.interaction_receipt_template_agent_blocks(&template),
                                window,
                                cx,
                            );
                            self.show_toast(
                                "Sent interaction receipt template to the agent panel",
                                cx,
                            );
                        }
                        _ => {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                Self::interaction_receipt_template_json(&template),
                            ));
                            self.show_toast(
                                "Copied web preview interaction receipt template JSON",
                                cx,
                            );
                        }
                    }

                    Ok(())
                })) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        self.report_action_error(
                            "Interaction receipt template collection failed",
                            error,
                            cx,
                        );
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Interaction receipt template crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "interaction-action-request" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("copy");
                    let request = self.interaction_action_request_snapshot(&payload, window);
                    self.latest_interaction_action_request = Some(request.clone());

                    match action {
                        "agent" => {
                            self.append_content_blocks_to_agent_panel(
                                self.interaction_action_request_agent_blocks(&request),
                                window,
                                cx,
                            );
                            self.show_toast(
                                "Sent interaction action request to the agent panel",
                                cx,
                            );
                        }
                        _ => {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                Self::interaction_action_request_json(&request),
                            ));
                            self.show_toast(
                                "Copied web preview interaction action request JSON",
                                cx,
                            );
                        }
                    }

                    Ok(())
                })) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        self.report_action_error(
                            "Interaction action request collection failed",
                            error,
                            cx,
                        );
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Interaction action request crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "blocked-interaction-receipt" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("copy");
                    let receipt = self.blocked_interaction_receipt_snapshot(&payload, window);
                    self.latest_blocked_interaction_receipt = Some(receipt.clone());

                    match action {
                        "agent" => {
                            self.append_content_blocks_to_agent_panel(
                                self.blocked_interaction_receipt_agent_blocks(&receipt),
                                window,
                                cx,
                            );
                            self.show_toast(
                                "Sent blocked interaction receipt to the agent panel",
                                cx,
                            );
                        }
                        _ => {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                Self::blocked_interaction_receipt_json(&receipt),
                            ));
                            self.show_toast(
                                "Copied web preview blocked interaction receipt JSON",
                                cx,
                            );
                        }
                    }

                    Ok(())
                })) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        self.report_action_error(
                            "Blocked interaction receipt collection failed",
                            error,
                            cx,
                        );
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Blocked interaction receipt crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "successful-interaction-receipt" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let action = payload
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or("copy");
                    let receipt = self.successful_interaction_receipt_snapshot(&payload, window);
                    self.latest_successful_interaction_receipt = Some(receipt.clone());

                    match action {
                        "agent" => {
                            self.append_content_blocks_to_agent_panel(
                                self.successful_interaction_receipt_agent_blocks(&receipt),
                                window,
                                cx,
                            );
                            self.show_toast(
                                "Sent successful interaction receipt template to the agent panel",
                                cx,
                            );
                        }
                        _ => {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                Self::successful_interaction_receipt_json(&receipt),
                            ));
                            self.show_toast(
                                "Copied web preview successful interaction receipt template JSON",
                                cx,
                            );
                        }
                    }

                    Ok(())
                })) {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        self.report_action_error(
                            "Successful interaction receipt collection failed",
                            error,
                            cx,
                        );
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Successful interaction receipt crashed while collecting page data",
                            cx,
                        );
                    }
                }
            }
            "annotated-screenshot" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let screenshot = self.annotated_screenshot_snapshot(&payload, window);
                    self.latest_annotated_screenshot = Some(screenshot.clone());
                    let captured = self.capture_screenshot_payload(None, window);
                    let _ = self.evaluate_script(
                        "window.__zedWebPreview && window.__zedWebPreview.clearActiveOverlay();",
                    );
                    let (_path, image, mut blocks) = captured?;
                    blocks.push(acp::ContentBlock::Text(acp::TextContent::new(
                        "\n\nAnnotated screenshot metadata:\n\n".to_string(),
                    )));
                    blocks.push(acp::ContentBlock::Text(acp::TextContent::new(format!(
                        "```json\n{}\n```",
                        Self::annotated_screenshot_json(&screenshot)
                    ))));
                    cx.write_to_clipboard(ClipboardItem::new_image(&image));
                    self.append_content_blocks_to_agent_panel(blocks, window, cx);
                    Ok(())
                })) {
                    Ok(Ok(())) => {
                        self.show_toast(
                            "Captured annotated web preview screenshot to clipboard and AI input",
                            cx,
                        );
                    }
                    Ok(Err(error)) => {
                        self.report_action_error("Annotated screenshot failed", error, cx);
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Annotated screenshot crashed while processing",
                            cx,
                        );
                    }
                }
            }
            "capture-area" => {
                match catch_unwind(AssertUnwindSafe(|| -> Result<()> {
                    let scale = payload
                        .get("scale")
                        .and_then(Value::as_f64)
                        .filter(|value| *value > 0.0)
                        .unwrap_or(1.0);
                    let rect = payload
                        .get("rect")
                        .and_then(parse_browser_rect)
                        .ok_or_else(|| anyhow!("Capture area payload is missing a rectangle"))?;
                    let crop = BrowserRect {
                        x: rect.x * scale,
                        y: rect.y * scale,
                        width: rect.width * scale,
                        height: rect.height * scale,
                    };
                    let (_path, image, blocks) =
                        self.capture_screenshot_payload(Some(crop), window)?;
                    cx.write_to_clipboard(ClipboardItem::new_image(&image));
                    self.append_content_blocks_to_agent_panel(blocks, window, cx);
                    Ok(())
                })) {
                    Ok(Ok(())) => {
                        self.show_toast(
                            "Captured selected web preview area to clipboard and AI input",
                            cx,
                        );
                    }
                    Ok(Err(error)) => {
                        self.report_action_error("Selected-area screenshot failed", error, cx);
                    }
                    Err(_) => {
                        self.report_action_panic(
                            "Selected-area screenshot crashed while processing",
                            cx,
                        );
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn append_content_blocks_to_agent_panel(
        &mut self,
        blocks: Vec<acp::ContentBlock>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if blocks.is_empty() {
            return;
        }

        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        let workspace_for_retry = workspace.downgrade();
        window.defer(cx, move |window, cx| {
            let Some(workspace) = workspace_for_retry.upgrade() else {
                return;
            };

            let mut needs_retry = false;
            let retry_blocks = blocks.clone();
            workspace.update(cx, |workspace, cx| {
                if workspace.panel::<AgentPanel>(cx).is_none() {
                    workspace.open_panel::<AgentPanel>(window, cx);
                }

                let Some(panel) = workspace
                    .focus_panel::<AgentPanel>(window, cx)
                    .or_else(|| workspace.panel::<AgentPanel>(cx))
                else {
                    needs_retry = true;
                    return;
                };

                panel.update(cx, |panel, cx| {
                    panel.insert_content_blocks(blocks, window, cx);
                });
            });

            if needs_retry {
                let workspace_for_second_retry = workspace.downgrade();
                window.defer(cx, move |window, cx| {
                    let Some(workspace) = workspace_for_second_retry.upgrade() else {
                        return;
                    };

                    workspace.update(cx, |workspace, cx| {
                        let Some(panel) = workspace
                            .focus_panel::<AgentPanel>(window, cx)
                            .or_else(|| workspace.panel::<AgentPanel>(cx))
                        else {
                            return;
                        };

                        panel.update(cx, |panel, cx| {
                            panel.insert_content_blocks(retry_blocks, window, cx);
                        });
                    });
                });
            }
        });
    }

    fn inspect_element_agent_blocks(
        &self,
        payload: &BrowserAgentPayload,
        screenshot_png_bytes: Option<&[u8]>,
    ) -> Vec<acp::ContentBlock> {
        let mut blocks = Vec::new();
        let mut has_context_attachment = false;

        if let Some(png_bytes) = screenshot_png_bytes {
            blocks.push(acp::ContentBlock::Image(acp::ImageContent::new(
                base64::engine::general_purpose::STANDARD.encode(png_bytes),
                "image/png",
            )));
            has_context_attachment = true;
        }

        if let Some(url_block) = self.inspect_element_url_attachment_block(payload) {
            if has_context_attachment {
                blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n")));
            }
            blocks.push(url_block);
            has_context_attachment = true;
        }

        if has_context_attachment {
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n\n")));
        }

        blocks.push(acp::ContentBlock::Text(acp::TextContent::new(
            format_agent_note(payload),
        )));
        blocks
    }

    fn show_toast(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        let message = message.into();
        let workspace = workspace.downgrade();
        cx.defer(move |cx| {
            let Some(workspace) = workspace.upgrade() else {
                return;
            };

            let _ = workspace.update(cx, |workspace, cx| {
                workspace.show_toast(
                    Toast::new(
                        NotificationId::named("web-preview-toast".into()),
                        message.to_string(),
                    )
                    .autohide(),
                    cx,
                );
            });
        });
    }

    fn ensure_extensions_scanned(&mut self, cx: &mut Context<Self>) {
        if self.extensions_scanned {
            return;
        }

        self.detected_extensions = scan_local_extensions().unwrap_or_default();
        self.extensions_scanned = true;
        cx.notify();
    }

    fn render_tab_bar_add_menu(&self) -> impl IntoElement {
        IconButton::new("web-preview-tab-bar-add-trigger", IconName::Plus)
            .icon_size(IconSize::Small)
            .tooltip(Tooltip::text("New Web Preview"))
            .on_click(|_, window, cx| {
                window.dispatch_action(NewWebPreview.boxed_clone(), cx);
            })
    }

    fn render_tab_bar_extensions_menu(&self, entity: Entity<Self>) -> impl IntoElement {
        PopoverMenu::new("web-preview-tab-bar-extensions-menu")
            .trigger_with_tooltip(
                IconButton::new("web-preview-tab-bar-extensions-trigger", IconName::Blocks)
                    .icon_size(IconSize::Small),
                Tooltip::text("Extensions"),
            )
            .anchor(Anchor::TopRight)
            .menu(move |window, cx| {
                let detected_extensions = entity.update(cx, |this, cx| {
                    this.ensure_extensions_scanned(cx);
                    this.detected_extensions.clone()
                });

                Some(ContextMenu::build(window, cx, {
                    let entity = entity.clone();
                    move |menu, _, _| {
                        if detected_extensions.is_empty() {
                            menu.item(
                                ContextMenuEntry::new("No local browser extensions detected")
                                    .icon(IconName::Blocks)
                                    .disabled(true),
                            )
                        } else {
                            let mut menu = menu;
                            for extension in detected_extensions.iter().cloned() {
                                let label = format!("{} ({})", extension.name, extension.browser);
                                let entity = entity.clone();

                                menu = menu.item(
                                    ContextMenuEntry::new(label).icon(IconName::Blocks).handler(
                                        move |window, cx| {
                                            let _ = entity.update(cx, |this, cx| {
                                                this.open_extension_location(
                                                    extension.name.clone(),
                                                    extension.path.clone(),
                                                    window,
                                                    cx,
                                                );
                                            });
                                        },
                                    ),
                                );
                            }
                            menu
                        }
                    }
                }))
            })
    }

    fn render_tab_bar_more_menu(&self, entity: Entity<Self>) -> impl IntoElement {
        PopoverMenu::new("web-preview-tab-bar-more-menu")
            .trigger_with_tooltip(
                IconButton::new("web-preview-tab-bar-more-trigger", IconName::Ellipsis)
                    .icon_size(IconSize::Small),
                Tooltip::text("More"),
            )
            .anchor(Anchor::TopRight)
            .menu(move |window, cx| {
                let (_is_bookmarked, bookmark_icon, bookmark_label) =
                    entity.read_with(cx, |this, _| {
                        let is_bookmarked = this.is_active_url_bookmarked();
                        let bookmark_icon = if is_bookmarked {
                            IconName::StarFilled
                        } else {
                            IconName::Star
                        };
                        let bookmark_label = if is_bookmarked {
                            "Remove Bookmark"
                        } else {
                            "Bookmark Page"
                        };

                        (is_bookmarked, bookmark_icon, bookmark_label)
                    });

                Some(ContextMenu::build(window, cx, {
                    let entity = entity.clone();
                    move |menu, _, _| {
                        menu.item(
                            ContextMenuEntry::new(bookmark_label)
                                .icon(bookmark_icon)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.toggle_bookmark_for_active_url(cx);
                                        });
                                    }
                                }),
                        )
                        .separator()
                        .item(
                            ContextMenuEntry::new("Copy Session Info")
                                .icon(IconName::Info)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_browser_session_info(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Session JSON")
                                .icon(IconName::Binary)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_browser_session_json(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy All Session JSON")
                                .icon(IconName::Blocks)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_workspace_session_inventory_json(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send All Sessions to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_workspace_session_inventory_to_agent(
                                                window, cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Session to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_browser_session_info_to_agent(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Page Diagnostics")
                                .icon(IconName::Info)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_page_diagnostics(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Page Diagnostics to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_page_diagnostics_to_agent(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Runtime Events")
                                .icon(IconName::Info)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_runtime_events(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Runtime Events to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_runtime_events_to_agent(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy DOM Snapshot")
                                .icon(IconName::Code)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_dom_snapshot(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send DOM Snapshot to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_dom_snapshot_to_agent(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Action Targets")
                                .icon(IconName::Crosshair)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_action_targets(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Action Targets to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_action_targets_to_agent(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Readiness Probe")
                                .icon(IconName::LoadCircle)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_readiness_probe(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Readiness Probe to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_readiness_probe_to_agent(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Wait Contract")
                                .icon(IconName::Clock)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_wait_contract(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Wait Contract to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_wait_contract_to_agent(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Interaction Plan")
                                .icon(IconName::ToolThink)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_interaction_plan(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Interaction Plan to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_interaction_plan_to_agent(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Interaction Preflight")
                                .icon(IconName::LockOutlined)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_interaction_preflight(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Interaction Preflight to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_interaction_preflight_to_agent(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Receipt Template")
                                .icon(IconName::FileTextOutlined)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_interaction_receipt_template(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Receipt Template to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_interaction_receipt_template_to_agent(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Action Request")
                                .icon(IconName::QueueMessage)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_interaction_action_request(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Action Request to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_interaction_action_request_to_agent(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Blocked Receipt")
                                .icon(IconName::Warning)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_blocked_interaction_receipt(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Blocked Receipt to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_blocked_interaction_receipt_to_agent(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Success Receipt Template")
                                .icon(IconName::Check)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_successful_interaction_receipt(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Success Receipt Template to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_successful_interaction_receipt_to_agent(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Agent Browser Status")
                                .icon(IconName::Info)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_agent_browser_status_packet(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Agent Browser Status")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_agent_browser_status_packet_to_agent(
                                                window, cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Executor Readiness")
                                .icon(IconName::Check)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_agent_browser_executor_readiness(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Executor Readiness")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_agent_browser_executor_readiness_to_agent(
                                                window, cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy No-op Executor Attempt")
                                .icon(IconName::Warning)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_agent_browser_noop_executor_attempt(
                                                window, cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send No-op Executor Attempt")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_agent_browser_noop_executor_attempt_to_agent(
                                                window, cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Run Reload Executor")
                                .icon(IconName::RotateCw)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_permissioned_reload_executor_attempt(
                                                window, cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Run Reload Executor to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_permissioned_reload_executor_attempt_to_agent(
                                                window, cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Run Clear Data Executor")
                                .icon(IconName::Trash)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_permissioned_clear_data_executor_attempt(
                                                window, cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Run Clear Data Executor to Agent")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_permissioned_clear_data_executor_attempt_to_agent(
                                                window, cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Agent Browser QA Runbook")
                                .icon(IconName::Check)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_agent_browser_qa_runbook(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Agent Browser QA Runbook")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_agent_browser_qa_runbook_to_agent(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Agent Browser Manifest")
                                .icon(IconName::Info)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_agent_browser_action_manifest(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Agent Browser Manifest")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_agent_browser_action_manifest_to_agent(
                                                window, cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Copy Agent Plugin Catalog")
                                .icon(IconName::Blocks)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.copy_agent_plugin_catalog(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Send Agent Plugin Catalog")
                                .icon(IconName::AiZed)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.send_agent_plugin_catalog_to_agent(window, cx);
                                        });
                                    }
                                }),
                        )
                        .separator()
                        .item(
                            ContextMenuEntry::new("Allow Interactive Agent Actions")
                                .icon(IconName::Warning)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.set_agent_action_permission(
                                                AgentBrowserActionPermission::Interactive,
                                                cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Lock Agent Browser Actions")
                                .icon(IconName::LockOutlined)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.set_agent_action_permission(
                                                AgentBrowserActionPermission::ReadOnly,
                                                cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Take Screenshot")
                                .icon(IconName::Screen)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.take_screenshot(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Capture Area")
                                .icon(IconName::Crosshair)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.capture_selected_area_screenshot(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Annotate Screenshot")
                                .icon(IconName::Pencil)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.annotate_screenshot(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Inspect Element")
                                .icon(IconName::Code)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.inspect_element(window, cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Open DevTools")
                                .icon(IconName::Terminal)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.open_devtools(window, cx);
                                        });
                                    }
                                }),
                        )
                        .separator()
                        .item(
                            ContextMenuEntry::new("Zoom In")
                                .icon(IconName::Plus)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.zoom_in_step(cx);
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Zoom Out")
                                .icon(IconName::Dash)
                                .handler({
                                    let entity = entity.clone();
                                    move |_, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.zoom_out_step(cx);
                                        });
                                    }
                                }),
                        )
                        .separator()
                        .item(
                            ContextMenuEntry::new("Viewport: Full")
                                .icon(IconName::Screen)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.set_viewport_mode(
                                                PreviewViewportMode::FULL,
                                                window,
                                                cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Viewport: iPhone 15")
                                .icon(IconName::Screen)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.set_viewport_mode(
                                                PreviewViewportMode::IPHONE_15,
                                                window,
                                                cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Viewport: iPad Air")
                                .icon(IconName::Screen)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.set_viewport_mode(
                                                PreviewViewportMode::IPAD_AIR,
                                                window,
                                                cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Viewport: Laptop")
                                .icon(IconName::Screen)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.set_viewport_mode(
                                                PreviewViewportMode::LAPTOP,
                                                window,
                                                cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .item(
                            ContextMenuEntry::new("Rotate Viewport")
                                .icon(IconName::RotateCw)
                                .handler({
                                    let entity = entity.clone();
                                    move |window, cx| {
                                        let _ = entity.update(cx, |this, cx| {
                                            this.rotate_viewport(window, cx);
                                        });
                                    }
                                }),
                        )
                        .separator()
                        .item(
                            ContextMenuEntry::new("Clear Cache")
                                .icon(IconName::Trash)
                                .handler(move |window, cx| {
                                    let _ = entity.update(cx, |this, cx| {
                                        this.clear_cache(window, cx);
                                    });
                                }),
                        )
                    }
                }))
            })
    }

    fn render_tab_bar_start_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .items_center()
            .gap_1()
            .child(
                IconButton::new("web-preview-tab-bar-back", IconName::ArrowLeft)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Back"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.go_back_in_history(cx);
                    })),
            )
            .child(
                IconButton::new("web-preview-tab-bar-forward", IconName::ArrowRight)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Forward"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.go_forward_in_history(cx);
                    })),
            )
            .child(
                IconButton::new("web-preview-tab-bar-reload", IconName::RotateCw)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Reload"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.reload_page(window, cx);
                    })),
            )
            .into_any_element()
    }

    fn render_tab_bar_end_controls(
        &self,
        entity: Entity<Self>,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        h_flex()
            .items_center()
            .gap_1()
            .child(self.render_tab_bar_add_menu())
            .child(self.render_tab_bar_extensions_menu(entity.clone()))
            .child(self.render_tab_bar_more_menu(entity))
            .into_any_element()
    }

    #[cfg(target_os = "windows")]
    fn ensure_native_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.native_preview.borrow().is_some() || self.native_mount_requested.get() {
            return;
        }

        let Ok(parent_window) = RawParentWindow::from_window(window) else {
            self.load_state = PreviewLoadState::Error(
                "The native web preview could not access the editor window handle.".into(),
            );
            return;
        };

        self.native_mount_requested.set(true);
        let request = NativePreviewMountRequest {
            entity_id: cx.entity().entity_id(),
            parent_window,
            profile_dir: self.workspace_context.profile_dir.clone(),
            initial_url: self.active_url.to_string(),
            zoom_factor: self.zoom_factor,
            scale_factor: window.scale_factor(),
            initially_visible: self.is_active_item,
            host_bounds: self.host_bounds.clone(),
            browser_events: self.browser_events.clone(),
            native_mount_requested: self.native_mount_requested.clone(),
            native_preview: self.native_preview.clone(),
        };

        self.native_mount_task = Some(cx.spawn(move |_this, cx: &mut AsyncApp| {
            mount_native_preview(request.clone());
            cx.update(|app| app.notify(request.entity_id));
            async {}
        }));
    }

    #[cfg(target_os = "macos")]
    fn ensure_native_preview(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        if self.native_preview.borrow().is_some() || self.native_mount_requested.get() {
            return;
        }

        self.native_mount_requested.set(true);
        let result = create_native_preview_for_macos_window(
            window,
            self.workspace_context.profile_dir.clone(),
            self.active_url.to_string(),
            self.zoom_factor,
            self.host_bounds.clone(),
            self.browser_events.clone(),
            self.native_preview.clone(),
        );
        self.native_mount_requested.set(false);

        if let Err(error) = result {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        }
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn ensure_native_preview(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.load_state = PreviewLoadState::Error(
            "Native web preview underlay support is wired for Windows and macOS. Linux still needs GTK/X11 integration or a separate Wayland embedding path.".into(),
        );
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn load_url(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.load_state = PreviewLoadState::Loading;
        self.ensure_native_preview(window, cx);
        let mut borrow = self.native_preview.borrow_mut();
        if let Some(preview) = borrow.as_mut() {
            preview.load_url(url)?;
        } else if !self.native_mount_requested.get() {
            return Err(anyhow!("The native web preview is not available"));
        }
        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn load_url(
        &mut self,
        _url: &str,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn reload_webview(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.load_state = PreviewLoadState::Loading;
        self.ensure_native_preview(window, cx);
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.reload()?;
        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn reload_webview(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn evaluate_script(&self, script: &str) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.evaluate_script(script)?;
        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn evaluate_script(&self, _script: &str) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn apply_zoom(&self) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.zoom(self.zoom_factor)?;
        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn apply_zoom(&self) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn sync_current_viewport_bounds(&self, window: &Window) -> Result<()> {
        let Some(layout_bounds) = *self.layout_bounds.borrow() else {
            return Ok(());
        };

        let viewport_bounds = viewport_bounds_for_layout(layout_bounds, self.viewport_mode);
        *self.host_bounds.borrow_mut() = Some(viewport_bounds);
        let mut borrow = self.native_preview.borrow_mut();
        if let Some(preview) = borrow.as_mut() {
            #[cfg(target_os = "windows")]
            {
                preview.sync_bounds(viewport_bounds, window.scale_factor())?;
            }

            #[cfg(target_os = "macos")]
            {
                let _ = window;
                set_webview_bounds(&preview.webview, viewport_bounds)?;
                *self.last_applied_bounds.borrow_mut() = Some(viewport_bounds);
            }
        }

        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn sync_current_viewport_bounds(&self, _window: &Window) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn clear_all_browsing_data(&self) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.clear_all_browsing_data()?;
        Ok(())
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    fn clear_all_browsing_data(&self) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(target_os = "windows")]
    fn capture_and_store_screenshot(
        &self,
        crop: Option<BrowserRect>,
        window: &Window,
    ) -> Result<PathBuf> {
        let bounds = *self
            .host_bounds
            .borrow()
            .as_ref()
            .ok_or_else(|| anyhow!("The web preview is not visible yet"))?;
        let rect = screen_rect_for_bounds(window, bounds)?;
        let mut image = capture_screen_rect(rect)?;
        if let Some(crop) = crop {
            image = crop_image(image, crop)?;
        }
        let screenshots_dir = self.workspace_context.profile_dir.join("screenshots");
        fs::create_dir_all(&screenshots_dir)
            .with_context(|| "Failed to create the screenshots directory")?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let path = screenshots_dir.join(format!("web-preview-{timestamp}.png"));
        image
            .save(&path)
            .with_context(|| format!("Failed to save screenshot to {}", path.display()))?;
        Ok(path)
    }

    #[cfg(not(target_os = "windows"))]
    fn capture_and_store_screenshot(
        &self,
        _crop: Option<BrowserRect>,
        _window: &Window,
    ) -> Result<PathBuf> {
        Err(anyhow!(
            "Web preview screenshots are not available on this platform"
        ))
    }

    fn screenshot_agent_blocks(&self, png_bytes: &[u8]) -> Vec<acp::ContentBlock> {
        let mut blocks = vec![acp::ContentBlock::Image(acp::ImageContent::new(
            base64::engine::general_purpose::STANDARD.encode(png_bytes),
            "image/png",
        ))];

        if let Some(url_block) = self.current_url_attachment_block() {
            blocks.push(acp::ContentBlock::Text(acp::TextContent::new("\n")));
            blocks.push(url_block);
        }

        blocks
    }

    fn current_url_attachment_block(&self) -> Option<acp::ContentBlock> {
        self.url_attachment_block(self.active_url.trim())
    }

    fn inspect_element_url_attachment_block(
        &self,
        payload: &BrowserAgentPayload,
    ) -> Option<acp::ContentBlock> {
        match payload {
            BrowserAgentPayload::InspectElement { url, .. } => self.url_attachment_block(url),
        }
    }

    fn url_attachment_block(&self, url: &str) -> Option<acp::ContentBlock> {
        let url = url.trim();
        let parsed = url::Url::parse(url).ok()?;
        match parsed.scheme() {
            "http" | "https" => Some(acp::ContentBlock::ResourceLink(acp::ResourceLink::new(
                url.to_string(),
                url.to_string(),
            ))),
            _ => None,
        }
    }

    #[cfg(target_os = "windows")]
    fn prepare_agent_png_bytes(&self, png_bytes: Vec<u8>) -> Result<Vec<u8>> {
        const MAX_AGENT_IMAGE_BYTES: usize = 2 * 1024 * 1024;
        const MAX_AGENT_IMAGE_EDGE: u32 = 1600;

        let Ok(image) = image::load_from_memory(&png_bytes) else {
            return Ok(png_bytes);
        };

        if png_bytes.len() <= MAX_AGENT_IMAGE_BYTES
            && image.width() <= MAX_AGENT_IMAGE_EDGE
            && image.height() <= MAX_AGENT_IMAGE_EDGE
        {
            return Ok(png_bytes);
        }

        let resized = image.resize(
            MAX_AGENT_IMAGE_EDGE,
            MAX_AGENT_IMAGE_EDGE,
            FilterType::Lanczos3,
        );
        let mut cursor = Cursor::new(Vec::new());
        resized
            .write_to(&mut cursor, ExternalImageFormat::Png)
            .with_context(|| "Failed to encode the AI screenshot attachment")?;
        Ok(cursor.into_inner())
    }

    #[cfg(not(target_os = "windows"))]
    fn prepare_agent_png_bytes(&self, png_bytes: Vec<u8>) -> Result<Vec<u8>> {
        Ok(png_bytes)
    }

    #[cfg(target_os = "windows")]
    fn capture_screenshot_png_bytes(
        &self,
        crop: Option<BrowserRect>,
        window: &Window,
    ) -> Result<Vec<u8>> {
        let path = self.capture_and_store_screenshot(crop, window)?;
        fs::read(&path)
            .with_context(|| format!("Failed to read screenshot bytes from {}", path.display()))
    }

    #[cfg(not(target_os = "windows"))]
    fn capture_screenshot_png_bytes(
        &self,
        _crop: Option<BrowserRect>,
        _window: &Window,
    ) -> Result<Vec<u8>> {
        Err(anyhow!(
            "Web preview screenshots are not available on this platform"
        ))
    }

    #[cfg(target_os = "windows")]
    fn capture_screenshot_payload(
        &self,
        crop: Option<BrowserRect>,
        window: &Window,
    ) -> Result<(PathBuf, GpuiImage, Vec<acp::ContentBlock>)> {
        let path = self.capture_and_store_screenshot(crop, window)?;
        let png_bytes = fs::read(&path)
            .with_context(|| format!("Failed to read screenshot from {}", path.display()))?;
        let agent_png_bytes = self.prepare_agent_png_bytes(png_bytes.clone())?;
        let image = GpuiImage::from_bytes(GpuiImageFormat::Png, png_bytes);
        let blocks = self.screenshot_agent_blocks(&agent_png_bytes);
        Ok((path, image, blocks))
    }

    #[cfg(not(target_os = "windows"))]
    fn capture_screenshot_payload(
        &self,
        _crop: Option<BrowserRect>,
        _window: &Window,
    ) -> Result<(PathBuf, GpuiImage, Vec<acp::ContentBlock>)> {
        Err(anyhow!(
            "Web preview screenshots are not available on this platform"
        ))
    }

    fn render_webview_body(&self, cx: &mut Context<Self>) -> AnyElement {
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            let layout_bounds = self.layout_bounds.clone();
            let host_bounds = self.host_bounds.clone();
            let viewport_mode = self.viewport_mode;
            #[cfg(target_os = "macos")]
            let last_applied_bounds = self.last_applied_bounds.clone();
            let native_preview = self.native_preview.clone();

            let canvas = canvas(
                move |bounds, window, _cx| {
                    *layout_bounds.borrow_mut() = Some(bounds);
                    let viewport_bounds = viewport_bounds_for_layout(bounds, viewport_mode);
                    *host_bounds.borrow_mut() = Some(viewport_bounds);
                    #[cfg(target_os = "windows")]
                    let preview_ready = native_preview.borrow().is_some();
                    if let Some(preview) = native_preview.borrow_mut().as_mut() {
                        #[cfg(target_os = "windows")]
                        {
                            let _ = preview.sync_bounds(viewport_bounds, window.scale_factor());
                        }

                        #[cfg(target_os = "macos")]
                        {
                            let should_update_bounds =
                                last_applied_bounds.borrow().as_ref().copied()
                                    != Some(viewport_bounds);
                            if should_update_bounds {
                                let _ = set_webview_bounds(&preview.webview, viewport_bounds);
                                *last_applied_bounds.borrow_mut() = Some(viewport_bounds);
                            }
                        }
                    }
                    #[cfg(target_os = "windows")]
                    if preview_ready {
                        let passthrough_hitbox =
                            window.insert_hitbox(viewport_bounds, gpui::HitboxBehavior::Normal);
                        window.insert_mouse_passthrough_region(&passthrough_hitbox);
                    }
                    bounds
                },
                |_bounds, _state, _window, _cx| {},
            )
            .size_full();

            #[cfg(target_os = "windows")]
            {
                let focus_handle = self.focus_handle(cx);
                return div()
                    .size_full()
                    .track_focus(&focus_handle)
                    .child(canvas)
                    .into_any_element();
            }

            #[cfg(not(target_os = "windows"))]
            {
                let _ = cx;
                return canvas.into_any_element();
            }
        }

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    Label::new("Web Preview native embedding is wired for Windows and macOS. Linux still needs compositor-specific host integration.")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .into_any_element();
        }
    }
}

impl EventEmitter<ItemEvent> for WebPreviewView {}

impl Focusable for WebPreviewView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl project::ProjectItem for WebPreviewFileItem {
    fn try_open(
        project: &Entity<Project>,
        path: &ProjectPath,
        cx: &mut App,
    ) -> Option<Task<Result<Entity<Self>>>> {
        let (absolute_path, entry_id) = {
            let project_ref = project.read(cx);
            (
                project_ref.absolute_path(path, cx)?,
                project_ref.entry_for_path(path, cx).map(|entry| entry.id),
            )
        };
        let kind = preview_file_kind_for_path(&absolute_path)?;
        let title = absolute_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| kind.label())
            .to_string();
        let project_path = path.clone();

        Some(Task::ready(Ok(cx.new(|_| Self {
            project_path,
            entry_id,
            absolute_path,
            title: title.into(),
            kind,
        }))))
    }

    fn entry_id(&self, _: &App) -> Option<ProjectEntryId> {
        self.entry_id
    }

    fn project_path(&self, _: &App) -> Option<ProjectPath> {
        Some(self.project_path.clone())
    }

    fn is_dirty(&self) -> bool {
        false
    }
}

impl WorkspaceProjectItem for WebPreviewView {
    type Item = WebPreviewFileItem;

    fn project_item_kind() -> Option<ProjectItemKind> {
        Some(ProjectItemKind("WebPreviewMediaFile"))
    }

    fn for_project_item(
        _project: Entity<Project>,
        pane: Option<&Pane>,
        item: Entity<Self::Item>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let preview_url = item
            .read(cx)
            .preview_url()
            .unwrap_or_else(|| DEFAULT_WEB_PREVIEW_URL.to_string());
        let title = Some(item.read(cx).title.clone());
        let workspace = pane
            .map(|pane| pane.workspace())
            .unwrap_or_else(WeakEntity::new_invalid);
        let workspace_context = workspace
            .upgrade()
            .map(|workspace| {
                let workspace = workspace.read(cx);
                Self::workspace_context(&workspace, cx)
            })
            .unwrap_or_else(|| PreviewWorkspaceContext {
                workspace_id: None,
                root_path: None,
                root_name: "workspace".into(),
                preview_key: "media-preview".into(),
                profile_dir: data_dir().join("media-preview"),
            });

        Self::new_for_url(
            workspace,
            workspace_context,
            preview_url,
            title,
            Some(item),
            window,
            cx,
        )
    }
}

impl WebPreviewFileItem {
    fn preview_url(&self) -> Option<String> {
        web_preview_file_url(&self.absolute_path, &self.title, self.kind)
    }
}

impl Item for WebPreviewView {
    type Event = ItemEvent;

    fn tab_content(&self, params: TabContentParams, window: &Window, _cx: &App) -> AnyElement {
        let editor_focused = params.selected
            && !self.native_preview_has_keyboard_focus(window)
            && (self.url_editor_focus_requested.get()
                || self.url_editor_focus_handle.is_focused(window));
        div()
            .min_w_0()
            .h_full()
            .flex()
            .items_center()
            .child(if editor_focused {
                div()
                    .w(px(240.))
                    .min_w_0()
                    .child(self.url_editor.clone())
                    .into_any_element()
            } else {
                Label::new(self.current_tab_title())
                    .single_line()
                    .truncate()
                    .color(params.text_color())
                    .into_any_element()
            })
            .into_any_element()
    }

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        self.current_tab_title()
    }

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<ui::Icon> {
        let icon = self
            .project_item
            .as_ref()
            .map(|item| item.read(_cx).kind.icon())
            .unwrap_or(IconName::Public);
        Some(ui::Icon::new(icon))
    }

    fn tab_tooltip_text(&self, _cx: &App) -> Option<SharedString> {
        (!self.active_url.trim().is_empty()).then(|| self.active_url.clone())
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("Web Preview Opened")
    }

    fn screen_kind(&self) -> WorkspaceScreenKind {
        WorkspaceScreenKind::Browser
    }

    fn requires_transparent_workspace_background() -> bool {
        true
    }

    fn show_toolbar(&self) -> bool {
        false
    }

    fn buffer_kind(&self, _cx: &App) -> ItemBufferKind {
        if self.project_item.is_some() {
            ItemBufferKind::Singleton
        } else {
            ItemBufferKind::None
        }
    }

    fn for_each_project_item(
        &self,
        cx: &App,
        f: &mut dyn FnMut(EntityId, &dyn project::ProjectItem),
    ) {
        if let Some(item) = &self.project_item {
            f(item.entity_id(), item.read(cx));
        }
    }

    fn pane_tab_bar_controls(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<PaneTabBarControls> {
        let entity = cx.entity();

        Some(PaneTabBarControls::new(
            Some(self.render_tab_bar_start_controls(cx)),
            Some(self.render_tab_bar_end_controls(entity, cx)),
        ))
    }

    fn on_tab_click(
        &mut self,
        params: TabContentParams,
        event: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !params.selected || event.click_count() != 1 {
            return false;
        }

        let editor_accepts_input = self.url_editor_focus_handle.is_focused(window)
            && !self.native_preview_has_keyboard_focus(window);
        if !editor_accepts_input {
            self.activate_url_editor(window, cx);
        }
        true
    }

    fn on_tab_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.url_editor_focus_handle.is_focused(window) {
            return false;
        }

        self.confirm_navigation(&Confirm, window, cx);
        true
    }

    fn can_split(&self) -> bool {
        true
    }

    fn clone_on_split(
        &self,
        _workspace_id: Option<WorkspaceId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<Option<Entity<Self>>> {
        let workspace = self.workspace.clone();
        let workspace_context = self.workspace_context.clone();
        let current_url = self.current_url_text(cx);
        let detected_extensions = self.detected_extensions.clone();
        let bookmarks = self.bookmarks.clone();

        Task::ready(Some(cx.new(|cx| {
            let url_editor = cx.new(|cx| {
                let mut editor = Editor::single_line(window, cx);
                editor.set_placeholder_text("Search Google or enter a URL", window, cx);
                editor.set_text(current_url.as_str(), window, cx);
                editor
            });
            let url_editor_focus_handle = url_editor.focus_handle(cx);

            let browser_events = Arc::new(Mutex::new(Vec::new()));
            let mut this = Self {
                workspace,
                workspace_context,
                session_id: format!("web-preview-{}", cx.entity_id().as_non_zero_u64()).into(),
                focus_handle: cx.focus_handle(),
                project_item: None,
                url_editor,
                url_editor_focus_handle,
                url_editor_focus_requested: Rc::new(Cell::new(false)),
                page_title: None,
                active_url: current_url.clone().into(),
                bookmarks,
                detected_extensions,
                extensions_scanned: self.extensions_scanned,
                load_state: PreviewLoadState::Loading,
                layout_bounds: Rc::new(RefCell::new(None)),
                host_bounds: Rc::new(RefCell::new(None)),
                #[cfg(target_os = "macos")]
                last_applied_bounds: Rc::new(RefCell::new(None)),
                native_mount_requested: Rc::new(Cell::new(false)),
                browser_events,
                deferred_ipc_messages: Vec::new(),
                ipc_flush_scheduled: false,
                latest_page_diagnostics: None,
                latest_runtime_events: None,
                latest_dom_snapshot: None,
                latest_action_targets: None,
                latest_readiness_probe: None,
                latest_wait_contract: None,
                latest_interaction_plan: None,
                latest_interaction_preflight: None,
                latest_interaction_receipt_template: None,
                latest_interaction_action_request: None,
                latest_blocked_interaction_receipt: None,
                latest_successful_interaction_receipt: None,
                latest_agent_browser_status_packet: None,
                latest_agent_browser_executor_readiness: None,
                latest_agent_browser_noop_executor_attempt: None,
                latest_agent_browser_reload_executor_attempt: None,
                latest_agent_browser_clear_data_executor_attempt: None,
                latest_agent_browser_qa_runbook: None,
                latest_agent_plugin_catalog: None,
                latest_annotated_screenshot: None,
                event_pump_task: None,
                native_mount_task: None,
                zoom_factor: 1.0,
                viewport_mode: self.viewport_mode,
                agent_action_permission: self.agent_action_permission,
                is_active_item: false,
                #[cfg(any(target_os = "windows", target_os = "macos"))]
                native_preview: Rc::new(RefCell::new(None)),
                _subscriptions: vec![],
            };
            this.start_event_pump(window, cx);
            this
        })))
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(ItemEvent)) {
        f(*event);
    }

    fn added_to_workspace(
        &mut self,
        _workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._subscriptions.push(cx.on_focus_out(
            &self.focus_handle(cx),
            window,
            |this, _, _window, _cx| {
                this.release_native_preview_focus();
            },
        ));
        #[cfg(target_os = "windows")]
        self._subscriptions
            .push(cx.observe_window_activation(window, |this, window, _cx| {
                this.sync_native_preview_window_activation(window);
            }));

        let url_editor_focus = self.url_editor_focus_handle.clone();
        self._subscriptions
            .push(cx.on_focus(&url_editor_focus, window, |this, _window, cx| {
                this.url_editor_focus_requested.set(true);
                this.release_native_preview_focus();
                cx.emit(ItemEvent::UpdateTab);
                cx.notify();
            }));
        self._subscriptions.push(
            cx.on_focus_out(&url_editor_focus, window, |this, _, _, cx| {
                this.url_editor_focus_requested.set(false);
                cx.emit(ItemEvent::UpdateTab);
                cx.notify();
            }),
        );

        #[cfg(target_os = "windows")]
        cx.defer_in(window, |this, window, cx| {
            this.ensure_native_preview(window, cx);
            this.sync_native_preview_window_activation(window);
        });

        let focus_handle = self.focus_handle(cx);
        cx.defer_in(window, move |_, window, cx| {
            focus_handle.focus(window, cx);
        });
    }

    fn deactivated(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.is_active_item = false;
        self.url_editor_focus_requested.set(false);
        // Hide webview when tab is deactivated
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        if let Some(preview) = self.native_preview.borrow_mut().as_mut() {
            let _ = preview.set_visible(false);
        }
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        _window.set_background_appearance(gpui::WindowBackgroundAppearance::Opaque);
    }

    fn workspace_deactivated(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.is_active_item = false;
        self.url_editor_focus_requested.set(false);
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        if let Some(preview) = self.native_preview.borrow_mut().as_mut() {
            let _ = preview.set_visible(false);
        }
        let _ = (_window, _cx);
    }
}

impl Render for WebPreviewView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.is_active_item = true;
        let pending_events = {
            let mut events = self
                .browser_events
                .lock()
                .expect("browser event queue lock poisoned");
            std::mem::take(&mut *events)
        };
        if !pending_events.is_empty() {
            self.apply_browser_events(pending_events, window, cx);
        }

        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            window.set_background_appearance(gpui::WindowBackgroundAppearance::Transparent);
            self.ensure_native_preview(window, cx);
            #[cfg(target_os = "windows")]
            self.sync_native_preview_window_activation(window);
        }

        let body = self.render_webview_body(cx);
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let preview_ready = self.native_preview.borrow().is_some();
        let error_message = match &self.load_state {
            PreviewLoadState::Loading => None,
            PreviewLoadState::Ready => None,
            PreviewLoadState::Error(error) => Some(error.clone()),
        };
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let show_loading_placeholder = error_message.is_none()
            && (!preview_ready || matches!(self.load_state, PreviewLoadState::Loading));
        #[cfg(target_os = "windows")]
        let preview_surface_background = if show_loading_placeholder {
            cx.theme().colors().surface_background
        } else {
            gpui::transparent_black().alpha(1.0 / 255.0)
        };
        #[cfg(not(target_os = "windows"))]
        let preview_surface_background = gpui::transparent_black();
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let loading_placeholder = show_loading_placeholder.then(|| {
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(cx.theme().colors().surface_background.alpha(0.94))
                .child(
                    h_flex()
                        .items_center()
                        .gap_1p5()
                        .px_3()
                        .py_2()
                        .rounded_xl()
                        .border_1()
                        .border_color(cx.theme().colors().border_variant)
                        .bg(cx.theme().colors().surface_background)
                        .child(
                            ui::Icon::new(IconName::LoadCircle)
                                .size(IconSize::Small)
                                .color(Color::Muted),
                        )
                        .child(
                            Label::new("Loading Web Preview")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        ),
                )
        });

        div()
            .id("web-preview")
            .key_context("WebPreview")
            .on_action(cx.listener(Self::confirm_navigation))
            .track_focus(&self.focus_handle(cx))
            .size_full()
            .overflow_hidden()
            .child(
                v_flex().size_full().child(
                    div()
                        .relative()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .overflow_hidden()
                        // Keep the Windows top GPUI window hit-testable over the underlay webview
                        // without visibly obscuring the page.
                        .bg(preview_surface_background)
                        .child(body)
                        .when_some(loading_placeholder, |this, placeholder| {
                            this.child(placeholder)
                        })
                        .when_some(error_message, |this, error| {
                            this.child(
                                div()
                                    .absolute()
                                    .top_3()
                                    .left_3()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(Color::Error.color(cx).alpha(0.14))
                                    .child(
                                        Label::new(error)
                                            .size(LabelSize::Small)
                                            .color(Color::Error),
                                    ),
                            )
                        }),
                ),
            )
    }
}

pub(crate) fn push_browser_event(event_queue: &Arc<Mutex<Vec<BrowserEvent>>>, event: BrowserEvent) {
    let mut queue = event_queue
        .lock()
        .expect("browser event queue lock poisoned");
    queue.push(event);
}

#[cfg(target_os = "windows")]
fn mount_native_preview(request: NativePreviewMountRequest) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        create_native_preview_for_request(&request)
    }));
    request.native_mount_requested.set(false);

    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => push_browser_event(
            &request.browser_events,
            BrowserEvent::MountFailed(error.to_string()),
        ),
        Err(_) => push_browser_event(
            &request.browser_events,
            BrowserEvent::MountFailed(
                "The native web preview crashed while mounting the embedded browser.".to_string(),
            ),
        ),
    }
}

#[cfg(target_os = "windows")]
fn create_native_preview_for_request(request: &NativePreviewMountRequest) -> Result<()> {
    if request.native_preview.borrow().is_some() {
        return Ok(());
    }

    fs::create_dir_all(&request.profile_dir)
        .with_context(|| "Failed to prepare the Web Preview profile directory")?;

    let url = normalized_url(&request.initial_url)?;
    let main_window = request.parent_window.as_hwnd();
    let initial_bounds = request
        .host_bounds
        .borrow()
        .as_ref()
        .copied()
        .map(|bounds| client_rect_for_bounds(bounds, request.scale_factor))
        .unwrap_or(RECT {
            left: 0,
            top: 0,
            right: 32,
            bottom: 32,
        });

    let webview = WindowsVisualWebView::new(
        main_window,
        request.profile_dir.clone(),
        url.as_str(),
        request.zoom_factor,
        request.scale_factor,
        initial_bounds,
        request.browser_events.clone(),
        request.initially_visible,
    )
    .with_context(|| "Failed to build the embedded web preview")?;

    *request.native_preview.borrow_mut() = Some(NativeWebPreview { webview });

    Ok(())
}

#[cfg(target_os = "macos")]
fn create_native_preview_for_macos_window(
    window: &Window,
    profile_dir: PathBuf,
    initial_url: String,
    zoom_factor: f64,
    host_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    native_preview: Rc<RefCell<Option<NativeWebPreview>>>,
) -> Result<()> {
    if native_preview.borrow().is_some() {
        return Ok(());
    }

    fs::create_dir_all(&profile_dir)
        .with_context(|| "Failed to prepare the Web Preview profile directory")?;

    let url = normalized_url(&initial_url)?;
    let event_queue = browser_events.clone();
    let mut web_context = Box::new(WebContext::new(Some(profile_dir)));

    let initial_bounds = host_bounds
        .borrow()
        .as_ref()
        .copied()
        .map(bounds_to_wry_rect)
        .unwrap_or_else(|| WryRect {
            position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
            size: Size::Logical(LogicalSize::new(32.0, 32.0)),
        });

    let webview = WebViewBuilder::new_with_web_context(web_context.as_mut())
        .with_bounds(initial_bounds)
        .with_url(url.as_str())
        .with_focused(false)
        .with_clipboard(true)
        .with_hotkeys_zoom(false)
        .with_back_forward_navigation_gestures(true)
        .with_default_context_menus(false)
        .with_devtools(true)
        .with_visible(true)
        .with_initialization_script(WEB_PREVIEW_BRIDGE_SCRIPT)
        .with_document_title_changed_handler({
            let event_queue = event_queue.clone();
            move |title| push_browser_event(&event_queue, BrowserEvent::TitleChanged(title))
        })
        .with_on_page_load_handler({
            let event_queue = event_queue.clone();
            move |event, url| {
                if matches!(event, PageLoadEvent::Finished) {
                    push_browser_event(&event_queue, BrowserEvent::UrlChanged(url));
                }
            }
        })
        .with_ipc_handler(move |request| {
            push_browser_event(
                &event_queue,
                BrowserEvent::IpcMessage(request.body().to_string()),
            );
        })
        .build_as_child(window)
        .with_context(|| "Failed to build the embedded web preview")?;

    webview.zoom(zoom_factor)?;

    if let Some(bounds) = *host_bounds.borrow() {
        let _ = set_webview_bounds(&webview, bounds);
    }

    *native_preview.borrow_mut() = Some(NativeWebPreview {
        _context: web_context,
        webview,
    });

    Ok(())
}

fn parse_browser_rect(value: &Value) -> Option<BrowserRect> {
    Some(BrowserRect {
        x: value.get("x")?.as_f64()?,
        y: value.get("y")?.as_f64()?,
        width: value.get("width")?.as_f64()?,
        height: value.get("height")?.as_f64()?,
    })
}

fn viewport_bounds_for_layout(
    layout_bounds: Bounds<Pixels>,
    viewport_mode: PreviewViewportMode,
) -> Bounds<Pixels> {
    let Some((desired_width, desired_height)) = viewport_mode.dimensions() else {
        return layout_bounds;
    };

    let available_width = layout_bounds.size.width.as_f32().max(1.0);
    let available_height = layout_bounds.size.height.as_f32().max(1.0);
    let width = (desired_width as f32).min(available_width).max(1.0);
    let height = (desired_height as f32).min(available_height).max(1.0);
    let offset_x = ((available_width - width) / 2.0).max(0.0);
    let offset_y = ((available_height - height) / 2.0).max(0.0);

    Bounds::new(
        layout_bounds.origin + point(px(offset_x), px(offset_y)),
        size(px(width), px(height)),
    )
}

fn format_agent_note(payload: &BrowserAgentPayload) -> String {
    match payload {
        BrowserAgentPayload::InspectElement {
            url,
            title,
            selector,
            tag,
            id,
            classes,
            text,
            href,
            src,
            rect,
            css,
            html,
        } => {
            let mut message = format!("Web Preview inspected element.\nURL: {url}");
            if let Some(title) = title.as_deref().filter(|title| !title.is_empty()) {
                message.push_str(&format!("\nPage title: {title}"));
            }
            if let Some(selector) = selector.as_deref().filter(|selector| !selector.is_empty()) {
                message.push_str(&format!("\nSelector: {selector}"));
            }
            if let Some(tag) = tag.as_deref().filter(|tag| !tag.is_empty()) {
                message.push_str(&format!("\nTag: {tag}"));
            }
            if let Some(id) = id.as_deref().filter(|id| !id.is_empty()) {
                message.push_str(&format!("\nID: {id}"));
            }
            if !classes.is_empty() {
                message.push_str(&format!("\nClasses: {}", classes.join(" ")));
            }
            if let Some(text) = text.as_deref().filter(|text| !text.is_empty()) {
                message.push_str(&format!("\nText: {text}"));
            }
            if let Some(href) = href.as_deref().filter(|href| !href.is_empty()) {
                message.push_str(&format!("\nHref: {href}"));
            }
            if let Some(src) = src.as_deref().filter(|src| !src.is_empty()) {
                message.push_str(&format!("\nSource: {src}"));
            }
            if let Some(rect) = rect.as_ref() {
                message.push_str(&format!(
                    "\nRect: x={:.1}, y={:.1}, width={:.1}, height={:.1}",
                    rect.x, rect.y, rect.width, rect.height
                ));
            }
            if let Some(css) = css.as_deref().filter(|css| !css.is_empty()) {
                message.push_str("\nCSS snapshot:\n```css\n");
                message.push_str(css);
                message.push_str("\n```");
            }
            if let Some(html) = html.as_deref().filter(|html| !html.is_empty()) {
                message.push_str("\nHTML snippet:\n```html\n");
                message.push_str(html);
                message.push_str("\n```");
            }
            message
        }
    }
}

fn display_title_from_url(url: &str) -> String {
    if url.eq_ignore_ascii_case("about:blank") {
        return "Preview".to_string();
    }

    url::Url::parse(url)
        .ok()
        .and_then(|url| {
            url.host_str()
                .map(|host| host.to_string())
                .or_else(|| Some(url.path().to_string()))
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Preview".to_string())
}

fn preview_file_kind_for_path(path: &Path) -> Option<PreviewFileKind> {
    let extension = path.extension()?.to_str()?.to_lowercase();
    match extension.as_str() {
        "mp4" | "webm" | "mov" | "m4v" | "avi" | "mkv" => Some(PreviewFileKind::Video),
        "mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" | "opus" => Some(PreviewFileKind::Audio),
        "pdf" | "epub" | "mobi" | "azw" | "azw3" | "doc" | "docx" | "docm" | "dot" | "dotx"
        | "dotm" | "odt" | "ott" | "rtf" | "pages" | "xls" | "xlsx" | "xlsm" | "xlsb" | "xlt"
        | "xltx" | "ods" | "ots" | "numbers" | "csv" | "tsv" | "ppt" | "pptx" | "pptm" | "pps"
        | "ppsx" | "odp" | "otp" | "key" => Some(PreviewFileKind::Document),
        _ => None,
    }
}

fn web_preview_file_url(path: &Path, title: &str, kind: PreviewFileKind) -> Option<String> {
    let source_url = url::Url::from_file_path(path).ok()?.to_string();
    let preview_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("G:/Zed"))
        .join("target")
        .join("web-preview-files");
    fs::create_dir_all(&preview_dir).ok()?;
    let preview_stem = {
        let stem = slugify(title);
        if stem.is_empty() {
            "preview".to_string()
        } else {
            stem
        }
    };
    let preview_path = preview_dir.join(format!("{}-{}.html", kind.label(), preview_stem));
    fs::write(
        &preview_path,
        web_preview_file_html(title, kind, &source_url),
    )
    .ok()?;
    url::Url::from_file_path(preview_path)
        .ok()
        .map(|url| url.to_string())
}

fn web_preview_file_html(title: &str, kind: PreviewFileKind, source_url: &str) -> String {
    let title = escape_html(title);
    let source = escape_attr(source_url);
    let media = match kind {
        PreviewFileKind::Video => {
            format!(r#"<video class="viewer-media" src="{source}" controls autoplay></video>"#)
        }
        PreviewFileKind::Audio => {
            format!(
                r#"<div class="audio-shell"><div class="audio-disc"></div><h1>{title}</h1><audio class="viewer-audio" src="{source}" controls autoplay></audio></div>"#
            )
        }
        PreviewFileKind::Document => {
            format!(
                r#"<iframe class="viewer-doc" src="{source}" title="{title}"></iframe><a class="doc-open" href="{source}">Open source</a>"#
            )
        }
    };

    format!(
        r#"<!doctype html>
<html lang="en" class="dark">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title} - Zed preview</title>
  <style>
    :root {{
      color-scheme: dark;
      --background: #09090b;
      --foreground: #f4f4f5;
      --card: #101113;
      --border: #27272a;
      --accent: #3fb950;
      --ring: rgba(63, 185, 80, .38);
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
      overflow: hidden;
      display: grid;
      place-items: center;
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
      gap: 22px;
      justify-items: center;
      padding: 40px;
      border: 1px solid var(--border);
      border-radius: 8px;
      background: color-mix(in srgb, var(--card) 92%, transparent);
      box-shadow: 0 24px 80px rgba(0, 0, 0, .34);
    }}
    .audio-disc {{
      width: 128px;
      aspect-ratio: 1;
      border-radius: 999px;
      background:
        radial-gradient(circle at center, var(--background) 0 17%, transparent 18%),
        conic-gradient(from 130deg, var(--accent), #6ee7b7, #64748b, var(--accent));
      box-shadow: 0 0 0 1px var(--border), 0 0 60px var(--ring);
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
    .viewer-doc {{
      width: 100vw;
      height: 100vh;
      border: 0;
      background: var(--card);
    }}
    .doc-open {{
      position: absolute;
      right: 16px;
      bottom: 16px;
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 7px 10px;
      background: color-mix(in srgb, var(--card) 94%, transparent);
      color: var(--accent);
      text-decoration: none;
    }}
  </style>
</head>
<body>
  <section class="stage">{media}</section>
</body>
</html>"#,
    )
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

fn normalized_url(raw: &str) -> Result<url::Url> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return url::Url::parse(DEFAULT_WEB_PREVIEW_URL).map_err(Into::into);
    }

    if let Ok(url) = url::Url::parse(trimmed) {
        return Ok(url);
    }

    if trimmed.contains('.') && !trimmed.contains(' ') {
        return url::Url::parse(&format!("https://{trimmed}"))
            .or_else(|_| url::Url::parse(&format!("http://{trimmed}")))
            .map_err(Into::into);
    }

    let mut url = url::Url::parse(GOOGLE_SEARCH_URL)?;
    url.query_pairs_mut().append_pair("q", trimmed);
    Ok(url)
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn load_bookmarks(profile_dir: &Path) -> Result<Vec<String>> {
    let path = profile_dir.join(BOOKMARKS_FILE_NAME);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let data = fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_slice(&data).with_context(|| format!("Failed to parse {}", path.display()))
}

fn scan_local_extensions() -> Result<Vec<DetectedExtension>> {
    let mut extensions = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let local_app_data = PathBuf::from(local_app_data);
            let sources = [
                (
                    "Chrome",
                    local_app_data
                        .join("Google")
                        .join("Chrome")
                        .join("User Data")
                        .join("Default")
                        .join("Extensions"),
                    true,
                ),
                (
                    "Edge",
                    local_app_data
                        .join("Microsoft")
                        .join("Edge")
                        .join("User Data")
                        .join("Default")
                        .join("Extensions"),
                    true,
                ),
                (
                    "Brave",
                    local_app_data
                        .join("BraveSoftware")
                        .join("Brave-Browser")
                        .join("User Data")
                        .join("Default")
                        .join("Extensions"),
                    true,
                ),
                (
                    "Firefox",
                    local_app_data
                        .join("Mozilla")
                        .join("Firefox")
                        .join("Profiles"),
                    false,
                ),
            ];

            for (browser, path, chromium_compatible) in sources {
                if browser == "Firefox" {
                    scan_firefox_extensions(browser, &path, &mut extensions)?;
                    continue;
                }

                scan_chromium_extensions(browser, &path, chromium_compatible, &mut extensions)?;
            }
        }
    }

    extensions.sort_by(|left, right| {
        left.browser
            .cmp(&right.browser)
            .then_with(|| left.name.cmp(&right.name))
    });
    extensions.dedup_by(|left, right| left.browser == right.browser && left.path == right.path);
    Ok(extensions)
}

#[cfg(target_os = "windows")]
fn scan_chromium_extensions(
    browser: &str,
    root: &Path,
    chromium_compatible: bool,
    extensions: &mut Vec<DetectedExtension>,
) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }

    for extension_dir in fs::read_dir(root)? {
        let extension_dir = extension_dir?;
        let extension_path = extension_dir.path();
        if !extension_path.is_dir() {
            continue;
        }

        let Some(extension_id) = extension_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        let mut versions = fs::read_dir(&extension_path)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        versions.sort();

        let Some(version_dir) = versions.pop() else {
            continue;
        };
        let manifest_path = version_dir.join("manifest.json");
        if !manifest_path.is_file() {
            continue;
        }

        let manifest: Value = serde_json::from_slice(
            &fs::read(&manifest_path)
                .with_context(|| format!("Failed to read {}", manifest_path.display()))?,
        )
        .with_context(|| format!("Failed to parse {}", manifest_path.display()))?;

        let name = manifest
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .unwrap_or(extension_id);

        let icon_path = manifest
            .get("icons")
            .and_then(Value::as_object)
            .and_then(|icons| {
                icons
                    .iter()
                    .filter_map(|(size, path)| Some((size.parse::<u32>().ok()?, path.as_str()?)))
                    .max_by_key(|(size, _)| *size)
                    .map(|(_, path)| version_dir.join(path))
            })
            .filter(|path| path.is_file());

        extensions.push(DetectedExtension {
            browser: browser.to_string().into(),
            id: extension_id.to_string().into(),
            name: name.to_string().into(),
            path: version_dir,
            icon_path,
            supports_chromium_loading: chromium_compatible,
        });
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn scan_firefox_extensions(
    browser: &str,
    root: &Path,
    extensions: &mut Vec<DetectedExtension>,
) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }

    for profile_dir in fs::read_dir(root)? {
        let profile_dir = profile_dir?;
        let extensions_dir = profile_dir.path().join("extensions");
        if !extensions_dir.is_dir() {
            continue;
        }

        for entry in fs::read_dir(&extensions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() && !path.is_dir() {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Firefox Extension");
            extensions.push(DetectedExtension {
                browser: browser.to_string().into(),
                id: slugify(name).into(),
                name: name.to_string().into(),
                path,
                icon_path: None,
                supports_chromium_loading: false,
            });
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn set_webview_bounds(webview: &WebView, bounds: Bounds<Pixels>) -> Result<()> {
    let rect = bounds_to_wry_rect(bounds);
    webview.set_bounds(rect)?;
    // Webview stays visible - no hiding
    Ok(())
}

#[cfg(target_os = "windows")]
fn client_rect_for_bounds(bounds: Bounds<Pixels>, scale_factor: f32) -> RECT {
    RECT {
        left: (bounds.origin.x.as_f32() * scale_factor).floor() as i32,
        top: (bounds.origin.y.as_f32() * scale_factor).floor() as i32,
        right: ((bounds.origin.x.as_f32() + bounds.size.width.as_f32()) * scale_factor).ceil()
            as i32,
        bottom: ((bounds.origin.y.as_f32() + bounds.size.height.as_f32()) * scale_factor).ceil()
            as i32,
    }
}

#[cfg(target_os = "windows")]
fn screen_rect_for_hwnd(hwnd: HWND, scale: f32, bounds: Bounds<Pixels>) -> Result<RECT> {
    let mut client_origin = POINT { x: 0, y: 0 };
    unsafe {
        if !ClientToScreen(hwnd, &mut client_origin).as_bool() {
            return Err(anyhow!(
                "Failed to translate the web preview bounds to screen coordinates"
            ));
        }
    }

    let left = client_origin.x + (f32::from(bounds.origin.x) * scale).round() as i32;
    let top = client_origin.y + (f32::from(bounds.origin.y) * scale).round() as i32;
    let width = (f32::from(bounds.size.width) * scale).round().max(1.0) as i32;
    let height = (f32::from(bounds.size.height) * scale).round().max(1.0) as i32;

    Ok(RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    })
}

#[cfg(target_os = "windows")]
fn screen_rect_for_bounds(window: &Window, bounds: Bounds<Pixels>) -> Result<RECT> {
    let handle = raw_window_handle::HasWindowHandle::window_handle(window)?;
    let RawWindowHandle::Win32(raw_handle) = handle.as_raw() else {
        return Err(anyhow!(
            "Unsupported window handle for web preview screenshots"
        ));
    };
    screen_rect_for_hwnd(
        HWND(raw_handle.hwnd.get() as *mut _),
        window.scale_factor(),
        bounds,
    )
}

#[cfg(target_os = "windows")]
fn capture_screen_rect(rect: RECT) -> Result<RgbaImage> {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        return Err(anyhow!("Screenshot bounds are empty"));
    }

    unsafe {
        let screen_dc = GetDC(None);
        if screen_dc.0.is_null() {
            return Err(anyhow!("Failed to acquire the screen device context"));
        }

        let memory_dc = CreateCompatibleDC(Some(screen_dc));
        if memory_dc.0.is_null() {
            ReleaseDC(None, screen_dc);
            return Err(anyhow!("Failed to create a compatible device context"));
        }

        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.0.is_null() {
            let _ = DeleteDC(memory_dc);
            let _ = ReleaseDC(None, screen_dc);
            return Err(anyhow!("Failed to create a compatible bitmap"));
        }

        let previous = SelectObject(memory_dc, HGDIOBJ(bitmap.0));
        let blit_result = BitBlt(
            memory_dc,
            0,
            0,
            width,
            height,
            Some(screen_dc),
            rect.left,
            rect.top,
            SRCCOPY | CAPTUREBLT,
        );
        let _ = SelectObject(memory_dc, previous);

        if blit_result.is_err() {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(memory_dc);
            let _ = ReleaseDC(None, screen_dc);
            return Err(anyhow!(
                "Failed to copy the web preview surface into a bitmap"
            ));
        }

        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bytes = vec![0u8; (width as usize) * (height as usize) * 4];
        let rows = GetDIBits(
            memory_dc,
            HBITMAP(bitmap.0),
            0,
            height as u32,
            Some(bytes.as_mut_ptr().cast()),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        );

        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(memory_dc);
        let _ = ReleaseDC(None, screen_dc);

        if rows == 0 {
            return Err(anyhow!("Failed to read the captured bitmap data"));
        }

        for pixel in bytes.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }

        RgbaImage::from_raw(width as u32, height as u32, bytes)
            .ok_or_else(|| anyhow!("Failed to construct the screenshot image buffer"))
    }
}

#[cfg(target_os = "windows")]
fn crop_image(image: RgbaImage, rect: BrowserRect) -> Result<RgbaImage> {
    let image_width = image.width();
    let image_height = image.height();
    let x = rect.x.max(0.0).round() as u32;
    let y = rect.y.max(0.0).round() as u32;
    let width = rect.width.max(1.0).round() as u32;
    let height = rect.height.max(1.0).round() as u32;

    if x >= image_width || y >= image_height {
        return Err(anyhow!(
            "The selected capture area is outside the visible page"
        ));
    }

    let width = width.min(image_width.saturating_sub(x));
    let height = height.min(image_height.saturating_sub(y));
    Ok(imageops::crop_imm(&image, x, y, width, height).to_image())
}

pub(crate) const WEB_PREVIEW_BRIDGE_SCRIPT: &str = r#"
(() => {
  if (window.__zedWebPreview) return;

  const post = (payload) => {
    try {
      window.ipc.postMessage(JSON.stringify(payload));
    } catch (_error) {}
  };

  const limitText = (value, max) => {
    if (!value) return null;
    const text = String(value).trim();
    return text ? text.slice(0, max) : null;
  };

  const cssSelector = (element) => {
    if (!(element instanceof Element)) return null;
    if (element.id) return `#${element.id}`;
    const parts = [];
    let node = element;
    while (node && node.nodeType === Node.ELEMENT_NODE && parts.length < 6) {
      let selector = node.tagName.toLowerCase();
      if (node.classList.length) {
        selector += Array.from(node.classList).slice(0, 3).map(cls => `.${cls}`).join("");
      }
      const parent = node.parentElement;
      if (parent) {
        const siblings = Array.from(parent.children).filter(child => child.tagName === node.tagName);
        if (siblings.length > 1) {
          selector += `:nth-of-type(${siblings.indexOf(node) + 1})`;
        }
      }
      parts.unshift(selector);
      node = parent;
    }
    return parts.join(" > ");
  };

  const styleSnapshot = (element) => {
    if (!(element instanceof Element)) return null;
    const computed = window.getComputedStyle(element);
    const properties = [
      "display",
      "position",
      "z-index",
      "width",
      "height",
      "min-width",
      "min-height",
      "max-width",
      "max-height",
      "margin",
      "padding",
      "gap",
      "align-items",
      "justify-content",
      "flex",
      "flex-direction",
      "grid-template-columns",
      "grid-template-rows",
      "color",
      "background-color",
      "font-family",
      "font-size",
      "font-weight",
      "line-height",
      "text-align",
      "border",
      "border-radius",
      "box-shadow",
      "opacity",
      "overflow",
      "transform"
    ];

    const lines = properties
      .map((property) => {
        const value = computed.getPropertyValue(property)?.trim();
        return value ? `${property}: ${value};` : null;
      })
      .filter(Boolean);

    return lines.length ? lines.join("\n") : null;
  };

  const createOverlay = () => {
    const overlay = document.createElement("div");
    overlay.style.position = "fixed";
    overlay.style.inset = "0";
    overlay.style.zIndex = "2147483647";
    overlay.style.cursor = "crosshair";
    overlay.style.pointerEvents = "none";
    overlay.style.background = "rgba(0, 0, 0, 0.02)";
    return overlay;
  };

  const elementSnapshot = (element) => {
    if (!(element instanceof Element)) return null;
    const rect = element.getBoundingClientRect();
    return {
      selector: cssSelector(element),
      tag: element.tagName.toLowerCase(),
      id: element.id || null,
      classes: Array.from(element.classList || []).slice(0, 8),
      name: element.getAttribute("name"),
      type: element.getAttribute("type"),
      role: element.getAttribute("role"),
      ariaLabel: element.getAttribute("aria-label"),
      text: limitText(element.innerText || element.textContent || element.getAttribute("alt"), 240),
      href: element.getAttribute("href"),
      src: element.getAttribute("src"),
      disabled: Boolean(element.disabled),
      rect: {
        x: Math.round(rect.x),
        y: Math.round(rect.y),
        width: Math.round(rect.width),
        height: Math.round(rect.height)
      }
    };
  };

  const sampleElements = (selector, limit) => {
    try {
      return Array.from(document.querySelectorAll(selector)).slice(0, limit).map(elementSnapshot).filter(Boolean);
    } catch (_error) {
      return [];
    }
  };

  const collectForms = () => Array.from(document.forms || []).slice(0, 12).map((form) => ({
    selector: cssSelector(form),
    action: form.getAttribute("action"),
    method: form.getAttribute("method") || "get",
    controls: Array.from(form.elements || []).slice(0, 24).map((control) => elementSnapshot(control)).filter(Boolean)
  }));

  const collectPerformance = () => {
    const navigation = performance.getEntriesByType ? performance.getEntriesByType("navigation")[0] : null;
    const resources = performance.getEntriesByType ? performance.getEntriesByType("resource") : [];
    return {
      navigation: navigation ? {
        type: navigation.type,
        domContentLoaded: Math.round(navigation.domContentLoadedEventEnd || 0),
        loadEventEnd: Math.round(navigation.loadEventEnd || 0),
        transferSize: navigation.transferSize || 0,
        encodedBodySize: navigation.encodedBodySize || 0,
        decodedBodySize: navigation.decodedBodySize || 0
      } : null,
      slowResources: Array.from(resources)
        .filter((entry) => entry.duration > 250)
        .sort((a, b) => b.duration - a.duration)
        .slice(0, 24)
        .map((entry) => ({
          name: limitText(entry.name, 240),
          initiatorType: entry.initiatorType,
          duration: Math.round(entry.duration),
          transferSize: entry.transferSize || 0
      }))
    };
  };

  const MAX_RUNTIME_EVENTS = 200;
  const runtimeEvents = {
    console: [],
    network: []
  };
  const runtimeState = {
    pendingNetwork: 0
  };

  const beginNetworkRequest = () => {
    runtimeState.pendingNetwork += 1;
  };

  const finishNetworkRequest = () => {
    runtimeState.pendingNetwork = Math.max(0, runtimeState.pendingNetwork - 1);
  };

  const pushRuntimeEvent = (bucket, event) => {
    const events = runtimeEvents[bucket];
    if (!events) return;
    events.push({
      timestamp: new Date().toISOString(),
      ...event
    });
    if (events.length > MAX_RUNTIME_EVENTS) {
      events.splice(0, events.length - MAX_RUNTIME_EVENTS);
    }
  };

  const requestUrl = (input) => {
    try {
      if (typeof input === "string") return input;
      if (input instanceof URL) return input.href;
      if (input && typeof input.url === "string") return input.url;
      return input == null ? null : String(input);
    } catch (_error) {
      return null;
    }
  };

  const requestMethod = (input, init) => {
    try {
      return String(init?.method || input?.method || "GET").toUpperCase();
    } catch (_error) {
      return "GET";
    }
  };

  const serializeRuntimeValue = (value) => {
    try {
      if (value == null) return value;
      if (typeof value === "string") return limitText(value, 1200);
      if (typeof value === "number" || typeof value === "boolean") return value;
      if (typeof value === "bigint") return `${value.toString()}n`;
      if (typeof value === "symbol") return value.toString();
      if (typeof value === "function") return `[Function ${value.name || "anonymous"}]`;
      if (value instanceof Error) {
        return {
          name: value.name,
          message: limitText(value.message, 1200),
          stack: limitText(value.stack, 4000)
        };
      }
      if (value instanceof Element) return elementSnapshot(value);
      const seen = new WeakSet();
      const json = JSON.stringify(value, (_key, nestedValue) => {
        if (typeof nestedValue === "bigint") return `${nestedValue.toString()}n`;
        if (typeof nestedValue === "function") return `[Function ${nestedValue.name || "anonymous"}]`;
        if (nestedValue instanceof Element) return elementSnapshot(nestedValue);
        if (nestedValue && typeof nestedValue === "object") {
          if (seen.has(nestedValue)) return "[Circular]";
          seen.add(nestedValue);
        }
        return nestedValue;
      });
      if (!json) return String(value);
      return json.length > 5000 ? `${json.slice(0, 5000)}...` : JSON.parse(json);
    } catch (_error) {
      try {
        return limitText(String(value), 1200);
      } catch (_innerError) {
        return "[Unserializable]";
      }
    }
  };

  const installConsoleCapture = () => {
    if (!window.console || window.console.__zedWebPreviewCaptured) return;

    const levels = ["debug", "log", "info", "warn", "error"];
    for (const level of levels) {
      const original = typeof window.console[level] === "function"
        ? window.console[level].bind(window.console)
        : null;
      window.console[level] = (...args) => {
        pushRuntimeEvent("console", {
          level,
          text: limitText(args.map((arg) => {
            if (typeof arg === "string") return arg;
            try {
              return JSON.stringify(serializeRuntimeValue(arg));
            } catch (_error) {
              return String(arg);
            }
          }).join(" "), 2400),
          args: args.slice(0, 12).map(serializeRuntimeValue)
        });
        if (original) {
          return original(...args);
        }
      };
    }

    Object.defineProperty(window.console, "__zedWebPreviewCaptured", {
      value: true,
      configurable: false
    });
  };

  const installPageErrorCapture = () => {
    window.addEventListener("error", (event) => {
      pushRuntimeEvent("console", {
        level: "error",
        source: "window.error",
        text: limitText(event.message, 2400),
        filename: event.filename || null,
        line: event.lineno || null,
        column: event.colno || null,
        error: serializeRuntimeValue(event.error)
      });
    }, true);

    window.addEventListener("unhandledrejection", (event) => {
      pushRuntimeEvent("console", {
        level: "error",
        source: "unhandledrejection",
        text: limitText(event.reason?.message || String(event.reason), 2400),
        error: serializeRuntimeValue(event.reason)
      });
    }, true);
  };

  const installFetchCapture = () => {
    if (typeof window.fetch !== "function" || window.fetch.__zedWebPreviewCaptured) return;

    const nativeFetch = window.fetch.bind(window);
    const capturedFetch = async (input, init) => {
      const startedAt = performance.now();
      const url = requestUrl(input);
      const method = requestMethod(input, init);
      beginNetworkRequest();
      try {
        const response = await nativeFetch(input, init);
        pushRuntimeEvent("network", {
          kind: "fetch",
          method,
          url: response.url || url,
          status: response.status,
          statusText: response.statusText,
          ok: response.ok,
          duration: Math.round(performance.now() - startedAt)
        });
        return response;
      } catch (error) {
        pushRuntimeEvent("network", {
          kind: "fetch",
          method,
          url,
          status: null,
          ok: false,
          error: serializeRuntimeValue(error),
          duration: Math.round(performance.now() - startedAt)
        });
        throw error;
      } finally {
        finishNetworkRequest();
      }
    };

    Object.defineProperty(capturedFetch, "__zedWebPreviewCaptured", {
      value: true,
      configurable: false
    });
    window.fetch = capturedFetch;
  };

  const installXhrCapture = () => {
    if (!window.XMLHttpRequest?.prototype || window.XMLHttpRequest.prototype.__zedWebPreviewCaptured) return;

    const prototype = window.XMLHttpRequest.prototype;
    const nativeOpen = prototype.open;
    const nativeSend = prototype.send;

    prototype.open = function(method, url, ...rest) {
      this.__zedWebPreviewRequest = {
        method: String(method || "GET").toUpperCase(),
        url: requestUrl(url)
      };
      return nativeOpen.call(this, method, url, ...rest);
    };

    prototype.send = function(...args) {
      const request = this.__zedWebPreviewRequest || {};
      const startedAt = performance.now();
      let completed = false;
      const complete = () => {
        if (completed) return;
        completed = true;
        finishNetworkRequest();
        pushRuntimeEvent("network", {
          kind: "xhr",
          method: request.method || "GET",
          url: request.url || this.responseURL || null,
          status: this.status || null,
          statusText: this.statusText || null,
          ok: this.status >= 200 && this.status < 400,
          duration: Math.round(performance.now() - startedAt)
        });
      };
      this.addEventListener("loadend", complete, { once: true });
      beginNetworkRequest();
      try {
        return nativeSend.apply(this, args);
      } catch (error) {
        if (!completed) {
          completed = true;
          finishNetworkRequest();
        }
        pushRuntimeEvent("network", {
          kind: "xhr",
          method: request.method || "GET",
          url: request.url || null,
          status: null,
          ok: false,
          error: serializeRuntimeValue(error),
          duration: Math.round(performance.now() - startedAt)
        });
        throw error;
      }
    };

    Object.defineProperty(prototype, "__zedWebPreviewCaptured", {
      value: true,
      configurable: false
    });
  };

  const mutationState = {
    observerAvailable: typeof window.MutationObserver === "function",
    observed: false,
    count: 0,
    lastMutationAt: null,
    lastMutationMs: null,
    observer: null
  };

  const noteMutation = (count = 1) => {
    mutationState.count += count;
    mutationState.lastMutationAt = new Date().toISOString();
    mutationState.lastMutationMs = performance.now();
  };

  const installMutationCapture = () => {
    if (!mutationState.observerAvailable || mutationState.observed) return;
    const root = document.documentElement || document.body;
    if (!root) {
      document.addEventListener("DOMContentLoaded", () => {
        try { installMutationCapture(); } catch (_error) {}
      }, { once: true });
      return;
    }
    const observer = new MutationObserver((mutations) => {
      noteMutation(mutations.length || 1);
    });
    observer.observe(root, {
      subtree: true,
      childList: true,
      attributes: true,
      characterData: true
    });
    mutationState.observed = true;
    mutationState.observer = observer;
  };

  const installRuntimeCapture = () => {
    try { installConsoleCapture(); } catch (_error) {}
    try { installPageErrorCapture(); } catch (_error) {}
    try { installFetchCapture(); } catch (_error) {}
    try { installXhrCapture(); } catch (_error) {}
    try { installMutationCapture(); } catch (_error) {}
  };

  const runtimeResourceSnapshot = () => {
    const resources = performance.getEntriesByType ? performance.getEntriesByType("resource") : [];
    return Array.from(resources)
      .slice(-80)
      .map((entry) => ({
        name: limitText(entry.name, 240),
        initiatorType: entry.initiatorType,
        duration: Math.round(entry.duration || 0),
        transferSize: entry.transferSize || 0,
        encodedBodySize: entry.encodedBodySize || 0,
        decodedBodySize: entry.decodedBodySize || 0
      }));
  };

  const MAX_DOM_SNAPSHOT_NODES = 650;
  const MAX_DOM_SNAPSHOT_DEPTH = 5;
  const MAX_DOM_SNAPSHOT_CHILDREN = 24;

  const roundedRect = (element) => {
    try {
      const rect = element.getBoundingClientRect();
      return {
        x: Math.round(rect.x),
        y: Math.round(rect.y),
        width: Math.round(rect.width),
        height: Math.round(rect.height)
      };
    } catch (_error) {
      return null;
    }
  };

  const isVisibleElement = (element, rect) => {
    try {
      if (!rect || rect.width <= 0 || rect.height <= 0) return false;
      const style = window.getComputedStyle(element);
      return style.display !== "none" && style.visibility !== "hidden" && style.opacity !== "0";
    } catch (_error) {
      return rect && rect.width > 0 && rect.height > 0;
    }
  };

  const isInteractiveElement = (element) => {
    try {
      return element.matches("a[href], button, input, textarea, select, option, summary, label, video, audio, [role='button'], [role='link'], [role='menuitem'], [role='tab'], [role='checkbox'], [role='radio'], [role='textbox'], [contenteditable='true']");
    } catch (_error) {
      return false;
    }
  };

  const domAttributeSnapshot = (element) => {
    const attributes = {};
    for (const name of ["id", "class", "role", "aria-label", "name", "type", "placeholder", "title", "alt", "href", "src", "data-testid", "data-test", "data-cy"]) {
      const value = element.getAttribute(name);
      if (value) attributes[name] = limitText(value, 220);
    }
    return attributes;
  };

  const domTreeNode = (node, depth, siblingIndex, budget) => {
    if (!node) return null;
    if (budget.nodes >= MAX_DOM_SNAPSHOT_NODES) {
      budget.truncated += 1;
      return { type: "truncated", reason: "node_budget_exceeded" };
    }

    if (node.nodeType === Node.TEXT_NODE) {
      const text = limitText(node.textContent, 180);
      if (!text) return null;
      budget.nodes += 1;
      return { type: "text", text };
    }

    if (!(node instanceof Element)) return null;

    const tag = node.tagName.toLowerCase();
    if (["script", "style", "noscript", "template"].includes(tag)) {
      return null;
    }

    budget.nodes += 1;
    const rect = roundedRect(node);
    const visible = isVisibleElement(node, rect);
    const childNodes = Array.from(node.childNodes || []);
    const children = [];

    if (depth < MAX_DOM_SNAPSHOT_DEPTH) {
      const childLimit = Math.min(childNodes.length, MAX_DOM_SNAPSHOT_CHILDREN);
      for (let index = 0; index < childLimit; index += 1) {
        const child = domTreeNode(childNodes[index], depth + 1, index, budget);
        if (child) children.push(child);
      }
      if (childNodes.length > MAX_DOM_SNAPSHOT_CHILDREN) {
        budget.truncated += childNodes.length - MAX_DOM_SNAPSHOT_CHILDREN;
      }
    } else if (childNodes.length) {
      budget.truncated += childNodes.length;
    }

    return {
      type: "element",
      tag,
      selector: cssSelector(node),
      sibling_index: siblingIndex,
      attributes: domAttributeSnapshot(node),
      text_preview: limitText(node.textContent, 180),
      rect: visible ? rect : null,
      flags: {
        visible,
        interactive: isInteractiveElement(node),
        disabled: Boolean(node.disabled),
        focused: node === document.activeElement
      },
      child_count: childNodes.length,
      children
    };
  };

  const ACTION_TARGET_SELECTOR = "a[href], button, input, textarea, select, option, summary, label, video, audio, [role='button'], [role='link'], [role='menuitem'], [role='tab'], [role='checkbox'], [role='radio'], [role='textbox'], [contenteditable='true']";
  const MAX_ACTION_TARGETS = 180;

  const actionKindForElement = (element) => {
    try {
      const tag = element.tagName.toLowerCase();
      const role = element.getAttribute("role");
      if (tag === "input" || tag === "textarea" || role === "textbox" || element.getAttribute("contenteditable") === "true") {
        return "type";
      }
      if (tag === "select" || tag === "option" || role === "checkbox" || role === "radio") {
        return "select";
      }
      if (tag === "video" || tag === "audio") {
        return "media";
      }
      return "click";
    } catch (_error) {
      return "click";
    }
  };

  const actionTargetLabel = (element) => {
    const value = element.getAttribute("aria-label")
      || element.getAttribute("title")
      || element.getAttribute("placeholder")
      || element.getAttribute("alt")
      || element.getAttribute("name")
      || element.getAttribute("value")
      || element.innerText
      || element.textContent
      || element.getAttribute("href")
      || element.getAttribute("src");
    return limitText(value, 180);
  };

  const actionTargetSnapshot = (element, index) => {
    const rect = roundedRect(element);
    const visible = isVisibleElement(element, rect);
    return {
      index,
      action_kind: actionKindForElement(element),
      selector: cssSelector(element),
      tag: element.tagName.toLowerCase(),
      role: element.getAttribute("role"),
      label: actionTargetLabel(element),
      name: element.getAttribute("name"),
      type: element.getAttribute("type"),
      href: element.getAttribute("href"),
      src: element.getAttribute("src"),
      text: limitText(element.innerText || element.textContent, 240),
      rect: visible ? rect : null,
      visible,
      disabled: Boolean(element.disabled) || element.getAttribute("aria-disabled") === "true",
      focused: element === document.activeElement,
      attributes: domAttributeSnapshot(element)
    };
  };

  const collectActionTargetRows = () => {
    let candidates = [];
    try {
      candidates = Array.from(document.querySelectorAll(ACTION_TARGET_SELECTOR));
    } catch (_error) {
      candidates = [];
    }

    const seen = new Set();
    const targets = [];
    let hiddenSkipped = 0;
    let duplicateSkipped = 0;

    for (const element of candidates) {
      if (!(element instanceof Element)) continue;
      const rect = roundedRect(element);
      const visible = isVisibleElement(element, rect);
      if (!visible && element !== document.activeElement) {
        hiddenSkipped += 1;
        continue;
      }

      const selector = cssSelector(element);
      const key = selector || `${element.tagName}:${actionTargetLabel(element) || ""}:${targets.length}`;
      if (seen.has(key)) {
        duplicateSkipped += 1;
        continue;
      }
      seen.add(key);

      targets.push(actionTargetSnapshot(element, targets.length));
      if (targets.length >= MAX_ACTION_TARGETS) break;
    }

    return {
      candidates: candidates.length,
      targets,
      hidden_skipped: hiddenSkipped,
      duplicate_skipped: duplicateSkipped,
      truncated: Math.max(0, candidates.length - hiddenSkipped - duplicateSkipped - targets.length)
    };
  };

  const readinessSelectorSnapshot = () => {
    const selectors = [
      "body",
      "main",
      "form",
      "input, textarea, select, [contenteditable='true']",
      "button, [role='button']",
      "a[href]",
      "[data-testid], [data-test], [data-cy]",
      "[aria-busy='true']",
      "[aria-live]",
      "[role='dialog']"
    ];

    return selectors.map((selector) => {
      let elements = [];
      try {
        elements = Array.from(document.querySelectorAll(selector));
      } catch (_error) {
        elements = [];
      }

      const firstVisible = elements.find((element) => {
        if (!(element instanceof Element)) return false;
        return isVisibleElement(element, roundedRect(element));
      });

      return {
        selector,
        count: elements.length,
        first_visible: firstVisible ? elementSnapshot(firstVisible) : null
      };
    });
  };

  const navigationTimingSnapshot = () => {
    const navigation = performance.getEntriesByType ? performance.getEntriesByType("navigation")[0] : null;
    if (!navigation) return null;
    return {
      type: navigation.type,
      dom_content_loaded_ms: Math.round(navigation.domContentLoadedEventEnd || 0),
      load_event_ms: Math.round(navigation.loadEventEnd || 0),
      response_end_ms: Math.round(navigation.responseEnd || 0),
      transfer_size: navigation.transferSize || 0,
      encoded_body_size: navigation.encodedBodySize || 0,
      decoded_body_size: navigation.decodedBodySize || 0
    };
  };

  const readinessBusyCount = () => {
    try {
      return document.querySelectorAll("[aria-busy='true'], [data-loading='true'], [data-pending='true'], .loading, .spinner, [role='progressbar']").length;
    } catch (_error) {
      return 0;
    }
  };

  const readinessQuietForMs = () => {
    if (mutationState.lastMutationMs == null) {
      return Math.round(performance.now());
    }
    return Math.max(0, Math.round(performance.now() - mutationState.lastMutationMs));
  };

  const waitSelectorContracts = () => {
    const rules = [
      { name: "body", selector: "body", reason: "Document body exists" },
      { name: "app root", selector: '#root, #app, #__next, [data-reactroot]', reason: "Common app shell root" },
      { name: "main content", selector: "main, [role='main']", reason: "Primary page content" },
      { name: "headings", selector: "h1, h2, h3", reason: "Human-readable page section anchor" },
      { name: "form controls", selector: "input, textarea, select, [contenteditable='true']", reason: "Typing/select targets" },
      { name: "buttons", selector: "button, [role='button']", reason: "Click targets" },
      { name: "links", selector: "a[href]", reason: "Navigation targets" },
      { name: "test ids", selector: "[data-testid], [data-test], [data-cy]", reason: "Stable automation hooks" },
      { name: "dialogs", selector: "[role='dialog'], dialog, [aria-modal='true']", reason: "Modal or blocking surfaces" },
      { name: "busy indicators", selector: "[aria-busy='true'], [data-loading='true'], [data-pending='true'], .loading, .spinner, [role='progressbar']", reason: "Loading state indicators" }
    ];

    return rules.map((rule) => {
      let elements = [];
      try {
        elements = Array.from(document.querySelectorAll(rule.selector));
      } catch (_error) {
        elements = [];
      }

      const visibleElements = elements.filter((element) => element instanceof Element && isVisibleElement(element, roundedRect(element)));
      return {
        name: rule.name,
        selector: rule.selector,
        reason: rule.reason,
        present: elements.length > 0,
        visible: visibleElements.length > 0,
        count: elements.length,
        visible_count: visibleElements.length,
        first_visible: visibleElements[0] ? elementSnapshot(visibleElements[0]) : null
      };
    });
  };

  const waitTextCandidates = () => {
    const sources = [
      { source: "heading", selector: "h1, h2, h3" },
      { source: "button", selector: "button, [role='button']" },
      { source: "label", selector: "label, [aria-label], [placeholder]" },
      { source: "link", selector: "a[href]" },
      { source: "live-region", selector: "[aria-live]" },
      { source: "dialog", selector: "[role='dialog'], dialog, [aria-modal='true']" },
      { source: "main", selector: "main, [role='main']" }
    ];
    const seen = new Set();
    const candidates = [];

    for (const source of sources) {
      let elements = [];
      try {
        elements = Array.from(document.querySelectorAll(source.selector)).slice(0, 36);
      } catch (_error) {
        elements = [];
      }

      for (const element of elements) {
        if (!(element instanceof Element)) continue;
        const rect = roundedRect(element);
        if (!isVisibleElement(element, rect)) continue;
        const text = limitText(
          element.getAttribute("aria-label")
            || element.getAttribute("placeholder")
            || element.innerText
            || element.textContent,
          source.source === "main" ? 280 : 160
        );
        if (!text || seen.has(text)) continue;
        seen.add(text);
        candidates.push({
          source: source.source,
          text,
          selector: cssSelector(element),
          tag: element.tagName.toLowerCase(),
          rect
        });
        if (candidates.length >= 60) return candidates;
      }
    }

    return candidates;
  };

  const recommendedWaitRecipe = () => {
    const quietForMs = readinessQuietForMs();
    const busyCount = readinessBusyCount();
    return {
      document_complete: document.readyState === "complete",
      network_idle: runtimeState.pendingNetwork === 0,
      dom_quiet_500ms: quietForMs >= 500,
      no_busy_indicators: busyCount === 0,
      suggested_sequence: [
        "wait for document readyState to be complete",
        "wait for pending_network to be 0",
        "wait for dom_quiet_500ms to be true",
        "prefer a stable selector or exact visible text from this contract before interacting"
      ],
      quiet_for_ms: quietForMs,
      pending_network: runtimeState.pendingNetwork,
      busy_indicators: busyCount
    };
  };

  const interactionPlanWarnings = (actionTargets, recommended) => {
    const warnings = [];
    if (!recommended.document_complete) {
      warnings.push({
        code: "document_not_complete",
        message: "The document has not reached readyState=complete."
      });
    }
    if (!recommended.network_idle) {
      warnings.push({
        code: "network_pending",
        message: "There are pending fetch/XHR requests."
      });
    }
    if (!recommended.dom_quiet_500ms) {
      warnings.push({
        code: "dom_mutating",
        message: "The DOM changed recently; wait for quiet_for_ms to pass 500ms."
      });
    }
    if (!recommended.no_busy_indicators) {
      warnings.push({
        code: "busy_indicators_present",
        message: "Loading or busy indicators are visible in the page."
      });
    }
    if (actionTargets.truncated > 0) {
      warnings.push({
        code: "targets_truncated",
        message: "The visible action target list was truncated; re-collect with a narrower page state if needed.",
        count: actionTargets.truncated
      });
    }
    if (document.querySelector("input[type='password']")) {
      warnings.push({
        code: "password_field_present",
        message: "A password field exists; automation should not type credentials without explicit user approval."
      });
    }
    if (document.querySelector("input[type='file']")) {
      warnings.push({
        code: "file_picker_present",
        message: "A file picker exists; file upload actions require a separate explicit file permission."
      });
    }
    if (document.querySelector("[role='dialog'], dialog, [aria-modal='true']")) {
      warnings.push({
        code: "dialog_present",
        message: "A dialog or modal may intercept page interaction."
      });
    }
    return warnings;
  };

  const interactionPlanScrollTargets = () => {
    const candidates = [
      document.scrollingElement,
      document.documentElement,
      document.body,
      ...Array.from(document.querySelectorAll("main, [role='main'], [data-scroll-area], [data-radix-scroll-area-viewport], [style*='overflow']"))
    ].filter(Boolean);
    const seen = new Set();
    const targets = [];

    for (const element of candidates) {
      if (!(element instanceof Element)) continue;
      const selector = cssSelector(element) || element.tagName.toLowerCase();
      if (seen.has(selector)) continue;
      seen.add(selector);
      const rect = roundedRect(element);
      const scrollHeight = element.scrollHeight || 0;
      const clientHeight = element.clientHeight || 0;
      const scrollWidth = element.scrollWidth || 0;
      const clientWidth = element.clientWidth || 0;
      if (scrollHeight <= clientHeight && scrollWidth <= clientWidth) continue;
      targets.push({
        selector,
        tag: element.tagName.toLowerCase(),
        rect,
        scroll_top: Math.round(element.scrollTop || 0),
        scroll_left: Math.round(element.scrollLeft || 0),
        scroll_height: scrollHeight,
        client_height: clientHeight,
        scroll_width: scrollWidth,
        client_width: clientWidth
      });
      if (targets.length >= 16) break;
    }

    return targets;
  };

  const interactionPlanGroups = (actionTargets) => {
    const targets = actionTargets.targets || [];
    const clickTargets = targets.filter((target) => target.action_kind === "click").slice(0, 32);
    const typeTargets = targets.filter((target) => target.action_kind === "type").slice(0, 24);
    const selectTargets = targets.filter((target) => target.action_kind === "select").slice(0, 24);
    const mediaTargets = targets.filter((target) => target.action_kind === "media").slice(0, 12);
    const scrollTargets = interactionPlanScrollTargets();

    return [
      {
        action: "wait",
        dry_run_only: true,
        available: true,
        requires_interactive_permission: false,
        reason: "Readiness checks must pass before browser control actions.",
        target_count: 0
      },
      {
        action: "click",
        dry_run_only: true,
        available: clickTargets.length > 0,
        requires_interactive_permission: true,
        target_count: clickTargets.length,
        targets: clickTargets
      },
      {
        action: "type_text",
        dry_run_only: true,
        available: typeTargets.length > 0,
        requires_interactive_permission: true,
        target_count: typeTargets.length,
        targets: typeTargets
      },
      {
        action: "select",
        dry_run_only: true,
        available: selectTargets.length > 0,
        requires_interactive_permission: true,
        target_count: selectTargets.length,
        targets: selectTargets
      },
      {
        action: "scroll",
        dry_run_only: true,
        available: scrollTargets.length > 0,
        requires_interactive_permission: true,
        target_count: scrollTargets.length,
        targets: scrollTargets
      },
      {
        action: "media_control",
        dry_run_only: true,
        available: mediaTargets.length > 0,
        requires_interactive_permission: true,
        target_count: mediaTargets.length,
        targets: mediaTargets
      },
      {
        action: "press_key",
        dry_run_only: true,
        available: true,
        requires_interactive_permission: true,
        target_count: 0,
        allowed_examples: ["Escape", "Enter", "Tab", "ArrowDown", "ArrowUp"]
      }
    ];
  };

  const interactionPreflight = (actionTargets, recommended) => {
    const warnings = interactionPlanWarnings(actionTargets, recommended);
    const hasPasswordField = Boolean(document.querySelector("input[type='password']"));
    const hasFilePicker = Boolean(document.querySelector("input[type='file']"));
    const actionTargetCount = (actionTargets.targets || []).length;
    const checks = [
      {
        check: "document_complete",
        passed: Boolean(recommended.document_complete),
        detail: "document.readyState must be complete before interactive browser actions."
      },
      {
        check: "network_idle",
        passed: Boolean(recommended.network_idle),
        detail: "fetch/XHR activity should be idle before targeting the page."
      },
      {
        check: "dom_quiet_500ms",
        passed: Boolean(recommended.dom_quiet_500ms),
        detail: "DOM mutations should be quiet for at least 500ms."
      },
      {
        check: "no_busy_indicators",
        passed: Boolean(recommended.no_busy_indicators),
        detail: "Visible loading or busy indicators can intercept or invalidate actions."
      },
      {
        check: "has_visible_action_targets",
        passed: actionTargetCount > 0,
        detail: "At least one visible target should be present before click/type/select planning."
      },
      {
        check: "no_password_field_without_permission",
        passed: !hasPasswordField,
        detail: "Password fields require explicit user approval for credential entry."
      },
      {
        check: "no_file_picker_without_permission",
        passed: !hasFilePicker,
        detail: "File pickers require explicit file-selection approval."
      }
    ];
    const blockers = checks
      .filter((check) => !check.passed)
      .map((check) => ({
        code: check.check,
        message: check.detail
      }));
    const status = blockers.length > 0 ? "blocked_until_ready" : "ready_for_permissioned_action";
    return {
      status,
      checks,
      blockers,
      warnings,
      receipt_contract: {
        schema: "zed.web_preview.interaction_receipt.v1",
        required_fields: [
          "session_id",
          "action_id",
          "action",
          "selector",
          "planned_at",
          "attempted_at",
          "permission_mode",
          "preflight_timestamp",
          "before_url",
          "after_url",
          "before_ready_state",
          "after_ready_state",
          "outcome",
          "error"
        ],
        post_action_context: [
          "readiness_probe",
          "runtime_events",
          "action_targets",
          "screenshot_or_capture_area_when_requested"
        ]
      },
      execution_rules: [
        "Do not execute actions from this payload; it is a preflight snapshot.",
        "Re-collect readiness, wait contract, action targets, and interaction plan immediately before execution.",
        "Interactive actions must be unlocked in Zed and must emit a receipt with before/after context.",
        "Never type credentials or select files without a separate explicit user instruction."
      ]
    };
  };

  const interactionReceiptTemplate = (actionTargets, recommended) => {
    const actionId =
      typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
        ? crypto.randomUUID()
        : `interaction-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    const targetCandidates = (actionTargets.targets || []).slice(0, 12).map((target) => ({
      action_kind: target.action_kind,
      selector: target.selector,
      label: target.label,
      tag: target.tag,
      role: target.role,
      rect: target.rect,
      disabled: target.disabled,
      focused: target.focused
    }));
    return {
      schema: "zed.web_preview.interaction_receipt.v1",
      action_id: actionId,
      dry_run_only: true,
      planned_at: new Date().toISOString(),
      target_candidates: targetCandidates,
      before_context: {
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        scroll_x: Math.round(window.scrollX || 0),
        scroll_y: Math.round(window.scrollY || 0),
        pending_network: runtimeState.pendingNetwork,
        dom_quiet_for_ms: readinessQuietForMs(),
        busy_indicators: readinessBusyCount(),
        recommended_wait: recommended
      },
      required_receipt_fields: [
        "session_id",
        "action_id",
        "action",
        "selector",
        "planned_at",
        "attempted_at",
        "permission_mode",
        "preflight_timestamp",
        "before_context",
        "after_context",
        "outcome",
        "error"
      ],
      after_context_fields: [
        "url",
        "title",
        "ready_state",
        "focused_selector",
        "scroll_x",
        "scroll_y",
        "pending_network",
        "dom_quiet_for_ms",
        "busy_indicators",
        "runtime_error_count",
        "failed_network_count"
      ],
      outcome_values: ["not_attempted", "succeeded", "blocked", "failed", "timed_out"],
      blocking_rules: [
        "Block when Zed interactive actions are locked.",
        "Block when the target selector is missing or no longer visible.",
        "Block credential or file picker actions without a separate explicit user instruction.",
        "Block when a fresh preflight reports document, network, DOM, or busy-indicator blockers."
      ],
      notes: [
        "This is a receipt template only; it does not execute browser input.",
        "Interactive tools should fill this receipt after every attempted action.",
        "Receipts must include before and after context so regressions can be audited."
      ]
    };
  };

  const interactionActionRequestEnvelope = (actionTargets, recommended) => {
    const requestId =
      typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
        ? crypto.randomUUID()
        : `request-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    const targets = actionTargets.targets || [];
    const candidatesFor = (kind, limit) =>
      targets
        .filter((target) => target.action_kind === kind)
        .slice(0, limit)
        .map((target) => ({
          selector: target.selector,
          label: target.label,
          tag: target.tag,
          role: target.role,
          rect: target.rect,
          disabled: target.disabled,
          focused: target.focused
        }));
    const preflight = interactionPreflight(actionTargets, recommended);
    const receipt = interactionReceiptTemplate(actionTargets, recommended);
    const actionRequests = [
      {
        action: "click",
        requires_interactive_permission: true,
        payload_schema: {
          selector: "string",
          button: "left|middle|right",
          click_count: "number"
        },
        candidates: candidatesFor("click", 12)
      },
      {
        action: "type_text",
        requires_interactive_permission: true,
        payload_schema: {
          selector: "string",
          text: "string",
          clear_existing: "boolean"
        },
        candidates: candidatesFor("type", 12)
      },
      {
        action: "select",
        requires_interactive_permission: true,
        payload_schema: {
          selector: "string",
          value_or_label: "string"
        },
        candidates: candidatesFor("select", 8)
      },
      {
        action: "scroll",
        requires_interactive_permission: true,
        payload_schema: {
          selector: "string|null",
          delta_x: "number",
          delta_y: "number"
        },
        candidates: interactionPlanScrollTargets().slice(0, 8)
      },
      {
        action: "press_key",
        requires_interactive_permission: true,
        payload_schema: {
          key: "Escape|Enter|Tab|ArrowDown|ArrowUp|custom",
          modifiers: "array"
        },
        candidates: []
      },
      {
        action: "media_control",
        requires_interactive_permission: true,
        payload_schema: {
          selector: "string",
          control: "play|pause|mute|unmute|seek"
        },
        candidates: candidatesFor("media", 8)
      }
    ];
    return {
      schema: "zed.web_preview.interaction_action_request.v1",
      request_id: requestId,
      status: preflight.status === "ready_for_permissioned_action"
        ? "permission_required"
        : "blocked_by_preflight",
      dry_run_only: true,
      generated_at: new Date().toISOString(),
      preflight_status: preflight.status,
      blockers: preflight.blockers,
      action_requests: actionRequests,
      receipt_template: receipt,
      execution_contract: {
        must_refresh_before_execution: [
          "readiness_probe",
          "wait_contract",
          "action_targets",
          "interaction_preflight"
        ],
        must_emit_after_execution: "interaction_receipt",
        must_not_execute_from_this_payload: true,
        requires_user_permission_gate: true
      },
      notes: [
        "This envelope is a planning artifact for future interactive browser tools.",
        "It intentionally does not click, type, scroll, or press keys.",
        "A future executor must reject this envelope unless Zed's interactive permission is unlocked and a fresh preflight passes."
      ]
    };
  };

  const blockedInteractionReceipt = (actionTargets, recommended) => {
    const actionEnvelope = interactionActionRequestEnvelope(actionTargets, recommended);
    const preflight = interactionPreflight(actionTargets, recommended);
    const baseReceipt = interactionReceiptTemplate(actionTargets, recommended);
    const blockers = preflight.blockers.slice();
    if (actionEnvelope.status === "blocked_by_preflight" && blockers.length === 0) {
      blockers.push({
        code: "preflight_not_ready",
        message: "The action request envelope reported a blocked preflight."
      });
    }
    return {
      schema: "zed.web_preview.interaction_receipt.v1",
      action_id: baseReceipt.action_id,
      request_id: actionEnvelope.request_id,
      action: "not_selected",
      selector: null,
      planned_at: actionEnvelope.generated_at,
      attempted_at: new Date().toISOString(),
      permission_mode: "unknown_until_rust_gate_is_applied",
      preflight_timestamp: new Date().toISOString(),
      before_context: baseReceipt.before_context,
      after_context: {
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        focused_selector: document.activeElement instanceof Element ? cssSelector(document.activeElement) : null,
        scroll_x: Math.round(window.scrollX || 0),
        scroll_y: Math.round(window.scrollY || 0),
        pending_network: runtimeState.pendingNetwork,
        dom_quiet_for_ms: readinessQuietForMs(),
        busy_indicators: readinessBusyCount(),
        runtime_error_count: runtimeEvents.console.filter((event) => event.level === "error").length,
        failed_network_count: runtimeEvents.network.filter((event) => event.ok === false || event.status >= 400).length
      },
      outcome: "blocked",
      error: {
        code: blockers.length > 0 ? "preflight_blocked" : "permission_gate_pending",
        message: blockers.length > 0
          ? "The browser action was blocked by page readiness or safety checks."
          : "The browser action must be blocked unless Rust confirms interactive permission is unlocked.",
        blockers
      },
      dry_run_only: true,
      action_request: actionEnvelope,
      notes: [
        "This is a blocked-action audit receipt, not an executed browser action.",
        "Rust adds the live Zed permission gate to this receipt snapshot.",
        "Future executors should emit this shape whenever an action is denied before input dispatch."
      ]
    };
  };

  const successfulInteractionReceipt = (actionTargets, recommended) => {
    const actionEnvelope = interactionActionRequestEnvelope(actionTargets, recommended);
    const baseReceipt = interactionReceiptTemplate(actionTargets, recommended);
    const firstCandidate = actionEnvelope.action_requests
      .flatMap((request) => request.candidates.map((candidate) => ({
        action: request.action,
        candidate
      })))
      [0] || null;
    return {
      schema: "zed.web_preview.interaction_receipt.v1",
      action_id: baseReceipt.action_id,
      request_id: actionEnvelope.request_id,
      action: firstCandidate?.action || "not_selected",
      selector: firstCandidate?.candidate?.selector || null,
      planned_at: actionEnvelope.generated_at,
      attempted_at: null,
      completed_at: null,
      permission_mode: "unknown_until_rust_gate_is_applied",
      preflight_timestamp: new Date().toISOString(),
      before_context: baseReceipt.before_context,
      after_context: {
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        focused_selector: document.activeElement instanceof Element ? cssSelector(document.activeElement) : null,
        scroll_x: Math.round(window.scrollX || 0),
        scroll_y: Math.round(window.scrollY || 0),
        pending_network: runtimeState.pendingNetwork,
        dom_quiet_for_ms: readinessQuietForMs(),
        busy_indicators: readinessBusyCount(),
        runtime_error_count: runtimeEvents.console.filter((event) => event.level === "error").length,
        failed_network_count: runtimeEvents.network.filter((event) => event.ok === false || event.status >= 400).length
      },
      outcome: "succeeded",
      error: null,
      sample_only: true,
      dry_run_only: true,
      action_request: actionEnvelope,
      verification: {
        required_after_action_refreshes: [
          "readiness_probe",
          "runtime_events",
          "action_targets",
          "optional_screenshot"
        ],
        success_requires: [
          "The requested target was still present or intentionally changed.",
          "The action did not create new runtime or network errors unless expected.",
          "The after_context was collected after the page settled or timed out.",
          "The receipt was attached to the Agent Panel response."
        ]
      },
      notes: [
        "This is a sample success receipt template, not proof that browser input was executed.",
        "A future executor must replace attempted_at, completed_at, action, selector, and after_context after real input dispatch.",
        "Successful receipts and blocked receipts use the same schema so agent workflows can audit every requested action."
      ]
    };
  };

  installRuntimeCapture();

  window.__zedWebPreview = {
    collectPageDiagnostics(action = "copy") {
      const root = document.documentElement;
      const body = document.body;
      const selection = window.getSelection ? window.getSelection() : null;
      const activeElement = document.activeElement instanceof Element ? document.activeElement : null;
      const headings = sampleElements("h1, h2, h3", 24).map((heading) => ({
        level: heading.tag,
        text: heading.text,
        selector: heading.selector
      }));
      const payload = {
        kind: "page-diagnostics",
        action,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        document: {
          ready_state: document.readyState,
          content_type: document.contentType,
          character_set: document.characterSet,
          compat_mode: document.compatMode,
          language: root?.lang || null,
          direction: root ? window.getComputedStyle(root).direction : null,
          referrer: document.referrer || null,
          has_focus: document.hasFocus(),
          visibility_state: document.visibilityState
        },
        viewport: {
          inner_width: window.innerWidth,
          inner_height: window.innerHeight,
          outer_width: window.outerWidth,
          outer_height: window.outerHeight,
          device_pixel_ratio: window.devicePixelRatio || 1,
          scroll_x: Math.round(window.scrollX || 0),
          scroll_y: Math.round(window.scrollY || 0),
          scroll_width: Math.max(root?.scrollWidth || 0, body?.scrollWidth || 0),
          scroll_height: Math.max(root?.scrollHeight || 0, body?.scrollHeight || 0)
        },
        counts: {
          links: document.links?.length || 0,
          buttons: document.querySelectorAll("button, [role='button']").length,
          inputs: document.querySelectorAll("input, textarea, select, [contenteditable='true']").length,
          forms: document.forms?.length || 0,
          images: document.images?.length || 0,
          videos: document.querySelectorAll("video").length,
          audio: document.querySelectorAll("audio").length,
          iframes: document.querySelectorAll("iframe").length,
          scripts: document.scripts?.length || 0,
          stylesheets: document.styleSheets?.length || 0,
          headings: document.querySelectorAll("h1, h2, h3, h4, h5, h6").length,
          landmarks: document.querySelectorAll("main, nav, aside, header, footer, [role='main'], [role='navigation'], [role='complementary']").length
        },
        active_element: elementSnapshot(activeElement),
        selection: selection && String(selection).trim() ? limitText(selection.toString(), 600) : null,
        headings,
        landmarks: sampleElements("main, nav, aside, header, footer, [role='main'], [role='navigation'], [role='complementary']", 18),
        controls: sampleElements("button, [role='button'], input, textarea, select, [contenteditable='true']", 40),
        links: sampleElements("a[href]", 30),
        media: {
          images: sampleElements("img", 20),
          videos: sampleElements("video", 12),
          audio: sampleElements("audio", 12),
          iframes: sampleElements("iframe", 12)
        },
        forms: collectForms(),
        performance: collectPerformance()
      };
      post(payload);
    },

    collectRuntimeEvents(action = "copy") {
      const consoleEvents = runtimeEvents.console.slice(-MAX_RUNTIME_EVENTS);
      const networkEvents = runtimeEvents.network.slice(-MAX_RUNTIME_EVENTS);
      const failedNetwork = networkEvents.filter((event) => event.ok === false || event.status >= 400);
      const warnings = consoleEvents.filter((event) => event.level === "warn");
      const errors = consoleEvents.filter((event) => event.level === "error");

      post({
        kind: "runtime-events",
        action,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        counts: {
          console: consoleEvents.length,
          warnings: warnings.length,
          errors: errors.length,
          network: networkEvents.length,
          failed_network: failedNetwork.length,
          pending_network: runtimeState.pendingNetwork,
          performance_resources: performance.getEntriesByType ? performance.getEntriesByType("resource").length : 0
        },
        console: consoleEvents,
        network: networkEvents,
        failed_network: failedNetwork.slice(-40),
        performance_resources: runtimeResourceSnapshot()
      });
    },

    collectDomSnapshot(action = "copy") {
      const root = document.body || document.documentElement;
      const html = document.documentElement;
      const body = document.body;
      const budget = { nodes: 0, truncated: 0 };
      const rootSnapshot = domTreeNode(root, 0, 0, budget);

      post({
        kind: "dom-snapshot",
        action,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        document: {
          content_type: document.contentType,
          character_set: document.characterSet,
          language: html?.lang || null,
          direction: html ? window.getComputedStyle(html).direction : null,
          has_focus: document.hasFocus(),
          visibility_state: document.visibilityState,
          active_element: elementSnapshot(document.activeElement instanceof Element ? document.activeElement : null)
        },
        viewport: {
          inner_width: window.innerWidth,
          inner_height: window.innerHeight,
          device_pixel_ratio: window.devicePixelRatio || 1,
          scroll_x: Math.round(window.scrollX || 0),
          scroll_y: Math.round(window.scrollY || 0),
          scroll_width: Math.max(html?.scrollWidth || 0, body?.scrollWidth || 0),
          scroll_height: Math.max(html?.scrollHeight || 0, body?.scrollHeight || 0)
        },
        counts: {
          nodes: budget.nodes,
          truncated_nodes: budget.truncated,
          max_nodes: MAX_DOM_SNAPSHOT_NODES,
          max_depth: MAX_DOM_SNAPSHOT_DEPTH,
          max_children_per_node: MAX_DOM_SNAPSHOT_CHILDREN,
          total_elements: document.getElementsByTagName("*").length,
          links: document.links?.length || 0,
          buttons: document.querySelectorAll("button, [role='button']").length,
          inputs: document.querySelectorAll("input, textarea, select, [contenteditable='true']").length,
          forms: document.forms?.length || 0
        },
        root: rootSnapshot
      });
    },

    collectActionTargets(action = "copy") {
      const result = collectActionTargetRows();
      post({
        kind: "action-targets",
        action,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        viewport: {
          inner_width: window.innerWidth,
          inner_height: window.innerHeight,
          device_pixel_ratio: window.devicePixelRatio || 1,
          scroll_x: Math.round(window.scrollX || 0),
          scroll_y: Math.round(window.scrollY || 0)
        },
        counts: {
          candidates: result.candidates,
          targets: result.targets.length,
          hidden_skipped: result.hidden_skipped,
          duplicate_skipped: result.duplicate_skipped,
          truncated: result.truncated,
          max_targets: MAX_ACTION_TARGETS
        },
        active_element: elementSnapshot(document.activeElement instanceof Element ? document.activeElement : null),
        targets: result.targets
      });
    },

    collectReadinessProbe(action = "copy") {
      try { installMutationCapture(); } catch (_error) {}
      const html = document.documentElement;
      const body = document.body;
      const actionTargets = collectActionTargetRows();
      const quietForMs = readinessQuietForMs();
      const busyCount = readinessBusyCount();
      const networkIdle = runtimeState.pendingNetwork === 0;
      const domQuiet = quietForMs >= 500;
      const documentInteractive = document.readyState === "interactive" || document.readyState === "complete";
      const documentComplete = document.readyState === "complete";

      post({
        kind: "readiness-probe",
        action,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        readiness: {
          interactive: documentInteractive,
          complete: documentComplete,
          network_idle: networkIdle,
          dom_quiet: domQuiet,
          no_busy_indicators: busyCount === 0,
          settled: documentComplete && networkIdle && domQuiet && busyCount === 0,
          quiet_for_ms: quietForMs,
          pending_network: runtimeState.pendingNetwork,
          busy_indicators: busyCount
        },
        document: {
          content_type: document.contentType,
          character_set: document.characterSet,
          language: html?.lang || null,
          has_focus: document.hasFocus(),
          visibility_state: document.visibilityState,
          active_element: elementSnapshot(document.activeElement instanceof Element ? document.activeElement : null)
        },
        viewport: {
          inner_width: window.innerWidth,
          inner_height: window.innerHeight,
          device_pixel_ratio: window.devicePixelRatio || 1,
          scroll_x: Math.round(window.scrollX || 0),
          scroll_y: Math.round(window.scrollY || 0),
          scroll_width: Math.max(html?.scrollWidth || 0, body?.scrollWidth || 0),
          scroll_height: Math.max(html?.scrollHeight || 0, body?.scrollHeight || 0)
        },
        mutation: {
          observer_available: mutationState.observerAvailable,
          observed: mutationState.observed,
          count: mutationState.count,
          last_mutation_at: mutationState.lastMutationAt,
          quiet_for_ms: quietForMs
        },
        counts: {
          action_targets: actionTargets.targets.length,
          action_target_candidates: actionTargets.candidates,
          links: document.links?.length || 0,
          buttons: document.querySelectorAll("button, [role='button']").length,
          inputs: document.querySelectorAll("input, textarea, select, [contenteditable='true']").length,
          forms: document.forms?.length || 0,
          busy_indicators: busyCount,
          body_text_length: body?.innerText?.length || 0
        },
        navigation_timing: navigationTimingSnapshot(),
        selector_probe: readinessSelectorSnapshot(),
        action_target_sample: actionTargets.targets.slice(0, 24)
      });
    },

    collectWaitContract(action = "copy") {
      try { installMutationCapture(); } catch (_error) {}
      const html = document.documentElement;
      const body = document.body;
      const actionTargets = collectActionTargetRows();
      const selectorContracts = waitSelectorContracts();
      const textCandidates = waitTextCandidates();
      const recommended = recommendedWaitRecipe();

      post({
        kind: "wait-contract",
        action,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        recommended,
        viewport: {
          inner_width: window.innerWidth,
          inner_height: window.innerHeight,
          device_pixel_ratio: window.devicePixelRatio || 1,
          scroll_x: Math.round(window.scrollX || 0),
          scroll_y: Math.round(window.scrollY || 0),
          scroll_width: Math.max(html?.scrollWidth || 0, body?.scrollWidth || 0),
          scroll_height: Math.max(html?.scrollHeight || 0, body?.scrollHeight || 0)
        },
        counts: {
          selector_contracts: selectorContracts.length,
          selector_contracts_present: selectorContracts.filter((contract) => contract.present).length,
          selector_contracts_visible: selectorContracts.filter((contract) => contract.visible).length,
          text_candidates: textCandidates.length,
          action_targets: actionTargets.targets.length,
          busy_indicators: readinessBusyCount(),
          pending_network: runtimeState.pendingNetwork
        },
        mutation: {
          observed: mutationState.observed,
          count: mutationState.count,
          last_mutation_at: mutationState.lastMutationAt,
          quiet_for_ms: readinessQuietForMs()
        },
        selector_contracts: selectorContracts,
        text_candidates: textCandidates,
        action_target_sample: actionTargets.targets.slice(0, 24),
        notes: [
          "This is a read-only snapshot for agent planning.",
          "Automation should re-collect this contract immediately before interacting.",
          "Prefer data-testid/data-test/data-cy selectors or exact visible text candidates when available."
        ]
      });
    },

    collectInteractionPlan(action = "copy") {
      try { installMutationCapture(); } catch (_error) {}
      const html = document.documentElement;
      const body = document.body;
      const actionTargets = collectActionTargetRows();
      const selectorContracts = waitSelectorContracts();
      const textCandidates = waitTextCandidates();
      const recommended = recommendedWaitRecipe();
      const actionGroups = interactionPlanGroups(actionTargets);
      const warnings = interactionPlanWarnings(actionTargets, recommended);

      post({
        kind: "interaction-plan",
        action,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        dry_run_only: true,
        recommended_wait: recommended,
        permissions: {
          required_for_interactive_actions: true,
          current_payload_executes_actions: false,
          user_must_unlock_interactive_actions_in_zed: true
        },
        viewport: {
          inner_width: window.innerWidth,
          inner_height: window.innerHeight,
          device_pixel_ratio: window.devicePixelRatio || 1,
          scroll_x: Math.round(window.scrollX || 0),
          scroll_y: Math.round(window.scrollY || 0),
          scroll_width: Math.max(html?.scrollWidth || 0, body?.scrollWidth || 0),
          scroll_height: Math.max(html?.scrollHeight || 0, body?.scrollHeight || 0)
        },
        counts: {
          action_groups: actionGroups.length,
          available_action_groups: actionGroups.filter((group) => group.available).length,
          action_targets: actionTargets.targets.length,
          selector_contracts: selectorContracts.length,
          text_candidates: textCandidates.length,
          warnings: warnings.length,
          pending_network: runtimeState.pendingNetwork,
          busy_indicators: readinessBusyCount()
        },
        warnings,
        action_groups: actionGroups,
        selector_contracts: selectorContracts,
        text_candidates: textCandidates,
        mutation: {
          observed: mutationState.observed,
          count: mutationState.count,
          last_mutation_at: mutationState.lastMutationAt,
          quiet_for_ms: readinessQuietForMs()
        },
        notes: [
          "This plan is intentionally dry-run only.",
          "Re-collect readiness, wait contract, and action targets before executing any future interactive action.",
          "The Rust-side session snapshot includes whether interactive agent actions are currently locked or unlocked."
        ]
      });
    },

    collectInteractionPreflight(action = "copy") {
      try { installMutationCapture(); } catch (_error) {}
      const html = document.documentElement;
      const body = document.body;
      const actionTargets = collectActionTargetRows();
      const selectorContracts = waitSelectorContracts();
      const textCandidates = waitTextCandidates();
      const recommended = recommendedWaitRecipe();
      const actionGroups = interactionPlanGroups(actionTargets);
      const preflight = interactionPreflight(actionTargets, recommended);

      post({
        kind: "interaction-preflight",
        action,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        recommended_wait: recommended,
        decision: {
          status: preflight.status,
          blockers: preflight.blockers,
          next_steps: preflight.status === "blocked_until_ready"
            ? ["Wait for the blocked checks to pass, then collect a fresh preflight."]
            : ["Unlock interactive agent actions in Zed only when the next action is user-approved."]
        },
        checks: preflight.checks,
        warnings: preflight.warnings,
        permissions: {
          required_for_interactive_actions: true,
          current_payload_executes_actions: false,
          user_must_unlock_interactive_actions_in_zed: true,
          credentials_require_separate_approval: true,
          file_uploads_require_separate_approval: true
        },
        viewport: {
          inner_width: window.innerWidth,
          inner_height: window.innerHeight,
          device_pixel_ratio: window.devicePixelRatio || 1,
          scroll_x: Math.round(window.scrollX || 0),
          scroll_y: Math.round(window.scrollY || 0),
          scroll_width: Math.max(html?.scrollWidth || 0, body?.scrollWidth || 0),
          scroll_height: Math.max(html?.scrollHeight || 0, body?.scrollHeight || 0)
        },
        counts: {
          action_groups: actionGroups.length,
          available_action_groups: actionGroups.filter((group) => group.available).length,
          action_targets: actionTargets.targets.length,
          selector_contracts: selectorContracts.length,
          text_candidates: textCandidates.length,
          warnings: preflight.warnings.length,
          blockers: preflight.blockers.length,
          pending_network: runtimeState.pendingNetwork,
          busy_indicators: readinessBusyCount()
        },
        action_groups: actionGroups,
        action_target_sample: actionTargets.targets.slice(0, 20),
        selector_contracts: selectorContracts.slice(0, 20),
        text_candidates: textCandidates.slice(0, 20),
        receipt_contract: preflight.receipt_contract,
        execution_rules: preflight.execution_rules,
        mutation: {
          observed: mutationState.observed,
          count: mutationState.count,
          last_mutation_at: mutationState.lastMutationAt,
          quiet_for_ms: readinessQuietForMs()
        },
        notes: [
          "This preflight is read-only and does not perform browser input.",
          "A future interactive action must compare its target against a fresh preflight and then emit a receipt.",
          "The Rust-side snapshot adds the current Zed permission gate for this WebPreview session."
        ]
      });
    },

    collectInteractionReceiptTemplate(action = "copy") {
      try { installMutationCapture(); } catch (_error) {}
      const actionTargets = collectActionTargetRows();
      const selectorContracts = waitSelectorContracts();
      const textCandidates = waitTextCandidates();
      const recommended = recommendedWaitRecipe();
      const actionGroups = interactionPlanGroups(actionTargets);
      const receipt = interactionReceiptTemplate(actionTargets, recommended);

      post({
        kind: "interaction-receipt-template",
        action,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        receipt,
        recommended_wait: recommended,
        permissions: {
          required_for_interactive_actions: true,
          current_payload_executes_actions: false,
          user_must_unlock_interactive_actions_in_zed: true,
          receipt_required_after_attempt: true
        },
        counts: {
          action_groups: actionGroups.length,
          available_action_groups: actionGroups.filter((group) => group.available).length,
          action_targets: actionTargets.targets.length,
          selector_contracts: selectorContracts.length,
          text_candidates: textCandidates.length,
          target_candidates: receipt.target_candidates.length,
          pending_network: runtimeState.pendingNetwork,
          busy_indicators: readinessBusyCount()
        },
        action_groups: actionGroups,
        selector_contracts: selectorContracts.slice(0, 16),
        text_candidates: textCandidates.slice(0, 16),
        mutation: {
          observed: mutationState.observed,
          count: mutationState.count,
          last_mutation_at: mutationState.lastMutationAt,
          quiet_for_ms: readinessQuietForMs()
        },
        notes: [
          "This receipt template is read-only and does not perform browser input.",
          "Future interactive browser actions must populate this receipt and attach fresh after-action context.",
          "Use this template to audit automation behavior before enabling real click, type, key, or scroll execution."
        ]
      });
    },

    collectInteractionActionRequest(action = "copy") {
      try { installMutationCapture(); } catch (_error) {}
      const actionTargets = collectActionTargetRows();
      const selectorContracts = waitSelectorContracts();
      const textCandidates = waitTextCandidates();
      const recommended = recommendedWaitRecipe();
      const envelope = interactionActionRequestEnvelope(actionTargets, recommended);

      post({
        kind: "interaction-action-request",
        action,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        envelope,
        recommended_wait: recommended,
        permissions: {
          required_for_interactive_actions: true,
          current_payload_executes_actions: false,
          user_must_unlock_interactive_actions_in_zed: true,
          fresh_preflight_required_before_execution: true,
          receipt_required_after_attempt: true
        },
        counts: {
          action_requests: envelope.action_requests.length,
          executable_action_requests: envelope.action_requests.filter((request) => request.candidates.length > 0 || request.action === "press_key").length,
          action_targets: actionTargets.targets.length,
          selector_contracts: selectorContracts.length,
          text_candidates: textCandidates.length,
          blockers: envelope.blockers.length,
          pending_network: runtimeState.pendingNetwork,
          busy_indicators: readinessBusyCount()
        },
        selector_contracts: selectorContracts.slice(0, 16),
        text_candidates: textCandidates.slice(0, 16),
        mutation: {
          observed: mutationState.observed,
          count: mutationState.count,
          last_mutation_at: mutationState.lastMutationAt,
          quiet_for_ms: readinessQuietForMs()
        },
        notes: [
          "This action request envelope is read-only and does not perform browser input.",
          "Future interactive tools should accept only this schema, refresh preflight, and emit the included receipt template.",
          "Keep URL bar, tabs, and Zed overlay UI under GPUI priority; only browser body targets are eligible."
        ]
      });
    },

    collectBlockedInteractionReceipt(action = "copy") {
      try { installMutationCapture(); } catch (_error) {}
      const actionTargets = collectActionTargetRows();
      const selectorContracts = waitSelectorContracts();
      const textCandidates = waitTextCandidates();
      const recommended = recommendedWaitRecipe();
      const blockedReceipt = blockedInteractionReceipt(actionTargets, recommended);

      post({
        kind: "blocked-interaction-receipt",
        action,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        blocked_receipt: blockedReceipt,
        recommended_wait: recommended,
        permissions: {
          required_for_interactive_actions: true,
          current_payload_executes_actions: false,
          user_must_unlock_interactive_actions_in_zed: true,
          blocked_receipt_only: true
        },
        counts: {
          action_targets: actionTargets.targets.length,
          selector_contracts: selectorContracts.length,
          text_candidates: textCandidates.length,
          blockers: blockedReceipt.error.blockers.length,
          pending_network: runtimeState.pendingNetwork,
          busy_indicators: readinessBusyCount(),
          runtime_errors: runtimeEvents.console.filter((event) => event.level === "error").length,
          failed_network: runtimeEvents.network.filter((event) => event.ok === false || event.status >= 400).length
        },
        selector_contracts: selectorContracts.slice(0, 12),
        text_candidates: textCandidates.slice(0, 12),
        mutation: {
          observed: mutationState.observed,
          count: mutationState.count,
          last_mutation_at: mutationState.lastMutationAt,
          quiet_for_ms: readinessQuietForMs()
        },
        notes: [
          "This blocked receipt is read-only and does not perform browser input.",
          "Use it when a future interactive request is denied by Zed permission or page preflight.",
          "A successful future action must emit a normal interaction receipt with after-action context."
        ]
      });
    },

    collectSuccessfulInteractionReceipt(action = "copy") {
      try { installMutationCapture(); } catch (_error) {}
      const actionTargets = collectActionTargetRows();
      const selectorContracts = waitSelectorContracts();
      const textCandidates = waitTextCandidates();
      const recommended = recommendedWaitRecipe();
      const successReceipt = successfulInteractionReceipt(actionTargets, recommended);

      post({
        kind: "successful-interaction-receipt",
        action,
        timestamp: new Date().toISOString(),
        url: window.location.href,
        title: document.title,
        ready_state: document.readyState,
        success_receipt: successReceipt,
        recommended_wait: recommended,
        permissions: {
          required_for_interactive_actions: true,
          current_payload_executes_actions: false,
          user_must_unlock_interactive_actions_in_zed: true,
          success_receipt_template_only: true
        },
        counts: {
          action_targets: actionTargets.targets.length,
          selector_contracts: selectorContracts.length,
          text_candidates: textCandidates.length,
          pending_network: runtimeState.pendingNetwork,
          busy_indicators: readinessBusyCount(),
          runtime_errors: runtimeEvents.console.filter((event) => event.level === "error").length,
          failed_network: runtimeEvents.network.filter((event) => event.ok === false || event.status >= 400).length
        },
        selector_contracts: selectorContracts.slice(0, 12),
        text_candidates: textCandidates.slice(0, 12),
        mutation: {
          observed: mutationState.observed,
          count: mutationState.count,
          last_mutation_at: mutationState.lastMutationAt,
          quiet_for_ms: readinessQuietForMs()
        },
        notes: [
          "This success receipt is read-only and does not perform browser input.",
          "Use it as the required shape future successful click/type/key/scroll actions must fill.",
          "A blocked future action must emit a blocked receipt instead."
        ]
      });
    },

    inspectNextElement() {
      if (window.__zedWebPreview.__cleanup) {
        window.__zedWebPreview.__cleanup();
      }

      const overlay = createOverlay();
      const highlight = document.createElement("div");
      highlight.style.position = "fixed";
      highlight.style.border = "2px solid #4ea1ff";
      highlight.style.background = "rgba(78, 161, 255, 0.14)";
      highlight.style.pointerEvents = "none";
      highlight.style.borderRadius = "8px";
      overlay.appendChild(highlight);
      document.documentElement.appendChild(overlay);

      const move = (event) => {
        const element = event.target instanceof Element ? event.target : null;
        if (!element) return;
        const rect = element.getBoundingClientRect();
        highlight.style.left = `${rect.left}px`;
        highlight.style.top = `${rect.top}px`;
        highlight.style.width = `${rect.width}px`;
        highlight.style.height = `${rect.height}px`;
      };

      const cleanup = () => {
        document.removeEventListener("mousemove", move, true);
        document.removeEventListener("click", click, true);
        document.removeEventListener("keydown", keydown, true);
        overlay.remove();
        window.__zedWebPreview.__cleanup = null;
      };

      const click = (event) => {
        event.preventDefault();
        event.stopPropagation();
        const element = event.target instanceof Element ? event.target : null;
        if (!element) {
          cleanup();
          return;
        }

        const rect = element.getBoundingClientRect();
        cleanup();
        post({
          kind: "inspect-element",
          url: window.location.href,
          title: document.title,
          scale: window.devicePixelRatio || 1,
          selector: cssSelector(element),
          tag: element.tagName.toLowerCase(),
          id: element.id || null,
          classes: Array.from(element.classList || []),
          text: limitText(element.innerText || element.textContent, 2000),
          href: element.getAttribute("href"),
          src: element.getAttribute("src"),
          css: styleSnapshot(element),
          html: limitText(element.outerHTML, 4000),
          rect: {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height
          }
        });
      };

      const keydown = (event) => {
        if (event.key === "Escape") cleanup();
      };

      document.addEventListener("mousemove", move, true);
      document.addEventListener("click", click, true);
      document.addEventListener("keydown", keydown, true);
      window.__zedWebPreview.__cleanup = cleanup;
    },

    captureAreaScreenshot() {
      if (window.__zedWebPreview.__cleanup) {
        window.__zedWebPreview.__cleanup();
      }

      const overlay = createOverlay();
      overlay.style.pointerEvents = "auto";
      const selection = document.createElement("div");
      selection.style.position = "fixed";
      selection.style.border = "2px solid #4ea1ff";
      selection.style.background = "rgba(78, 161, 255, 0.18)";
      selection.style.pointerEvents = "none";
      selection.style.borderRadius = "10px";
      overlay.appendChild(selection);
      document.documentElement.appendChild(overlay);

      let start = null;

      const updateSelection = (x, y) => {
        if (!start) return;
        const left = Math.min(start.x, x);
        const top = Math.min(start.y, y);
        const width = Math.abs(start.x - x);
        const height = Math.abs(start.y - y);
        selection.style.left = `${left}px`;
        selection.style.top = `${top}px`;
        selection.style.width = `${width}px`;
        selection.style.height = `${height}px`;
      };

      const cleanup = () => {
        document.removeEventListener("mousedown", mouseDown, true);
        document.removeEventListener("mousemove", mouseMove, true);
        document.removeEventListener("mouseup", mouseUp, true);
        document.removeEventListener("keydown", keydown, true);
        overlay.remove();
        window.__zedWebPreview.__cleanup = null;
      };

      const mouseDown = (event) => {
        event.preventDefault();
        event.stopPropagation();
        start = { x: event.clientX, y: event.clientY };
        updateSelection(event.clientX, event.clientY);
      };

      const mouseMove = (event) => {
        if (!start) return;
        event.preventDefault();
        updateSelection(event.clientX, event.clientY);
      };

      const mouseUp = (event) => {
        if (!start) return;
        event.preventDefault();
        const left = Math.min(start.x, event.clientX);
        const top = Math.min(start.y, event.clientY);
        const width = Math.max(1, Math.abs(start.x - event.clientX));
        const height = Math.max(1, Math.abs(start.y - event.clientY));
        cleanup();
        post({
          kind: "capture-area",
          url: window.location.href,
          title: document.title,
          scale: window.devicePixelRatio || 1,
          rect: { x: left, y: top, width, height }
        });
      };

      const keydown = (event) => {
        if (event.key === "Escape") cleanup();
      };

      document.addEventListener("mousedown", mouseDown, true);
      document.addEventListener("mousemove", mouseMove, true);
      document.addEventListener("mouseup", mouseUp, true);
      document.addEventListener("keydown", keydown, true);
      window.__zedWebPreview.__cleanup = cleanup;
    },

    clearActiveOverlay() {
      if (window.__zedWebPreview.__cleanup) {
        window.__zedWebPreview.__cleanup();
      }
    },

    startAnnotationMode() {
      if (window.__zedWebPreview.__cleanup) {
        window.__zedWebPreview.__cleanup();
      }

      const overlay = createOverlay();
      overlay.style.pointerEvents = "auto";
      overlay.style.background = "rgba(0, 0, 0, 0.04)";

      const toolbar = document.createElement("div");
      toolbar.textContent = "Drag to mark. Enter captures. Esc cancels.";
      toolbar.style.position = "fixed";
      toolbar.style.top = "14px";
      toolbar.style.left = "50%";
      toolbar.style.transform = "translateX(-50%)";
      toolbar.style.padding = "7px 10px";
      toolbar.style.border = "1px solid rgba(63, 185, 80, 0.55)";
      toolbar.style.borderRadius = "999px";
      toolbar.style.background = "rgba(13, 17, 23, 0.92)";
      toolbar.style.color = "rgb(240, 246, 252)";
      toolbar.style.font = "12px system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif";
      toolbar.style.boxShadow = "0 8px 24px rgba(0, 0, 0, 0.35)";
      toolbar.style.pointerEvents = "none";
      overlay.appendChild(toolbar);

      const annotations = [];
      let active = null;
      let draft = null;

      const annotationRect = (rect, label) => {
        const box = document.createElement("div");
        box.style.position = "fixed";
        box.style.left = `${rect.x}px`;
        box.style.top = `${rect.y}px`;
        box.style.width = `${rect.width}px`;
        box.style.height = `${rect.height}px`;
        box.style.border = "2px solid #3fb950";
        box.style.background = "rgba(63, 185, 80, 0.16)";
        box.style.borderRadius = "8px";
        box.style.pointerEvents = "none";
        box.style.boxShadow = "0 0 0 1px rgba(13, 17, 23, 0.7), 0 0 22px rgba(63, 185, 80, 0.2)";

        const tag = document.createElement("div");
        tag.textContent = String(label);
        tag.style.position = "absolute";
        tag.style.left = "-2px";
        tag.style.top = "-24px";
        tag.style.minWidth = "20px";
        tag.style.height = "20px";
        tag.style.padding = "0 6px";
        tag.style.borderRadius = "999px";
        tag.style.background = "rgb(63, 185, 80)";
        tag.style.color = "rgb(13, 17, 23)";
        tag.style.font = "700 12px/20px system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif";
        tag.style.textAlign = "center";
        tag.style.boxShadow = "0 4px 12px rgba(0, 0, 0, 0.35)";
        box.appendChild(tag);
        return box;
      };

      const normalizedRect = (start, end) => {
        const left = Math.min(start.x, end.x);
        const top = Math.min(start.y, end.y);
        const width = Math.abs(start.x - end.x);
        const height = Math.abs(start.y - end.y);
        return { x: left, y: top, width, height };
      };

      const setDraftRect = (rect) => {
        if (!draft) {
          draft = annotationRect({ x: 0, y: 0, width: 1, height: 1 }, annotations.length + 1);
          draft.style.borderStyle = "dashed";
          overlay.appendChild(draft);
        }
        draft.style.left = `${rect.x}px`;
        draft.style.top = `${rect.y}px`;
        draft.style.width = `${rect.width}px`;
        draft.style.height = `${rect.height}px`;
      };

      const cleanup = () => {
        document.removeEventListener("mousedown", mouseDown, true);
        document.removeEventListener("mousemove", mouseMove, true);
        document.removeEventListener("mouseup", mouseUp, true);
        document.removeEventListener("keydown", keydown, true);
        overlay.remove();
        window.__zedWebPreview.__cleanup = null;
      };

      const mouseDown = (event) => {
        event.preventDefault();
        event.stopPropagation();
        active = { x: event.clientX, y: event.clientY };
        setDraftRect({ x: active.x, y: active.y, width: 1, height: 1 });
      };

      const mouseMove = (event) => {
        if (!active) return;
        event.preventDefault();
        event.stopPropagation();
        setDraftRect(normalizedRect(active, { x: event.clientX, y: event.clientY }));
      };

      const mouseUp = (event) => {
        if (!active) return;
        event.preventDefault();
        event.stopPropagation();
        const rect = normalizedRect(active, { x: event.clientX, y: event.clientY });
        active = null;
        if (draft) {
          draft.remove();
          draft = null;
        }
        if (rect.width < 6 || rect.height < 6) return;

        const label = annotations.length + 1;
        const box = annotationRect(rect, label);
        overlay.appendChild(box);
        annotations.push({
          id: `annotation-${label}`,
          label,
          rect: {
            x: Math.round(rect.x),
            y: Math.round(rect.y),
            width: Math.round(rect.width),
            height: Math.round(rect.height)
          },
          page: {
            scroll_x: Math.round(window.scrollX || 0),
            scroll_y: Math.round(window.scrollY || 0)
          },
          created_at: new Date().toISOString()
        });
      };

      const keydown = (event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          cleanup();
          return;
        }
        if (event.key !== "Enter") return;

        event.preventDefault();
        event.stopPropagation();
        post({
          kind: "annotated-screenshot",
          timestamp_ms: Date.now(),
          timestamp: new Date().toISOString(),
          url: window.location.href,
          title: document.title,
          scale: window.devicePixelRatio || 1,
          viewport: {
            inner_width: window.innerWidth,
            inner_height: window.innerHeight,
            scroll_x: Math.round(window.scrollX || 0),
            scroll_y: Math.round(window.scrollY || 0),
            device_pixel_ratio: window.devicePixelRatio || 1
          },
          counts: {
            annotations: annotations.length
          },
          annotations
        });
      };

      document.addEventListener("mousedown", mouseDown, true);
      document.addEventListener("mousemove", mouseMove, true);
      document.addEventListener("mouseup", mouseUp, true);
      document.addEventListener("keydown", keydown, true);
      document.documentElement.appendChild(overlay);
      window.__zedWebPreview.__cleanup = cleanup;
    }
  };
})();
"#;

#[cfg(test)]
mod tests {
    use super::{display_title_from_url, normalized_url, slugify};

    #[test]
    fn slugify_strips_extra_separators() {
        assert_eq!(slugify("Friday Web App"), "friday-web-app");
        assert_eq!(slugify("___Preview___"), "preview");
    }

    #[test]
    fn normalize_search_queries_to_google() {
        let url = normalized_url("react devtools extension").unwrap();
        assert_eq!(url.host_str(), Some("www.google.com"));
        assert_eq!(url.path(), "/search");
    }

    #[test]
    fn normalize_missing_scheme_urls() {
        let url = normalized_url("localhost:5173").unwrap();
        assert_eq!(url.host_str(), Some("localhost"));
    }

    #[test]
    fn title_uses_host_name() {
        assert_eq!(
            display_title_from_url("https://www.google.com/"),
            "www.google.com"
        );
    }

    #[test]
    fn title_uses_preview_for_blank_page() {
        assert_eq!(display_title_from_url("about:blank"), "Preview");
    }
}
