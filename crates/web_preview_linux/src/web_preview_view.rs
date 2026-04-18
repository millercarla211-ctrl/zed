use agent_client_protocol as acp;
use agent_ui::AgentPanel;
use anyhow::{Context as _, Result, anyhow};
use base64::Engine as _;
use editor::Editor;
#[cfg(target_os = "linux")]
use gpui::AsyncApp;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use gpui::ImageFormat as GpuiImageFormat;
use gpui::{
    App, AppContext as _, Bounds, ClipboardItem, Context, Entity, EventEmitter, FocusHandle,
    Focusable, Image as GpuiImage, MouseButton, Pixels, Render, SharedString, Subscription, Task,
    WeakEntity, Window, canvas, svg,
};
#[cfg(target_os = "windows")]
use gpui::{AsyncApp, EntityId};
#[cfg(target_os = "linux")]
use gpui_linux::exported_wayland_window_handle;
use menu::Confirm;
use paths::data_dir;
#[cfg(target_os = "linux")]
use paths::home_dir;
use serde_json::Value;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::io::Cursor;
#[cfg(target_os = "windows")]
use std::num::NonZeroIsize;
use std::{
    cell::{Cell, RefCell},
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use ui::{Color, IconButton, IconName, IconSize, Label, LabelSize, prelude::*};
use workspace::item::{Item, ItemEvent, PaneTabBarControls};
use workspace::notifications::NotificationId;
use workspace::{NewWebPreview, Pane, Toast, Workspace, WorkspaceId};

#[cfg(target_os = "windows")]
use crate::windows_visual_webview::WindowsVisualWebView;
use crate::{OpenPreview, OpenPreviewToTheSide};

#[cfg(target_os = "windows")]
use gpui_windows::window_has_focused_webview;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use image::{ImageFormat as ExternalImageFormat, imageops::FilterType};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use image::{RgbaImage, imageops};
#[cfg(target_os = "windows")]
use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
#[cfg(target_os = "linux")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, ClientToScreen,
        CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC,
        GetDIBits, HBITMAP, HGDIOBJ, ReleaseDC, SRCCOPY, SelectObject,
    },
};
#[cfg(target_os = "linux")]
use wry::WebViewBuilderExtUnix;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use wry::{
    PageLoadEvent, Rect as WryRect, WebContext, WebView, WebViewBuilder,
    dpi::{LogicalPosition, LogicalSize, Position, Size},
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

#[derive(Clone, Debug)]
enum PreviewLoadState {
    Ready,
    Error(SharedString),
}

#[derive(Clone, Debug)]
pub(crate) enum BrowserEvent {
    UrlChanged(String),
    TitleChanged(String),
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

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, PartialEq)]
struct LinuxPreviewLayout {
    host_bounds: Bounds<Pixels>,
    webview_bounds: Bounds<Pixels>,
}

#[cfg(target_os = "windows")]
struct NativeWebPreview {
    webview: WindowsVisualWebView,
}

#[cfg(target_os = "linux")]
enum LinuxNativeHost {
    X11Popup(crate::x11_host::X11PreviewHost),
    WaylandPopup(crate::wayland_host::WaylandPreviewHost),
}

#[cfg(target_os = "linux")]
#[derive(Clone, Debug, PartialEq, Eq)]
enum LinuxNativePreviewTarget {
    X11 { parent_xid: u64 },
    Wayland { exported_parent_handle: String },
}

#[cfg(target_os = "linux")]
struct NativeWebPreview {
    _context: Box<WebContext>,
    webview: WebView,
    host: LinuxNativeHost,
    target: LinuxNativePreviewTarget,
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
    host_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    native_preview: Rc<RefCell<Option<NativeWebPreview>>>,
}

pub struct WebPreviewView {
    workspace: WeakEntity<Workspace>,
    workspace_context: PreviewWorkspaceContext,
    focus_handle: FocusHandle,
    url_editor: Entity<Editor>,
    url_editor_focus_handle: FocusHandle,
    page_title: Option<SharedString>,
    active_url: SharedString,
    bookmarks: Vec<String>,
    detected_extensions: Vec<DetectedExtension>,
    extensions_scanned: bool,
    load_state: PreviewLoadState,
    host_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    #[cfg(target_os = "linux")]
    last_applied_bounds: Rc<RefCell<Option<LinuxPreviewLayout>>>,
    native_mount_requested: Rc<Cell<bool>>,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    deferred_ipc_messages: Vec<String>,
    ipc_flush_scheduled: bool,
    event_pump_task: Option<Task<()>>,
    zoom_factor: f64,
    is_active_item: bool,
    #[cfg(target_os = "linux")]
    native_preview_visible: Cell<bool>,
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    native_preview: Rc<RefCell<Option<NativeWebPreview>>>,
    _subscriptions: Vec<Subscription>,
}

impl WebPreviewView {
    pub fn register(workspace: &mut Workspace, _window: &mut Window, _cx: &mut Context<Workspace>) {
        workspace.register_action(move |workspace, _: &NewWebPreview, window, cx| {
            Self::open_in_active_pane(workspace, window, cx);
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
            let current_url = DEFAULT_WEB_PREVIEW_URL.to_string();
            let url_editor = cx.new(|cx| {
                let mut editor = Editor::single_line(window, cx);
                editor.set_placeholder_text("Search Google or enter a URL", window, cx);
                editor.set_text(current_url.as_str(), window, cx);
                editor
            });

            let browser_events = Arc::new(Mutex::new(Vec::new()));
            let mut this = Self {
                workspace: weak_workspace.clone(),
                workspace_context: workspace_context.clone(),
                focus_handle: cx.focus_handle(),
                url_editor_focus_handle: url_editor.focus_handle(cx),
                url_editor,
                page_title: None,
                active_url: current_url.into(),
                bookmarks: load_bookmarks(&workspace_context.profile_dir).unwrap_or_default(),
                detected_extensions: Vec::new(),
                extensions_scanned: false,
                load_state: PreviewLoadState::Ready,
                host_bounds: Rc::new(RefCell::new(None)),
                #[cfg(target_os = "linux")]
                last_applied_bounds: Rc::new(RefCell::new(None)),
                native_mount_requested: Rc::new(Cell::new(false)),
                browser_events,
                deferred_ipc_messages: Vec::new(),
                ipc_flush_scheduled: false,
                event_pump_task: None,
                zoom_factor: 1.0,
                is_active_item: false,
                #[cfg(target_os = "linux")]
                native_preview_visible: Cell::new(false),
                #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
                native_preview: Rc::new(RefCell::new(None)),
                _subscriptions: vec![],
            };
            this.start_event_pump(window, cx);
            this
        })
    }

