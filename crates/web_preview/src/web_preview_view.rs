use agent_ui::AgentPanel;
use anyhow::{Context as _, Result, anyhow};
use editor::Editor;
use gpui::{
    App, AppContext as _, AsyncApp, Bounds, ClipboardItem, Context, Entity, EntityId,
    EventEmitter, FocusHandle, Focusable, Pixels, Render, SharedString, Subscription, Task,
    WeakEntity, Window, canvas,
};
use menu::Confirm;
use paths::data_dir;
use serde_json::Value;
use std::{
    cell::{Cell, RefCell},
    fs,
    num::NonZeroIsize,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use ui::{
    Color, ContextMenu, IconButton, IconName, IconSize, Label, LabelSize, PopoverMenuHandle,
    Tooltip, prelude::*,
};
use workspace::item::{Item, ItemEvent, PaneTabBarControls};
use workspace::notifications::NotificationId;
use workspace::{NewWebPreview, Pane, SplitDirection, Toast, ToggleZoom, Workspace, WorkspaceId};

use crate::{OpenPreview, OpenPreviewToTheSide};

#[cfg(target_os = "windows")]
use image::{RgbaImage, imageops};
#[cfg(target_os = "windows")]
use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap,
        CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, HBITMAP,
        HGDIOBJ, ReleaseDC, SRCCOPY, SelectObject, ClientToScreen,
    },
    UI::WindowsAndMessaging::{
        FindWindowExW, GWL_STYLE, GetWindowLongPtrW, HWND_BOTTOM, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowLongPtrW,
        SetWindowPos, ShowWindow, WS_CLIPCHILDREN, WS_CLIPSIBLINGS,
    },
};
#[cfg(target_os = "windows")]
use wry::{
    PageLoadEvent, Rect as WryRect, WebContext, WebView, WebViewBuilder,
    dpi::{LogicalPosition, LogicalSize, Position, Size},
};
#[cfg(target_os = "windows")]
use wry::WebViewBuilderExtWindows;

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
enum BrowserEvent {
    UrlChanged(String),
    TitleChanged(String),
    IpcMessage(String),
    MountFailed(String),
}

#[derive(Clone, Debug)]
enum BrowserAgentPayload {
    Screenshot {
        path: PathBuf,
        url: String,
        kind: &'static str,
    },
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
    _context: Box<WebContext>,
    webview: WebView,
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
    host_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    native_preview: Rc<RefCell<Option<NativeWebPreview>>>,
}