    #[cfg(not(target_os = "linux"))]
    fn start_event_pump(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = (window, cx);
        self.event_pump_task = None;
    }

    #[cfg(target_os = "linux")]
    fn start_event_pump(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let entity_id = cx.entity().entity_id();
        let browser_events = self.browser_events.clone();
        self.event_pump_task = Some(cx.spawn(move |_this, cx: &mut AsyncApp| async move {
            loop {
                let processed_gtk_events = pump_linux_webview_events();
                let should_notify = processed_gtk_events
                    || browser_events
                        .lock()
                        .map(|events| !events.is_empty())
                        .unwrap_or(false);
                if should_notify {
                    cx.update(|app| app.notify(entity_id));
                }
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
            }
        }));
    }

    fn find_existing_preview_item_idx(
        pane: &Pane,
        view: &Entity<WebPreviewView>,
        cx: &App,
    ) -> Option<usize> {
        let preview_key = view.read(cx).workspace_context.preview_key.clone();
        pane.items_of_type::<WebPreviewView>()
            .find(|candidate| candidate.read(cx).workspace_context.preview_key == preview_key)
            .and_then(|candidate| pane.index_for_item(&candidate))
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
        self.navigate_to_input(window, cx);
        cx.notify();
    }

    #[cfg(target_os = "windows")]
    fn release_native_preview_focus(&self) {
        let borrow = self.native_preview.borrow();
        if let Some(preview) = borrow.as_ref() {
            let _ = preview.webview.focus_parent();
        }
    }

    #[cfg(target_os = "macos")]
    fn release_native_preview_focus(&self) {
        let borrow = self.native_preview.borrow();
        if let Some(preview) = borrow.as_ref() {
            let _ = preview.webview.focus_parent();
        }
    }

    #[cfg(target_os = "linux")]
    fn release_native_preview_focus(&self) {
        let borrow = self.native_preview.borrow();
        if let Some(preview) = borrow.as_ref() {
            let _ = preview.webview.focus_parent();
        }
    }

    #[cfg(target_os = "linux")]
    fn should_focus_native_preview_page(&self, window: &Window) -> bool {
        self.is_active_item
            && window.is_window_active()
            && self.focus_handle.is_focused(window)
            && !self.url_editor_focus_handle.is_focused(window)
    }

    #[cfg(target_os = "linux")]
    fn focus_native_preview_page(&self) {
        let borrow = self.native_preview.borrow();
        if let Some(preview) = borrow.as_ref() {
            let _ = preview.webview.focus();
        }
    }

    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "linux")
    ))]
    fn release_native_preview_focus(&self) {}

    fn activate_url_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus_handle = self.url_editor_focus_handle.clone();
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

    #[cfg(target_os = "linux")]
    fn sync_native_preview_host_bounds(&mut self, window: &mut Window) {
        self.reconcile_linux_native_preview_target(window);
        if self.native_preview.borrow().is_none() {
            window.refresh();
            return;
        }
        let Some(bounds) = self.host_bounds.borrow().as_ref().copied() else {
            return;
        };
        let Some(preview) = self.native_preview.borrow_mut().as_mut() else {
            return;
        };
        let resolved_bounds = linux_preview_layout(preview, window, bounds);
        if self.last_applied_bounds.borrow().as_ref().copied() == Some(resolved_bounds) {
            return;
        }
        let _ = set_linux_native_preview_bounds(preview, resolved_bounds);
        *self.last_applied_bounds.borrow_mut() = Some(resolved_bounds);
    }

    #[cfg(target_os = "linux")]
    fn sync_native_preview_window_activation(&mut self, window: &mut Window) {
        self.reconcile_linux_native_preview_target(window);
        if self.native_preview.borrow().is_none() {
            window.refresh();
            return;
        }
        let Some(preview) = self.native_preview.borrow_mut().as_mut() else {
            return;
        };

        let should_be_visible = self.is_active_item && window.is_window_active();
        if should_be_visible {
            if let Some(bounds) = self.host_bounds.borrow().as_ref().copied() {
                let resolved_bounds = linux_preview_layout(preview, window, bounds);
                if self.last_applied_bounds.borrow().as_ref().copied() != Some(resolved_bounds) {
                    let _ = set_linux_native_preview_bounds(preview, resolved_bounds);
                    *self.last_applied_bounds.borrow_mut() = Some(resolved_bounds);
                }
            }
            if !self.native_preview_visible.replace(true) {
                let _ = set_linux_native_preview_visible(preview, true);
            }
        } else if self.native_preview_visible.replace(false) {
            let _ = set_linux_native_preview_visible(preview, false);
        }

        drop(preview);
        if self.should_focus_native_preview_page(window) {
            self.focus_native_preview_page();
        }
    }

    fn focus_url_editor(
        &mut self,
        _: &gpui::MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_url_editor(window, cx);
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
        } else {
            self.load_state = PreviewLoadState::Ready;
        }
        cx.emit(ItemEvent::UpdateTab);
        cx.notify();
    }

    fn reload(&mut self, _: &gpui::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = self.reload_webview(window, cx) {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
            cx.notify();
        }
    }

    fn toggle_bookmark(
        &mut self,
        _: &gpui::ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    fn go_back(&mut self, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.evaluate_script("history.back();");
        cx.notify();
    }

    fn go_forward(&mut self, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.evaluate_script("history.forward();");
        cx.notify();
    }

    fn zoom_in(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.zoom_factor = (self.zoom_factor + 0.1).min(3.0);
        let _ = self.apply_zoom();
        cx.notify();
    }

    fn zoom_out(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.zoom_factor = (self.zoom_factor - 0.1).max(0.25);
        let _ = self.apply_zoom();
        cx.notify();
    }

    fn open_devtools(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        {
            let opened = {
                let borrow = self.native_preview.borrow();
                if let Some(preview) = borrow.as_ref() {
                    preview.webview.open_devtools();
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

        for event in events {
            match event {
                BrowserEvent::UrlChanged(url) => {
                    let previous_url = self.active_url.to_string();
                    self.active_url = url.clone().into();
                    let editor_focus = self.url_editor.focus_handle(cx);
                    let editor_text = self.current_url_text(cx);
                    let should_sync_editor = !editor_focus.is_focused(window)
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

    fn open_extensions_root(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ensure_extensions_scanned(cx);
        let Some(path) = self
            .detected_extensions
            .first()
            .and_then(|extension| extension.path.parent().map(Path::to_path_buf))
            .or_else(|| {
                let staging_root = self.workspace_context.profile_dir.join("wry_extensions");
                staging_root.exists().then_some(staging_root)
            })
        else {
            self.show_toast("No local browser extensions detected", cx);
            return;
        };

        self.open_extension_location("Browser".into(), path, window, cx);
    }

    fn render_extensions_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_toolbar_action_button(
            "web-preview-extensions-trigger",
            IconName::Blocks,
            true,
            cx.listener(|this, _, window, cx| {
                this.open_extensions_root(window, cx);
            }),
            cx,
        )
    }

    fn ensure_extensions_scanned(&mut self, cx: &mut Context<Self>) {
        if self.extensions_scanned {
            return;
        }

        self.detected_extensions = scan_local_extensions().unwrap_or_default();
        self.extensions_scanned = true;
        cx.notify();
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
        eprintln!("[web-preview] schedule mount");
        let request = NativePreviewMountRequest {
            entity_id: cx.entity().entity_id(),
            parent_window,
            profile_dir: self.workspace_context.profile_dir.clone(),
            initial_url: self.active_url.to_string(),
            zoom_factor: self.zoom_factor,
            scale_factor: window.scale_factor(),
            host_bounds: self.host_bounds.clone(),
            browser_events: self.browser_events.clone(),
            native_preview: self.native_preview.clone(),
        };

        self.event_pump_task = Some(cx.spawn(move |_this, cx: &mut AsyncApp| {
            mount_native_preview(request.clone());
            cx.update(|app| app.notify(request.entity_id));
            async {}
        }));
    }

    #[cfg(target_os = "macos")]
    fn ensure_native_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
        } else if self.should_focus_native_preview_page(window) {
            self.focus_native_preview_page();
        }
    }

    #[cfg(target_os = "linux")]
    fn ensure_native_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.native_preview.borrow().is_some() || self.native_mount_requested.get() {
            self.reconcile_linux_native_preview_target(window);
            if self.native_preview.borrow().is_some() {
                self.sync_native_preview_window_activation(window);
                return;
            }
        }
        if self.native_mount_requested.get() {
            return;
        }
        if self.host_bounds.borrow().is_none() {
            window.refresh();
            return;
        }

        let target = match resolve_linux_native_preview_target(window) {
            Ok(target) => target,
            Err(error) => {
                let message = error.to_string();
                if should_retry_linux_native_preview_mount(&message) {
                    self.native_mount_requested.set(true);
                    self.load_state = PreviewLoadState::Ready;
                    cx.defer_in(window, |this, window, cx| {
                        this.native_mount_requested.set(false);
                        this.ensure_native_preview(window, cx);
                    });
                } else {
                    self.load_state = PreviewLoadState::Error(message.into());
                }
                return;
            }
        };

        self.native_mount_requested.set(true);
        let result = create_native_preview_for_linux_window(
            window,
            self.workspace_context.profile_dir.clone(),
            self.active_url.to_string(),
            self.zoom_factor,
            target,
            self.host_bounds.clone(),
            self.browser_events.clone(),
            self.native_preview.clone(),
        );
        self.native_mount_requested.set(false);

        if let Err(error) = result {
            let message = error.to_string();
            if should_retry_linux_native_preview_mount(&message) {
                self.native_mount_requested.set(true);
                self.load_state = PreviewLoadState::Ready;
                cx.defer_in(window, |this, window, cx| {
                    this.native_mount_requested.set(false);
                    this.ensure_native_preview(window, cx);
                });
            } else {
                self.load_state = PreviewLoadState::Error(message.into());
            }
        } else {
            self.sync_native_preview_window_activation(window);
        }
    }

    #[cfg(target_os = "linux")]
    fn reconcile_linux_native_preview_target(&mut self, window: &mut Window) {
        let target = match resolve_linux_native_preview_target(window) {
            Ok(target) => target,
            Err(error) => {
                let message = error.to_string();
                if self.native_preview.borrow().is_some()
                    && should_retry_linux_native_preview_mount(&message)
                {
                    self.teardown_linux_native_preview();
                    self.load_state = PreviewLoadState::Ready;
                    window.refresh();
                }
                return;
            }
        };

        let mut remount_required = false;
        let should_be_visible = self.native_preview_visible.get();
        {
            let mut preview = self.native_preview.borrow_mut();
            let Some(preview) = preview.as_mut() else {
                return;
            };
            let current_layout = self
                .host_bounds
                .borrow()
                .as_ref()
                .copied()
                .map(|bounds| linux_preview_layout(preview, window, bounds));
            if let Err(error) = sync_linux_native_preview_target(
                preview,
                &target,
                current_layout,
                should_be_visible,
            ) {
                eprintln!("[web-preview-linux] failed to retarget native preview host: {error:#}");
                remount_required = true;
            }
        }

        if remount_required {
            self.teardown_linux_native_preview();
            self.load_state = PreviewLoadState::Ready;
            window.refresh();
        }
    }

    #[cfg(target_os = "linux")]
    fn teardown_linux_native_preview(&mut self) {
        if let Some(preview) = self.native_preview.borrow_mut().take() {
            let _ = set_linux_native_preview_visible(&preview, false);
        }
        self.native_preview_visible.set(false);
        self.last_applied_bounds.borrow_mut().take();
    }

    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "linux")
    ))]
    fn ensure_native_preview(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.load_state = PreviewLoadState::Error(
            "Native web preview underlay support is not available on this platform.".into(),
        );
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn load_url(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.ensure_native_preview(window, cx);
        let mut borrow = self.native_preview.borrow_mut();
        let preview = borrow
            .as_mut()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.load_url(url)?;
        Ok(())
    }

    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "linux")
    ))]
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

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn reload_webview(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.ensure_native_preview(window, cx);
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.reload()?;
        Ok(())
    }

    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "linux")
    ))]
    fn reload_webview(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn evaluate_script(&self, script: &str) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.evaluate_script(script)?;
        Ok(())
    }

    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "linux")
    ))]
    fn evaluate_script(&self, _script: &str) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn apply_zoom(&self) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.zoom(self.zoom_factor)?;
        Ok(())
    }

    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "linux")
    ))]
    fn apply_zoom(&self) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn clear_all_browsing_data(&self) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.clear_all_browsing_data()?;
        Ok(())
    }

    #[cfg(all(
        not(target_os = "windows"),
        not(target_os = "macos"),
        not(target_os = "linux")
    ))]
    fn clear_all_browsing_data(&self) -> Result<()> {
        Err(anyhow!(
            "Native web preview is not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    fn capture_and_store_screenshot(
        &self,
        crop: Option<BrowserRect>,
        window: &Window,
    ) -> Result<PathBuf> {
        #[cfg(target_os = "windows")]
        let mut image = {
            let bounds = *self
                .host_bounds
                .borrow()
                .as_ref()
                .ok_or_else(|| anyhow!("The web preview is not visible yet"))?;
            let rect = screen_rect_for_bounds(window, bounds)?;
            capture_screen_rect(rect)?
        };
        #[cfg(target_os = "linux")]
        let mut image = {
            let preview = self.native_preview.borrow();
            let preview = preview
                .as_ref()
                .ok_or_else(|| anyhow!("The native Linux web preview is not available"))?;
            match &preview.host {
                LinuxNativeHost::X11Popup(host) => host.capture_image()?,
                LinuxNativeHost::WaylandPopup(host) => host.capture_image()?,
            }
        };
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

    #[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
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

    #[cfg(any(target_os = "windows", target_os = "linux"))]
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

    #[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
    fn prepare_agent_png_bytes(&self, png_bytes: Vec<u8>) -> Result<Vec<u8>> {
        Ok(png_bytes)
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    fn capture_screenshot_png_bytes(
        &self,
        crop: Option<BrowserRect>,
        window: &Window,
    ) -> Result<Vec<u8>> {
        let path = self.capture_and_store_screenshot(crop, window)?;
        fs::read(&path)
            .with_context(|| format!("Failed to read screenshot bytes from {}", path.display()))
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
    fn capture_screenshot_png_bytes(
        &self,
        _crop: Option<BrowserRect>,
        _window: &Window,
    ) -> Result<Vec<u8>> {
        Err(anyhow!(
            "Web preview screenshots are not available on this platform"
        ))
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
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

    #[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
    fn capture_screenshot_payload(
        &self,
        _crop: Option<BrowserRect>,
        _window: &Window,
    ) -> Result<(PathBuf, GpuiImage, Vec<acp::ContentBlock>)> {
        Err(anyhow!(
            "Web preview screenshots are not available on this platform"
        ))
    }

    fn render_more_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .items_center()
            .gap_1()
            .child(self.render_toolbar_action_button(
                "web-preview-screenshot",
                IconName::Screen,
                true,
                cx.listener(|this, _, window, cx| {
                    this.take_screenshot(window, cx);
                }),
                cx,
            ))
            .child(self.render_toolbar_action_button(
                "web-preview-inspect",
                IconName::Code,
                true,
                cx.listener(|this, _, window, cx| {
                    this.inspect_element(window, cx);
                }),
                cx,
            ))
            .child(self.render_toolbar_action_button(
                "web-preview-devtools",
                IconName::Terminal,
                true,
                cx.listener(|this, _, window, cx| {
                    this.open_devtools(window, cx);
                }),
                cx,
            ))
            .child(self.render_toolbar_action_button(
                "web-preview-zoom-in",
                IconName::Plus,
                true,
                cx.listener(|this, _, window, cx| {
                    this.zoom_in(window, cx);
                }),
                cx,
            ))
            .child(self.render_toolbar_action_button(
                "web-preview-zoom-out",
                IconName::Dash,
                true,
                cx.listener(|this, _, window, cx| {
                    this.zoom_out(window, cx);
                }),
                cx,
            ))
            .child(self.render_toolbar_action_button(
                "web-preview-clear-cache",
                IconName::Trash,
                true,
                cx.listener(|this, _, window, cx| {
                    this.clear_cache(window, cx);
                }),
                cx,
            ))
    }

    fn render_toolbar_action_button(
        &self,
        id: &'static str,
        icon: IconName,
        enabled: bool,
        on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = cx.theme().colors();
        let group_name = SharedString::from(format!("{id}-group"));

        let icon = svg()
            .size(IconSize::Small.rems())
            .flex_none()
            .path(icon.path())
            .text_color(if enabled {
                colors.icon_muted
            } else {
                colors.icon_disabled
            })
            .when(enabled, |this| {
                this.group_hover(group_name.clone(), |style| {
                    style.text_color(colors.icon_accent)
                })
            });

        let button = h_flex()
            .id(id)
            .group(group_name)
            .flex_none()
            .w_6()
            .h_6()
            .items_center()
            .justify_center()
            .rounded_md()
            .child(icon);

        if enabled {
            button
                .cursor_pointer()
                .hover(|style| style.bg(colors.ghost_element_hover))
                .active(|style| style.bg(colors.ghost_element_active))
                .on_click(on_click)
                .into_any_element()
        } else {
            button.into_any_element()
        }
    }

    fn render_webview_body(&self, cx: &mut Context<Self>) -> AnyElement {
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        {
            let host_bounds = self.host_bounds.clone();
            #[cfg(target_os = "linux")]
            let last_applied_bounds = self.last_applied_bounds.clone();
            #[cfg(target_os = "linux")]
            let is_active_item = self.is_active_item;
            #[cfg(target_os = "linux")]
            let native_mount_requested = self.native_mount_requested.clone();
            let native_preview = self.native_preview.clone();

            let canvas = canvas(
                move |bounds, window, _cx| {
                    *host_bounds.borrow_mut() = Some(bounds);
                    let preview_ready = native_preview.borrow().is_some();
                    if let Some(preview) = native_preview.borrow_mut().as_mut() {
                        #[cfg(target_os = "windows")]
                        {
                            let _ = preview.webview.set_visible(true);
                            let _ = preview.webview.set_bounds(
                                client_rect_for_bounds(bounds, window.scale_factor()),
                                window.scale_factor(),
                            );
                        }

                        #[cfg(any(target_os = "macos", target_os = "linux"))]
                        {
                            #[cfg(target_os = "linux")]
                            let should_update_bounds =
                                linux_native_preview_needs_dynamic_bounds_sync(preview)
                                    || last_applied_bounds.borrow().as_ref().copied()
                                        != Some(linux_preview_layout(preview, window, bounds));
                            #[cfg(target_os = "macos")]
                            let should_update_bounds =
                                last_applied_bounds.borrow().as_ref().copied() != Some(bounds);
                            if should_update_bounds {
                                #[cfg(target_os = "macos")]
                                {
                                    let _ = set_webview_bounds(&preview.webview, bounds);
                                    *last_applied_bounds.borrow_mut() = Some(bounds);
                                }
                                #[cfg(target_os = "linux")]
                                {
                                    let resolved_bounds =
                                        linux_preview_layout(preview, window, bounds);
                                    let _ =
                                        set_linux_native_preview_bounds(preview, resolved_bounds);
                                    *last_applied_bounds.borrow_mut() = Some(resolved_bounds);
                                }
                            }
                        }
                    }
                    #[cfg(target_os = "linux")]
                    let should_allow_passthrough =
                        preview_ready && is_active_item && window.is_window_active();
                    #[cfg(target_os = "windows")]
                    let should_allow_passthrough = preview_ready;
                    #[cfg(target_os = "linux")]
                    if !preview_ready && !native_mount_requested.get() {
                        window.refresh();
                    }
                    if should_allow_passthrough {
                        let passthrough_hitbox =
                            window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal);
                        window.insert_mouse_passthrough_region(&passthrough_hitbox);
                    }
                    bounds
                },
                |_bounds, _state, _window, _cx| {},
            )
            .size_full();

            #[cfg(target_os = "windows")]
            {
                let _ = cx;
                return canvas.into_any_element();
            }

            #[cfg(not(target_os = "windows"))]
            {
                let _ = cx;
                return canvas.into_any_element();
            }
        }

        #[cfg(all(
            not(target_os = "windows"),
            not(target_os = "macos"),
            not(target_os = "linux")
        ))]
        {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    Label::new("Web Preview native embedding is not available on this platform.")
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

impl Item for WebPreviewView {
    type Event = ItemEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        format!("Web {}", self.current_tab_title()).into()
    }

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<ui::Icon> {
        Some(ui::Icon::new(IconName::Public))
    }

    fn tab_tooltip_text(&self, _cx: &App) -> Option<SharedString> {
        (!self.active_url.trim().is_empty()).then(|| self.active_url.clone())
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("Web Preview Opened")
    }

    fn requires_transparent_workspace_background() -> bool {
        true
    }

    fn show_toolbar(&self) -> bool {
        false
    }

    fn pane_tab_bar_controls(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<PaneTabBarControls> {
        // Return None to keep default tab navigation arrows
        // Web navigation is handled in the preview's own toolbar
        None
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

            let browser_events = Arc::new(Mutex::new(Vec::new()));
            let mut this = Self {
                workspace,
                workspace_context,
                focus_handle: cx.focus_handle(),
                url_editor_focus_handle: url_editor.focus_handle(cx),
                url_editor,
                page_title: None,
                active_url: current_url.clone().into(),
                bookmarks,
                detected_extensions,
                extensions_scanned: self.extensions_scanned,
                load_state: PreviewLoadState::Ready,
                host_bounds: Rc::new(RefCell::new(None)),
                #[cfg(any(target_os = "macos", target_os = "linux"))]
                last_applied_bounds: Rc::new(RefCell::new(None)),
                native_mount_requested: Rc::new(Cell::new(false)),
                browser_events,
                deferred_ipc_messages: Vec::new(),
                ipc_flush_scheduled: false,
                event_pump_task: None,
                zoom_factor: self.zoom_factor,
                is_active_item: false,
                #[cfg(target_os = "linux")]
                native_preview_visible: Cell::new(false),
                #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
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
        #[cfg(target_os = "linux")]
        self._subscriptions
            .push(cx.observe_window_bounds(window, |this, window, _cx| {
                this.sync_native_preview_host_bounds(window);
            }));
        #[cfg(target_os = "linux")]
        self._subscriptions
            .push(cx.observe_window_activation(window, |this, window, _cx| {
                this.sync_native_preview_window_activation(window);
            }));

        // Defer focus to ensure webview doesn't interfere
        let focus_handle = self.url_editor.focus_handle(cx);
        cx.defer_in(window, move |_, window, cx| {
            focus_handle.focus(window, cx);
        });
    }

    fn deactivated(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.is_active_item = false;
        // Hide webview when tab is deactivated
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        if let Some(preview) = self.native_preview.borrow_mut().as_mut() {
            #[cfg(target_os = "linux")]
            let _ = set_linux_native_preview_visible(preview, false);
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            let _ = preview.webview.set_visible(false);
        }
        #[cfg(target_os = "linux")]
        self.native_preview_visible.set(false);
        #[cfg(target_os = "linux")]
        self.last_applied_bounds.borrow_mut().take();
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        _window.set_background_appearance(gpui::WindowBackgroundAppearance::Opaque);
    }

    fn workspace_deactivated(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.is_active_item = false;
        #[cfg(target_os = "linux")]
        if let Some(preview) = self.native_preview.borrow_mut().as_mut() {
            let _ = set_linux_native_preview_visible(preview, false);
        }
        #[cfg(target_os = "linux")]
        self.native_preview_visible.set(false);
        #[cfg(target_os = "linux")]
        self.last_applied_bounds.borrow_mut().take();
        _window.set_background_appearance(gpui::WindowBackgroundAppearance::Opaque);
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

        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        {
            window.set_background_appearance(gpui::WindowBackgroundAppearance::Transparent);
            self.ensure_native_preview(window, cx);
            #[cfg(target_os = "linux")]
            self.sync_native_preview_window_activation(window);
        }

        let body = self.render_webview_body(cx);
        let error_message = match &self.load_state {
            PreviewLoadState::Ready => None,
            PreviewLoadState::Error(error) => Some(error.clone()),
        };
        let is_bookmarked = self.is_active_url_bookmarked();
        let bookmark_icon = if is_bookmarked {
            IconName::StarFilled
        } else {
            IconName::Star
        };
        let bookmark_color = if is_bookmarked {
            Color::Accent
        } else {
            Color::Muted
        };
        #[cfg(target_os = "windows")]
        let preview_surface_background = gpui::transparent_black().alpha(1.0 / 255.0);
        #[cfg(not(target_os = "windows"))]
        let preview_surface_background = gpui::transparent_black();

        div()
            .id("web-preview")
            .key_context("WebPreview")
            .on_action(cx.listener(Self::confirm_navigation))
            .track_focus(&self.focus_handle(cx))
            .size_full()
            .overflow_hidden()
            .child(
                v_flex()
                    .size_full()
                    .child(
                        h_flex()
                            .id("web-preview-toolbar")
                            .relative()
                            .flex_none()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_2()
                            .border_b_1()
                            .border_color(cx.theme().colors().border_variant)
                            .bg(cx.theme().colors().surface_background)
                            .child(
                                IconButton::new("web-preview-back", IconName::ArrowLeft)
                                    .icon_size(IconSize::Small)
                                    .on_click(cx.listener(Self::go_back)),
                            )
                            .child(
                                IconButton::new("web-preview-forward", IconName::ArrowRight)
                                    .icon_size(IconSize::Small)
                                    .on_click(cx.listener(Self::go_forward)),
                            )
                            .child(
                                IconButton::new("web-preview-reload", IconName::RotateCw)
                                    .icon_size(IconSize::Small)
                                    .on_click(cx.listener(Self::reload)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .h_8()
                                    .min_w_0()
                                    .px_3()
                                    .occlude()
                                    .rounded_full()
                                    .bg(cx.theme().colors().editor_background)
                                    .border_1()
                                    .border_color(cx.theme().colors().border_variant)
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(Self::focus_url_editor),
                                    )
                                    .child(
                                        h_flex()
                                            .size_full()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .child(self.url_editor.clone()),
                                            )
                                            .child(
                                                IconButton::new(
                                                    "web-preview-bookmark",
                                                    bookmark_icon,
                                                )
                                                .icon_size(IconSize::Small)
                                                .icon_color(Color::Muted)
                                                .toggle_state(is_bookmarked)
                                                .selected_icon_color(bookmark_color)
                                                .on_click(cx.listener(Self::toggle_bookmark)),
                                            ),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_center()
                                    .gap_1()
                                    .flex_none()
                                    .child(self.render_extensions_menu(cx))
                                    .child(self.render_more_menu(cx)),
                            ),
                    )
                    .child(
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
    eprintln!("[web-preview] deferred mount start");
    let result = catch_unwind(AssertUnwindSafe(|| {
        create_native_preview_for_request(&request)
    }));

    match result {
        Ok(Ok(())) => eprintln!("[web-preview] deferred mount ok"),
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

    eprintln!("[web-preview] create request prepare");
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

    eprintln!("[web-preview] create request build");
    let webview = WindowsVisualWebView::new(
        main_window,
        request.profile_dir.clone(),
        url.as_str(),
        request.zoom_factor,
        request.scale_factor,
        initial_bounds,
        request.browser_events.clone(),
    )
    .with_context(|| "Failed to build the embedded web preview")?;

    *request.native_preview.borrow_mut() = Some(NativeWebPreview { webview });
    eprintln!("[web-preview] create request done");

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

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LinuxWindowSystem {
    X11,
    Wayland,
}

#[cfg(target_os = "linux")]
fn create_native_preview_for_linux_window(
    window: &Window,
    profile_dir: PathBuf,
    initial_url: String,
    zoom_factor: f64,
    target: LinuxNativePreviewTarget,
    host_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    native_preview: Rc<RefCell<Option<NativeWebPreview>>>,
) -> Result<()> {
    if native_preview.borrow().is_some() {
        return Ok(());
    }

    let window_system = linux_native_preview_target_window_system(&target);
    ensure_linux_webview_runtime(window_system)?;

    match target {
        LinuxNativePreviewTarget::X11 { parent_xid } => create_native_preview_for_linux_x11_window(
            window,
            parent_xid,
            profile_dir,
            initial_url,
            zoom_factor,
            host_bounds,
            browser_events,
            native_preview,
        ),
        LinuxNativePreviewTarget::Wayland {
            exported_parent_handle,
        } => create_native_preview_for_linux_wayland_window(
            window,
            exported_parent_handle,
            profile_dir,
            initial_url,
            zoom_factor,
            host_bounds,
            browser_events,
            native_preview,
        ),
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_window_system(window: &Window) -> Result<LinuxWindowSystem> {
    match HasWindowHandle::window_handle(window)?.as_raw() {
        RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_) => Ok(LinuxWindowSystem::X11),
        RawWindowHandle::Wayland(_) => Ok(LinuxWindowSystem::Wayland),
        _ => Err(anyhow!(
            "Unsupported Linux window handle for native web preview"
        )),
    }
}

#[cfg(target_os = "linux")]
fn x11_parent_window_id(window: &Window) -> Result<u64> {
    match HasWindowHandle::window_handle(window)?.as_raw() {
        RawWindowHandle::Xlib(handle) => Ok(handle.window),
        RawWindowHandle::Xcb(handle) => Ok(handle.window.into()),
        _ => Err(anyhow!(
            "The Linux window is not using an X11 window handle for native web preview"
        )),
    }
}

#[cfg(target_os = "linux")]
fn wayland_parent_exported_handle(window: &Window) -> Result<String> {
    let handle = HasWindowHandle::window_handle(window)?;
    let RawWindowHandle::Wayland(raw_handle) = handle.as_raw() else {
        return Err(anyhow!(
            "The Linux window is not using a Wayland surface for web preview embedding"
        ));
    };

    exported_wayland_window_handle(raw_handle.surface.as_ptr()).ok_or_else(|| {
        anyhow!("The GPUI Wayland window does not have an exported xdg-foreign parent handle yet")
    })
}

#[cfg(target_os = "linux")]
fn resolve_linux_native_preview_target(window: &Window) -> Result<LinuxNativePreviewTarget> {
    match detect_linux_window_system(window)? {
        LinuxWindowSystem::X11 => Ok(LinuxNativePreviewTarget::X11 {
            parent_xid: x11_parent_window_id(window)?,
        }),
        LinuxWindowSystem::Wayland => Ok(LinuxNativePreviewTarget::Wayland {
            exported_parent_handle: wayland_parent_exported_handle(window)?,
        }),
    }
}

#[cfg(target_os = "linux")]
fn linux_native_preview_target_window_system(
    target: &LinuxNativePreviewTarget,
) -> LinuxWindowSystem {
    match target {
        LinuxNativePreviewTarget::X11 { .. } => LinuxWindowSystem::X11,
        LinuxNativePreviewTarget::Wayland { .. } => LinuxWindowSystem::Wayland,
    }
}

#[cfg(target_os = "linux")]
fn create_native_preview_for_linux_x11_window(
    window: &Window,
    parent_xid: u64,
    profile_dir: PathBuf,
    initial_url: String,
    zoom_factor: f64,
    host_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    native_preview: Rc<RefCell<Option<NativeWebPreview>>>,
) -> Result<()> {
    fs::create_dir_all(&profile_dir)
        .with_context(|| "Failed to prepare the Web Preview profile directory")?;

    let url = normalized_url(&initial_url)?;
    let event_queue = browser_events.clone();
    let mut web_context = Box::new(WebContext::new(Some(profile_dir)));
    let initial_layout = host_bounds
        .borrow()
        .as_ref()
        .copied()
        .map(|bounds| linux_x11_preview_layout(window, bounds));
    let host = crate::x11_host::X11PreviewHost::new(
        parent_xid,
        initial_layout.map(|layout| layout.host_bounds),
        initial_layout.map(|layout| layout.webview_bounds),
    )?;

    let initial_bounds = host_bounds
        .borrow()
        .as_ref()
        .copied()
        .map(bounds_to_linux_popup_rect)
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
        .build_gtk(host.container())
        .with_context(|| "Failed to build the embedded Linux web preview")?;

    webview.zoom(zoom_factor)?;

    let preview = NativeWebPreview {
        _context: web_context,
        webview,
        host: LinuxNativeHost::X11Popup(host),
        target: LinuxNativePreviewTarget::X11 { parent_xid },
    };
    if let Some(bounds) = host_bounds
        .borrow()
        .as_ref()
        .copied()
        .map(|bounds| linux_x11_preview_layout(window, bounds))
    {
        let _ = set_linux_native_preview_bounds(&preview, bounds);
    }

    *native_preview.borrow_mut() = Some(preview);

    Ok(())
}

#[cfg(target_os = "linux")]
fn create_native_preview_for_linux_wayland_window(
    window: &Window,
    exported_parent_handle: String,
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

    let initial_layout = host_bounds
        .borrow()
        .as_ref()
        .copied()
        .map(|bounds| linux_wayland_preview_layout(window, bounds));
    let host = crate::wayland_host::WaylandPreviewHost::new(
        exported_parent_handle.as_str(),
        initial_layout.map(|layout| layout.host_bounds),
        initial_layout.map(|layout| layout.webview_bounds),
    )?;
    let url = normalized_url(&initial_url)?;
    let event_queue = browser_events.clone();
    let mut web_context = Box::new(WebContext::new(Some(profile_dir)));

    let initial_bounds = host_bounds
        .borrow()
        .as_ref()
        .copied()
        .map(bounds_to_linux_popup_rect)
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
        .build_gtk(host.container())
        .with_context(|| "Failed to build the embedded Linux Wayland web preview")?;

    webview.zoom(zoom_factor)?;

    let preview = NativeWebPreview {
        _context: web_context,
        webview,
        host: LinuxNativeHost::WaylandPopup(host),
        target: LinuxNativePreviewTarget::Wayland {
            exported_parent_handle,
        },
    };
    if let Some(bounds) = host_bounds
        .borrow()
        .as_ref()
        .copied()
        .map(|bounds| linux_wayland_preview_layout(window, bounds))
    {
        let _ = set_linux_native_preview_bounds(&preview, bounds);
    }

    *native_preview.borrow_mut() = Some(preview);

    Ok(())
}

#[cfg(target_os = "linux")]
fn should_retry_linux_native_preview_mount(message: &str) -> bool {
    message
        .contains("The GPUI Wayland window does not have an exported xdg-foreign parent handle yet")
}

#[cfg(target_os = "linux")]
static GTK_WEBVIEW_RUNTIME: OnceLock<std::result::Result<(), String>> = OnceLock::new();

#[cfg(target_os = "linux")]
fn ensure_linux_webview_runtime(window_system: LinuxWindowSystem) -> Result<()> {
    GTK_WEBVIEW_RUNTIME
        .get_or_init(|| {
            let allowed_backends = match window_system {
                LinuxWindowSystem::X11 => "x11",
                LinuxWindowSystem::Wayland => "wayland",
            };
            gtk::gdk::set_allowed_backends(allowed_backends);
            gtk::init()
                .map_err(|error| format!("Failed to initialize GTK for Linux web preview: {error}"))
        })
        .as_ref()
        .map_err(|error| anyhow!(error.clone()))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn set_linux_native_preview_visible(preview: &NativeWebPreview, visible: bool) -> Result<()> {
    match &preview.host {
        LinuxNativeHost::X11Popup(host) => {
            if visible {
                host.set_visible(true);
                preview.webview.set_visible(true)?;
            } else {
                let _ = preview.webview.focus_parent();
                preview.webview.set_visible(false)?;
                host.set_visible(false);
            }
        }
        LinuxNativeHost::WaylandPopup(host) => {
            if visible {
                host.set_visible(true);
                preview.webview.set_visible(true)?;
            } else {
                let _ = preview.webview.focus_parent();
                preview.webview.set_visible(false)?;
                host.set_visible(false);
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn linux_native_preview_needs_dynamic_bounds_sync(preview: &NativeWebPreview) -> bool {
    matches!(
        preview.host,
        LinuxNativeHost::X11Popup(_) | LinuxNativeHost::WaylandPopup(_)
    )
}

#[cfg(target_os = "linux")]
fn linux_x11_preview_layout(window: &Window, local_bounds: Bounds<Pixels>) -> LinuxPreviewLayout {
    let parent_bounds = window.inner_window_bounds().get_bounds();
    LinuxPreviewLayout {
        host_bounds: Bounds::new(
            gpui::point(
                parent_bounds.origin.x + local_bounds.origin.x,
                parent_bounds.origin.y + local_bounds.origin.y,
            ),
            local_bounds.size,
        ),
        webview_bounds: Bounds::new(gpui::point(Pixels::ZERO, Pixels::ZERO), local_bounds.size),
    }
}

#[cfg(target_os = "linux")]
fn linux_wayland_preview_layout(
    window: &Window,
    local_bounds: Bounds<Pixels>,
) -> LinuxPreviewLayout {
    LinuxPreviewLayout {
        host_bounds: window.inner_window_bounds().get_bounds(),
        webview_bounds: local_bounds,
    }
}

#[cfg(target_os = "linux")]
fn linux_preview_layout(
    preview: &NativeWebPreview,
    window: &Window,
    local_bounds: Bounds<Pixels>,
) -> LinuxPreviewLayout {
    match &preview.host {
        LinuxNativeHost::X11Popup(_) => linux_x11_preview_layout(window, local_bounds),
        LinuxNativeHost::WaylandPopup(_) => linux_wayland_preview_layout(window, local_bounds),
    }
}

#[cfg(target_os = "linux")]
fn set_linux_native_preview_bounds(
    preview: &NativeWebPreview,
    layout: LinuxPreviewLayout,
) -> Result<()> {
    match &preview.host {
        LinuxNativeHost::X11Popup(host) => {
            host.set_layout(layout.host_bounds, layout.webview_bounds);
            preview
                .webview
                .set_bounds(bounds_to_linux_popup_rect(layout.webview_bounds))?;
        }
        LinuxNativeHost::WaylandPopup(host) => {
            host.set_layout(layout.host_bounds, layout.webview_bounds);
            preview
                .webview
                .set_bounds(bounds_to_linux_popup_rect(layout.webview_bounds))?;
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn sync_linux_native_preview_target(
    preview: &mut NativeWebPreview,
    target: &LinuxNativePreviewTarget,
    layout: Option<LinuxPreviewLayout>,
    visible: bool,
) -> Result<()> {
    if &preview.target == target {
        return Ok(());
    }

    match (&preview.host, target) {
        (LinuxNativeHost::X11Popup(host), LinuxNativePreviewTarget::X11 { parent_xid }) => {
            host.set_parent_xid(*parent_xid)?;
        }
        (
            LinuxNativeHost::WaylandPopup(host),
            LinuxNativePreviewTarget::Wayland {
                exported_parent_handle,
            },
        ) => {
            host.set_exported_parent_handle(exported_parent_handle)?;
        }
        _ => {
            return Err(anyhow!(
                "The Linux web preview host kind changed and must be remounted"
            ));
        }
    }

    preview.target = target.clone();
    if let Some(layout) = layout {
        set_linux_native_preview_bounds(preview, layout)?;
    }
    if visible {
        set_linux_native_preview_visible(preview, true)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn bounds_to_linux_popup_rect(bounds: Bounds<Pixels>) -> WryRect {
    WryRect {
        position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
        size: Size::Logical(LogicalSize::new(
            f64::from(bounds.size.width.max(Pixels::from(1.0))),
            f64::from(bounds.size.height.max(Pixels::from(1.0))),
        )),
    }
}

#[cfg(target_os = "linux")]
fn pump_linux_webview_events() -> bool {
    let Some(runtime_status) = GTK_WEBVIEW_RUNTIME.get() else {
        return false;
    };
    if runtime_status.is_err() {
        return false;
    }

    let mut processed_any = false;
    while gtk::events_pending() {
        processed_any = true;
        gtk::main_iteration_do(false);
    }
    processed_any
}

fn parse_browser_rect(value: &Value) -> Option<BrowserRect> {
    Some(BrowserRect {
        x: value.get("x")?.as_f64()?,
        y: value.get("y")?.as_f64()?,
        width: value.get("width")?.as_f64()?,
        height: value.get("height")?.as_f64()?,
    })
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

    #[cfg(target_os = "linux")]
    {
        let home = home_dir();
        let config_home = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        let flatpak_config_home = home.join(".var").join("app");
        let firefox_root = home.join(".mozilla").join("firefox");

        let chromium_sources = [
            ("Chrome", config_home.join("google-chrome"), true),
            ("Chrome Beta", config_home.join("google-chrome-beta"), true),
            (
                "Chrome Dev",
                config_home.join("google-chrome-unstable"),
                true,
            ),
            ("Chromium", config_home.join("chromium"), true),
            ("Edge", config_home.join("microsoft-edge"), true),
            (
                "Brave",
                config_home.join("BraveSoftware").join("Brave-Browser"),
                true,
            ),
            ("Vivaldi", config_home.join("vivaldi"), true),
            ("Opera", config_home.join("opera"), true),
            (
                "Chrome (Flatpak)",
                flatpak_config_home
                    .join("com.google.Chrome")
                    .join("config")
                    .join("google-chrome"),
                true,
            ),
            (
                "Chromium (Flatpak)",
                flatpak_config_home
                    .join("org.chromium.Chromium")
                    .join("config")
                    .join("chromium"),
                true,
            ),
            (
                "Edge (Flatpak)",
                flatpak_config_home
                    .join("com.microsoft.Edge")
                    .join("config")
                    .join("microsoft-edge"),
                true,
            ),
            (
                "Brave (Flatpak)",
                flatpak_config_home
                    .join("com.brave.Browser")
                    .join("config")
                    .join("BraveSoftware")
                    .join("Brave-Browser"),
                true,
            ),
            (
                "Vivaldi (Flatpak)",
                flatpak_config_home
                    .join("com.vivaldi.Vivaldi")
                    .join("config")
                    .join("vivaldi"),
                true,
            ),
        ];

        for (browser, path, chromium_compatible) in chromium_sources {
            scan_linux_chromium_browser_profiles(
                browser,
                &path,
                chromium_compatible,
                &mut extensions,
            )?;
        }

        let firefox_sources = [
            ("Firefox", firefox_root.clone()),
            (
                "Firefox (Flatpak)",
                flatpak_config_home
                    .join("org.mozilla.firefox")
                    .join(".mozilla")
                    .join("firefox"),
            ),
        ];

        for (browser, path) in firefox_sources {
            scan_firefox_extensions(browser, &path, &mut extensions)?;
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

#[cfg(target_os = "linux")]
fn scan_linux_chromium_browser_profiles(
    browser: &str,
    root: &Path,
    chromium_compatible: bool,
    extensions: &mut Vec<DetectedExtension>,
) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }

    let direct_extensions_root = root.join("Extensions");
    if direct_extensions_root.is_dir() {
        scan_chromium_extensions(
            browser,
            &direct_extensions_root,
            chromium_compatible,
            extensions,
        )?;
    }

    for profile_dir in fs::read_dir(root)? {
        let profile_dir = profile_dir?;
        let profile_path = profile_dir.path();
        if !profile_path.is_dir() {
            continue;
        }

        let Some(profile_name) = profile_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !is_linux_chromium_profile_dir(profile_name) {
            continue;
        }

        let extensions_root = profile_path.join("Extensions");
        scan_chromium_extensions(browser, &extensions_root, chromium_compatible, extensions)?;
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn is_linux_chromium_profile_dir(name: &str) -> bool {
    name == "Default"
        || name == "Guest Profile"
        || name == "System Profile"
        || name.starts_with("Profile ")
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
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

#[cfg(any(target_os = "windows", target_os = "linux"))]
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

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn set_webview_bounds(webview: &WebView, bounds: Bounds<Pixels>) -> Result<()> {
    let rect = bounds_to_wry_rect(bounds);
    webview.set_bounds(rect)?;
    // Webview stays visible - no hiding
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn bounds_to_wry_rect(bounds: Bounds<Pixels>) -> WryRect {
    WryRect {
        position: Position::Logical(LogicalPosition::new(
            f64::from(bounds.origin.x),
            f64::from(bounds.origin.y),
        )),
        size: Size::Logical(LogicalSize::new(
            f64::from(bounds.size.width.max(Pixels::from(1.0))),
            f64::from(bounds.size.height.max(Pixels::from(1.0))),
        )),
    }
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

#[cfg(any(target_os = "windows", target_os = "linux"))]
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

  window.__zedWebPreview = {
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