pub struct WebPreviewView {
    workspace: WeakEntity<Workspace>,
    workspace_context: PreviewWorkspaceContext,
    focus_handle: FocusHandle,
    url_editor: Entity<Editor>,
    page_title: Option<SharedString>,
    active_url: SharedString,
    bookmarks: Vec<String>,
    detected_extensions: Vec<DetectedExtension>,
    load_state: PreviewLoadState,
    host_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    last_applied_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    native_mount_requested: Rc<Cell<bool>>,
    browser_events: Arc<Mutex<Vec<BrowserEvent>>>,
    deferred_ipc_messages: Vec<String>,
    ipc_flush_scheduled: bool,
    extensions_menu_handle: PopoverMenuHandle<ContextMenu>,
    more_menu_handle: PopoverMenuHandle<ContextMenu>,
    event_pump_task: Option<Task<()>>,
    zoom_factor: f64,
    #[cfg(target_os = "windows")]
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
                url_editor,
                page_title: None,
                active_url: current_url.into(),
                bookmarks: load_bookmarks(&workspace_context.profile_dir).unwrap_or_default(),
                detected_extensions: scan_local_extensions().unwrap_or_default(),
                load_state: PreviewLoadState::Ready,
                host_bounds: Rc::new(RefCell::new(None)),
                last_applied_bounds: Rc::new(RefCell::new(None)),
                native_mount_requested: Rc::new(Cell::new(false)),
                browser_events,
                deferred_ipc_messages: Vec::new(),
                ipc_flush_scheduled: false,
                extensions_menu_handle: Default::default(),
                more_menu_handle: Default::default(),
                event_pump_task: None,
                zoom_factor: 1.0,
                #[cfg(target_os = "windows")]
                native_preview: Rc::new(RefCell::new(None)),
                _subscriptions: vec![],
            };
            this.start_event_pump(window, cx);
            this
        })
    }

    fn start_event_pump(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = (window, cx);
        self.event_pump_task = None;
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
            .or_else(|| workspace.database_id().map(|id| format!("workspace-{id:?}")))
            .unwrap_or_else(|| "workspace".to_string());

        PreviewWorkspaceContext {
            workspace_id: workspace.database_id(),
            root_path,
            root_name: root_name.clone().into(),
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

    fn should_disable_webview_input(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        // Disable webview input when URL editor has focus
        self.url_editor.focus_handle(cx).is_focused(window)
    }

    fn confirm_navigation(
        &mut self,
        _: &Confirm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.navigate_to_input(window, cx);
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

    fn open_in_browser(
        &mut self,
        _: &gpui::ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Ok(url) = normalized_url(&self.current_url_text(cx)) {
            cx.open_url(url.as_str());
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

        let message = if let Some(index) = self.bookmarks.iter().position(|bookmark| bookmark == &url)
        {
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

    fn go_forward(
        &mut self,
        _: &gpui::ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

    fn reset_zoom(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.zoom_factor = 1.0;
        let _ = self.apply_zoom();
        cx.notify();
    }

    fn copy_current_url(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(self.active_url.to_string()));
        self.show_toast("Copied current URL", cx);
    }

    fn open_devtools(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(target_os = "windows")]
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
        if self.evaluate_script(script).is_ok() {
            self.show_toast("Click an element in the page to send it to the agent.", cx);
        }
    }

    fn hard_reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.evaluate_script(
            "window.location.reload(); if (window.performance && performance.clearResourceTimings) { performance.clearResourceTimings(); }",
        );
        let _ = self.reload_webview(window, cx);
    }

    fn clear_browsing_history(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = self.clear_all_browsing_data() {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        } else {
            self.show_toast("Cleared browsing history", cx);
        }
        cx.notify();
    }

    fn clear_cache(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = self.clear_all_browsing_data() {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        } else {
            self.show_toast("Cleared browser cache", cx);
        }
        cx.notify();
    }

    fn clear_cookies(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = self.delete_all_cookies() {
            self.load_state = PreviewLoadState::Error(error.to_string().into());
        } else {
            self.show_toast("Cleared cookies", cx);
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
                    format!(
                        "Failed to open extension path {}",
                        extension_path.display()
                    )
                    .into(),
                );
            }
        }
    }

    fn take_screenshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.capture_and_store_screenshot(None, window) {
            Ok(path) => {
                self.send_to_agent_panel(
                    BrowserAgentPayload::Screenshot {
                        path,
                        url: self.active_url.to_string(),
                        kind: "Viewport",
                    },
                    window,
                    cx,
                );
                self.show_toast("Captured web preview screenshot", cx);
            }
            Err(error) => {
                self.load_state = PreviewLoadState::Error(error.to_string().into());
            }
        }
        cx.notify();
    }

    fn capture_area_screenshot(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let script = "window.__zedWebPreview && window.__zedWebPreview.captureAreaScreenshot();";
        if self.evaluate_script(script).is_ok() {
            self.show_toast("Drag an area on the page to capture it.", cx);
        }
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
                    self.active_url = url.clone().into();
                    self.url_editor.update(cx, |editor, cx| {
                        editor.set_text(url.as_str(), window, cx);
                    });
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
                    tag: payload.get("tag").and_then(Value::as_str).map(ToOwned::to_owned),
                    id: payload.get("id").and_then(Value::as_str).map(ToOwned::to_owned),
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
                    src: payload.get("src").and_then(Value::as_str).map(ToOwned::to_owned),
                    rect: payload.get("rect").and_then(parse_browser_rect),
                    html: payload
                        .get("html")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                };
                self.send_to_agent_panel(data, window, cx);
                self.show_toast("Sent inspected element to the agent panel", cx);
            }
            "capture-area" => {
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
                let path = self.capture_and_store_screenshot(Some(crop), window)?;
                self.send_to_agent_panel(
                    BrowserAgentPayload::Screenshot {
                        path,
                        url: self.active_url.to_string(),
                        kind: "Area",
                    },
                    window,
                    cx,
                );
                self.show_toast("Captured selected area to the agent panel", cx);
            }
            _ => {}
        }

        Ok(())
    }

    fn send_to_agent_panel(
        &mut self,
        payload: BrowserAgentPayload,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        let note = format_agent_note(payload);
        let window_handle = Window::window_handle(window);

        let _ = cx.update_window(window_handle, |_, window, cx| {
            workspace.update(cx, |workspace, cx| {
                if workspace.panel::<AgentPanel>(cx).is_none() {
                    workspace.open_panel::<AgentPanel>(window, cx);
                }

                let Some(panel) = workspace
                    .focus_panel::<AgentPanel>(window, cx)
                    .or_else(|| workspace.panel::<AgentPanel>(cx))
                else {
                    return;
                };

                let Some(thread_view) = panel.read(cx).active_thread_view(cx) else {
                    return;
                };

                thread_view.update(cx, |thread_view, cx| {
                    thread_view.message_editor.update(cx, |editor, cx| {
                        if !editor.is_empty(cx) {
                            editor.insert_text("\n\n", window, cx);
                        }
                        editor.insert_text(&note, window, cx);
                    });
                });
            });
        });
    }

    fn show_toast(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        let message = message.into();
        workspace.update(cx, |workspace, cx| {
            workspace.show_toast(
                Toast::new(
                    NotificationId::named("web-preview-toast".into()),
                    message.to_string(),
                )
                    .autohide(),
                cx,
            );
        });
    }

    fn open_new_browser_tab(
        &mut self,
        _event: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        workspace.update(cx, |workspace, cx| {
            let view = Self::open_or_create(workspace, window, cx);
            workspace.active_pane().update(cx, |pane, cx| {
                pane.add_item(Box::new(view), true, true, None, window, cx);
            });
            cx.notify();
        });
    }

    fn split_browser_tab(
        &mut self,
        _event: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        workspace.update(cx, |workspace, cx| {
            let _ = workspace.split_and_clone(
                workspace.active_pane().clone(),
                SplitDirection::Right,
                window,
                cx,
            );
        });
    }

    fn toggle_browser_zoom(
        &mut self,
        _event: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        workspace.update(cx, |workspace, cx| {
            workspace.active_pane().update(cx, |pane, cx| {
                pane.toggle_zoom(&ToggleZoom, window, cx);
            });
        });
    }

    fn open_extensions_root(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
        let has_extensions = !self.detected_extensions.is_empty();
        let tooltip_text = if has_extensions {
            format!("Open extensions folder ({})", self.detected_extensions.len())
        } else {
            "No local browser extensions detected".to_string()
        };

        IconButton::new("web-preview-extensions-trigger", IconName::Blocks)
            .icon_size(IconSize::Small)
            .icon_color(if has_extensions {
                Color::Default
            } else {
                Color::Muted
            })
            .disabled(!has_extensions)
            .tooltip(Tooltip::text(tooltip_text))
            .on_click(cx.listener(|this, _, window, cx| {
                this.open_extensions_root(window, cx);
            }))
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

    #[cfg(not(target_os = "windows"))]
    fn ensure_native_preview(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.load_state = PreviewLoadState::Error(
            "Native web preview is currently implemented for Windows only in this fork.".into(),
        );
    }

    #[cfg(target_os = "windows")]
    fn load_url(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.ensure_native_preview(window, cx);
        let mut borrow = self.native_preview.borrow_mut();
        let preview = borrow
            .as_mut()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.load_url(url)?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn load_url(&mut self, _url: &str, _window: &mut Window, _cx: &mut Context<Self>) -> Result<()> {
        Err(anyhow!("Native web preview is not available on this platform"))
    }

    #[cfg(target_os = "windows")]
    fn reload_webview(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<()> {
        self.ensure_native_preview(window, cx);
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.reload()?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn reload_webview(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Result<()> {
        Err(anyhow!("Native web preview is not available on this platform"))
    }

    #[cfg(target_os = "windows")]
    fn evaluate_script(&self, script: &str) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.evaluate_script(script)?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn evaluate_script(&self, _script: &str) -> Result<()> {
        Err(anyhow!("Native web preview is not available on this platform"))
    }

    #[cfg(target_os = "windows")]
    fn apply_zoom(&self) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.zoom(self.zoom_factor)?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn apply_zoom(&self) -> Result<()> {
        Err(anyhow!("Native web preview is not available on this platform"))
    }

    #[cfg(target_os = "windows")]
    fn clear_all_browsing_data(&self) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        preview.webview.clear_all_browsing_data()?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn clear_all_browsing_data(&self) -> Result<()> {
        Err(anyhow!("Native web preview is not available on this platform"))
    }

    #[cfg(target_os = "windows")]
    fn delete_all_cookies(&self) -> Result<()> {
        let borrow = self.native_preview.borrow();
        let preview = borrow
            .as_ref()
            .ok_or_else(|| anyhow!("The native web preview is not available"))?;
        for cookie in preview.webview.cookies()? {
            preview.webview.delete_cookie(&cookie)?;
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn delete_all_cookies(&self) -> Result<()> {
        Err(anyhow!("Native web preview is not available on this platform"))
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
        Err(anyhow!("Web preview screenshots are not available on this platform"))
    }

    fn render_more_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .items_center()
            .gap_1()
            .child(
                IconButton::new("web-preview-devtools", IconName::Debug)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Open DevTools"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_devtools(window, cx);
                    })),
            )
            .child(
                IconButton::new("web-preview-open-browser", IconName::ArrowUpRight)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Open In Browser"))
                    .on_click(cx.listener(Self::open_in_browser)),
            )
            .child(
                IconButton::new("web-preview-clear-cache", IconName::Trash)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Clear Cache"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.clear_cache(window, cx);
                    })),
            )
    }

    fn render_webview_body(&self, _cx: &mut Context<Self>) -> AnyElement {
        #[cfg(target_os = "windows")]
        {
            let host_bounds = self.host_bounds.clone();
            let last_applied_bounds = self.last_applied_bounds.clone();
            let native_preview = self.native_preview.clone();

            return canvas(
                move |bounds, _window, _cx| {
                    *host_bounds.borrow_mut() = Some(bounds);
                    if let Some(preview) = native_preview.borrow_mut().as_mut() {
                        let should_update_bounds = last_applied_bounds
                            .borrow()
                            .as_ref()
                            .copied()
                            != Some(bounds);
                        if should_update_bounds {
                            let _ = set_webview_bounds(&preview.webview, bounds);
                            *last_applied_bounds.borrow_mut() = Some(bounds);
                        }
                    }
                    bounds
                },
                |_bounds, _state, _window, _cx| {},
            )
            .size_full()
            .into_any_element();
        }

        #[cfg(not(target_os = "windows"))]
        {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    Label::new("Web Preview is currently available on Windows in this fork.")
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

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("Web Preview Opened")
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
                url_editor,
                page_title: None,
                active_url: current_url.clone().into(),
                bookmarks,
                detected_extensions,
                load_state: PreviewLoadState::Ready,
                host_bounds: Rc::new(RefCell::new(None)),
                last_applied_bounds: Rc::new(RefCell::new(None)),
                native_mount_requested: Rc::new(Cell::new(false)),
                browser_events,
                deferred_ipc_messages: Vec::new(),
                ipc_flush_scheduled: false,
                extensions_menu_handle: Default::default(),
                more_menu_handle: Default::default(),
                event_pump_task: None,
                zoom_factor: 1.0,
                #[cfg(target_os = "windows")]
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
        #[cfg(target_os = "windows")]
        self.ensure_native_preview(window, cx);
        self.url_editor.focus_handle(cx).focus(window, cx);
    }

    fn deactivated(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        // Webview stays visible - no hiding needed
    }

    fn workspace_deactivated(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        // Webview stays visible - no hiding needed
    }
}

impl Render for WebPreviewView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let bookmark_tooltip = if is_bookmarked {
            "Remove bookmark"
        } else {
            "Bookmark page"
        };

        div()
            .id("web-preview")
            .key_context("WebPreview")
            .on_action(cx.listener(Self::confirm_navigation))
            .track_focus(&self.focus_handle(cx))
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().colors().editor_background)
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
                                    .tooltip(Tooltip::text("Back"))
                                    .on_click(cx.listener(Self::go_back)),
                            )
                            .child(
                                IconButton::new("web-preview-forward", IconName::ArrowRight)
                                    .icon_size(IconSize::Small)
                                    .tooltip(Tooltip::text("Forward"))
                                    .on_click(cx.listener(Self::go_forward)),
                            )
                            .child(
                                IconButton::new("web-preview-reload", IconName::RotateCw)
                                    .icon_size(IconSize::Small)
                                    .tooltip(Tooltip::text("Reload"))
                                    .on_click(cx.listener(Self::reload)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .h_8()
                                    .min_w_0()
                                    .px_3()
                                    .rounded_full()
                                    .bg(cx.theme().colors().editor_background)
                                    .border_1()
                                    .border_color(cx.theme().colors().border_variant)
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
                                                .icon_color(bookmark_color)
                                                .tooltip(Tooltip::text(bookmark_tooltip))
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
                            .child(body)
                            .when(self.should_disable_webview_input(window, cx), |this| {
                                // Add transparent overlay to block webview input when URL editor is focused
                                this.child(
                                    div()
                                        .absolute()
                                        .inset_0()
                                        .bg(gpui::transparent_black())
                                        .cursor_text()
                                )
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

fn push_browser_event(event_queue: &Arc<Mutex<Vec<BrowserEvent>>>, event: BrowserEvent) {
    let mut queue = event_queue.lock().expect("browser event queue lock poisoned");
    queue.push(event);
}

#[cfg(target_os = "windows")]
fn mount_native_preview(request: NativePreviewMountRequest) {
    eprintln!("[web-preview] deferred mount start");
    let result = catch_unwind(AssertUnwindSafe(|| create_native_preview_for_request(&request)));

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
    let event_queue = request.browser_events.clone();
    let mut web_context = Box::new(WebContext::new(Some(request.profile_dir.clone())));

    prepare_parent_for_child_webview(HWND(request.parent_window.hwnd.get() as *mut _));

    let initial_bounds = request
        .host_bounds
        .borrow()
        .as_ref()
        .copied()
        .map(bounds_to_wry_rect)
        .unwrap_or_else(|| WryRect {
            position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
            size: Size::Logical(LogicalSize::new(32.0, 32.0)),
        });

    eprintln!("[web-preview] create request builder");
    let builder = WebViewBuilder::new_with_web_context(web_context.as_mut())
        .with_bounds(initial_bounds)
        .with_url(url.as_str())
        .with_clipboard(true)
        .with_hotkeys_zoom(false)
        .with_back_forward_navigation_gestures(true)
        .with_default_context_menus(false)
        .with_devtools(true)
        .with_visible(true)
        .with_browser_extensions_enabled(false)
        .with_additional_browser_args(
            "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection",
        )
        .with_initialization_script(WEB_PREVIEW_BRIDGE_SCRIPT)
        .with_document_title_changed_handler({
            let event_queue = event_queue.clone();
            move |title| {
                eprintln!("[web-preview] title changed: {title}");
                push_browser_event(&event_queue, BrowserEvent::TitleChanged(title))
            }
        })
        .with_on_page_load_handler({
            let event_queue = event_queue.clone();
            move |event, url| {
                if matches!(event, PageLoadEvent::Finished) {
                    eprintln!("[web-preview] page finished: {url}");
                    push_browser_event(&event_queue, BrowserEvent::UrlChanged(url));
                }
            }
        })
        .with_ipc_handler(move |request| {
            push_browser_event(
                &event_queue,
                BrowserEvent::IpcMessage(request.body().to_string()),
            );
        });

    eprintln!("[web-preview] create request build");
    let webview = builder
        .build_as_child(&request.parent_window)
        .with_context(|| "Failed to build the embedded web preview")?;
    eprintln!("[web-preview] create request built");
    webview.zoom(request.zoom_factor)?;

    if let Some(bounds) = *request.host_bounds.borrow() {
        eprintln!("[web-preview] create request bounds");
        let _ = set_webview_bounds(&webview, bounds);
    }
    promote_child_webviews(HWND(request.parent_window.hwnd.get() as *mut _));

    *request.native_preview.borrow_mut() = Some(NativeWebPreview {
        _context: web_context,
        webview,
    });
    eprintln!("[web-preview] create request done");

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

fn format_agent_note(payload: BrowserAgentPayload) -> String {
    match payload {
        BrowserAgentPayload::Screenshot { path, url, kind } => format!(
            "Web Preview {kind} screenshot captured.\nURL: {url}\nImage path: {}",
            path.display()
        ),
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
            html,
        } => {
            let mut message = format!("Web Preview inspected element.\nURL: {url}");
            if let Some(title) = title.filter(|title| !title.is_empty()) {
                message.push_str(&format!("\nPage title: {title}"));
            }
            if let Some(selector) = selector.filter(|selector| !selector.is_empty()) {
                message.push_str(&format!("\nSelector: {selector}"));
            }
            if let Some(tag) = tag.filter(|tag| !tag.is_empty()) {
                message.push_str(&format!("\nTag: {tag}"));
            }
            if let Some(id) = id.filter(|id| !id.is_empty()) {
                message.push_str(&format!("\nID: {id}"));
            }
            if !classes.is_empty() {
                message.push_str(&format!("\nClasses: {}", classes.join(" ")));
            }
            if let Some(text) = text.filter(|text| !text.is_empty()) {
                message.push_str(&format!("\nText: {text}"));
            }
            if let Some(href) = href.filter(|href| !href.is_empty()) {
                message.push_str(&format!("\nHref: {href}"));
            }
            if let Some(src) = src.filter(|src| !src.is_empty()) {
                message.push_str(&format!("\nSource: {src}"));
            }
            if let Some(rect) = rect {
                message.push_str(&format!(
                    "\nRect: x={:.1}, y={:.1}, width={:.1}, height={:.1}",
                    rect.x, rect.y, rect.width, rect.height
                ));
            }
            if let Some(html) = html.filter(|html| !html.is_empty()) {
                message.push_str("\nHTML snippet:\n```html\n");
                message.push_str(&html);
                message.push_str("\n```");
            }
            message
        }
    }
}

fn display_title_from_url(url: &str) -> String {
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

    let data =
        fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_slice(&data)
        .with_context(|| format!("Failed to parse {}", path.display()))
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
                    local_app_data.join("Mozilla").join("Firefox").join("Profiles"),
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

#[cfg(target_os = "windows")]
fn prepare_wry_extensions_dir(
    profile_dir: &Path,
    detected_extensions: &[DetectedExtension],
) -> Result<Option<PathBuf>> {
    let loadable = detected_extensions
        .iter()
        .filter(|extension| extension.supports_chromium_loading)
        .collect::<Vec<_>>();
    if loadable.is_empty() {
        return Ok(None);
    }

    let staging_root = profile_dir.join("wry_extensions");
    fs::create_dir_all(&staging_root)
        .with_context(|| format!("Failed to create {}", staging_root.display()))?;

    for extension in loadable {
        let target = staging_root.join(format!(
            "{}-{}",
            slugify(&extension.browser),
            slugify(&extension.id)
        ));
        if target.exists() {
            continue;
        }
        copy_dir_all(&extension.path, &target)?;
    }

    Ok(Some(staging_root))
}

#[cfg(not(target_os = "windows"))]
fn prepare_wry_extensions_dir(
    _profile_dir: &Path,
    _detected_extensions: &[DetectedExtension],
) -> Result<Option<PathBuf>> {
    Ok(None)
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)
        .with_context(|| format!("Failed to create {}", destination.display()))?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let entry_path = entry.path();
        let target_path = destination.join(entry.file_name());
        if entry_path.is_dir() {
            copy_dir_all(&entry_path, &target_path)?;
        } else {
            fs::copy(&entry_path, &target_path).with_context(|| {
                format!(
                    "Failed to copy {} to {}",
                    entry_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_webview_bounds(webview: &WebView, bounds: Bounds<Pixels>) -> Result<()> {
    let rect = bounds_to_wry_rect(bounds);
    webview.set_bounds(rect)?;
    // Webview stays visible - no hiding
    Ok(())
}

#[cfg(target_os = "windows")]
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
fn prepare_parent_for_child_webview(hwnd: HWND) {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let clipped_style = style | WS_CLIPCHILDREN.0 as isize | WS_CLIPSIBLINGS.0 as isize;
        if clipped_style != style {
            let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, clipped_style);
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

#[cfg(target_os = "windows")]
fn promote_child_webviews(parent: HWND) {
    unsafe {
        let mut previous = None;
        loop {
            let child = FindWindowExW(
                Some(parent),
                previous,
                windows::core::w!("WRY_WEBVIEW"),
                None,
            )
            .unwrap_or(HWND(std::ptr::null_mut()));
            if child.0.is_null() {
                break;
            }
            let _ = SetWindowPos(
                child,
                Some(HWND_BOTTOM),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = ShowWindow(child, windows::Win32::UI::WindowsAndMessaging::SW_SHOW);
            previous = Some(child);
        }
    }
}

#[cfg(target_os = "windows")]
fn screen_rect_for_bounds(window: &Window, bounds: Bounds<Pixels>) -> Result<RECT> {
    let handle = raw_window_handle::HasWindowHandle::window_handle(window)?;
    let RawWindowHandle::Win32(raw_handle) = handle.as_raw() else {
        return Err(anyhow!("Unsupported window handle for web preview screenshots"));
    };

    let hwnd = HWND(raw_handle.hwnd.get() as *mut _);
    let mut client_origin = POINT { x: 0, y: 0 };
    unsafe {
        if !ClientToScreen(hwnd, &mut client_origin).as_bool() {
            return Err(anyhow!(
                "Failed to translate the web preview bounds to screen coordinates"
            ));
        }
    }

    let scale = window.scale_factor();
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
            DeleteDC(memory_dc);
            ReleaseDC(None, screen_dc);
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
            DeleteObject(HGDIOBJ(bitmap.0));
            DeleteDC(memory_dc);
            ReleaseDC(None, screen_dc);
            return Err(anyhow!("Failed to copy the web preview surface into a bitmap"));
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

        DeleteObject(HGDIOBJ(bitmap.0));
        DeleteDC(memory_dc);
        ReleaseDC(None, screen_dc);

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
        return Err(anyhow!("The selected capture area is outside the visible page"));
    }

    let width = width.min(image_width.saturating_sub(x));
    let height = height.min(image_height.saturating_sub(y));
    Ok(imageops::crop_imm(&image, x, y, width, height).to_image())
}

const WEB_PREVIEW_BRIDGE_SCRIPT: &str = r#"
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
          selector: cssSelector(element),
          tag: element.tagName.toLowerCase(),
          id: element.id || null,
          classes: Array.from(element.classList || []),
          text: limitText(element.innerText || element.textContent, 2000),
          href: element.getAttribute("href"),
          src: element.getAttribute("src"),
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
        assert_eq!(display_title_from_url("https://www.google.com/"), "www.google.com");
    }
}
