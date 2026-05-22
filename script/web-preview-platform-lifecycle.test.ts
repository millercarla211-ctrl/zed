import { readFileSync } from "node:fs";
import test from "node:test";
import assert from "node:assert/strict";

const read = (path) => readFileSync(path, "utf8");

const platformLibs = [
  ["macOS", "crates/web_preview_macos/src/lib.rs", "target_os = \"macos\""],
  ["Linux", "crates/web_preview_linux/src/lib.rs", "target_os = \"linux\""],
];

const platformViews = [
  ["macOS", "crates/web_preview_macos/src/web_preview_view.rs"],
  ["Linux", "crates/web_preview_linux/src/web_preview_view.rs"],
];

const platformHostFiles = [
  "crates/web_preview_macos/src/macos_host.rs",
  "crates/web_preview_linux/src/x11_host.rs",
  "crates/web_preview_linux/src/wayland_host.rs",
];

const desktopPreviewCallsites = [
  ["font panel", "crates/font_panel/src/font_panel.rs"],
  ["media panel", "crates/media_panel/src/media_panel.rs"],
  ["shadcn UI panel", "crates/shadcn_ui_panel/src/shadcn_ui_panel.rs"],
  ["sidebar browser grid", "crates/sidebar/src/sidebar.rs"],
];

const desktopPreviewCargoManifests = [
  ["font panel", "crates/font_panel/Cargo.toml", true],
  ["media panel", "crates/media_panel/Cargo.toml", true],
  ["shadcn UI panel", "crates/shadcn_ui_panel/Cargo.toml", true],
  ["sidebar", "crates/sidebar/Cargo.toml", false],
  ["onboarding", "crates/onboarding/Cargo.toml", false],
];

test("main web_preview crate keeps Windows WebView2 isolated", () => {
  const source = read("crates/web_preview/src/web_preview.rs");

  assert.match(source, /#\[cfg\(target_os = "windows"\)\]\s+pub mod web_preview_view;/);
  assert.match(source, /#\[cfg\(target_os = "windows"\)\]\s+pub\(crate\) mod windows_visual_webview;/);
  assert.match(
    source,
    /#\[cfg\(target_os = "macos"\)\]\s+pub use web_preview_macos::\{OpenPreview, OpenPreviewToTheSide, init, web_preview_view\};/,
  );
  assert.match(
    source,
    /#\[cfg\(target_os = "linux"\)\]\s+pub use web_preview_linux::\{OpenPreview, OpenPreviewToTheSide, init, web_preview_view\};/,
  );
});

test("main web_preview crate re-exports platform action types", () => {
  const source = read("crates/web_preview/src/web_preview.rs");

  assert.match(
    source,
    /#\[cfg\(target_os = "macos"\)\]\s+pub use web_preview_macos::\{OpenPreview, OpenPreviewToTheSide, init, web_preview_view\};/,
  );
  assert.match(
    source,
    /#\[cfg\(target_os = "linux"\)\]\s+pub use web_preview_linux::\{OpenPreview, OpenPreviewToTheSide, init, web_preview_view\};/,
  );
  assert.match(
    source,
    /not\(target_os = "linux"\),\s+not\(target_os = "macos"\),\s+not\(target_os = "windows"\)\s+\)\)\]\s+pub use web_preview_linux::init;/,
  );
});

test("desktop preview entry points use native Web Preview on macOS and Linux", () => {
  for (const [name, path] of desktopPreviewCallsites) {
    const source = read(path);

    assert.match(
      source,
      /#\[cfg\(any\(target_os = "windows", target_os = "macos", target_os = "linux"\)\)\]\s+use web_preview::web_preview_view::WebPreviewView;/,
      `${name} should import WebPreviewView on every supported desktop platform`,
    );
    assert.match(
      source,
      /#\[cfg\(any\(target_os = "windows", target_os = "macos", target_os = "linux"\)\)\]\s+\{/,
      `${name} should use native Web Preview on supported desktop platforms`,
    );
    assert.match(
      source,
      /WebPreviewView::open_url_in_active_pane\(workspace, &?preview_url|WebPreviewView::open_url_in_active_pane\(workspace, url/,
      `${name} should route preview URLs through WebPreviewView`,
    );
    assert.match(
      source,
      /#\[cfg\(not\(any\(target_os = "windows", target_os = "macos", target_os = "linux"\)\)\)\]/,
      `${name} should keep external browser fallback only for unsupported platforms`,
    );
  }
});

test("desktop preview crates depend on web_preview for every supported desktop OS", () => {
  for (const [name, path, unconditional] of desktopPreviewCargoManifests) {
    const source = read(path);

    if (unconditional) {
      assert.match(
        source,
        /^\s*web_preview\.workspace = true$/m,
        `${name} should have an unconditional web_preview dependency`,
      );
      continue;
    }

    assert.match(
      source,
      /\[target\.'cfg\(any\(target_os = "windows", target_os = "macos", target_os = "linux"\)\)'\.dependencies\]\s+web_preview\.workspace = true/s,
      `${name} should enable web_preview on Windows, macOS, and Linux`,
    );
    assert.doesNotMatch(
      source,
      /\[target\.'cfg\(target_os = "windows"\)'\.dependencies\]\s+web_preview\.workspace = true/s,
      `${name} should not keep web_preview Windows-only`,
    );
  }
});

test("onboarding DX preview uses native Web Preview on macOS and Linux", () => {
  const source = read("crates/onboarding/src/onboarding.rs");

  assert.match(
    source,
    /#\[cfg\(any\(target_os = "windows", target_os = "macos", target_os = "linux"\)\)\]\s+use web_preview::web_preview_view::WebPreviewView;/,
  );
  assert.match(
    source,
    /#\[cfg\(any\(target_os = "windows", target_os = "macos", target_os = "linux"\)\)\]\s+dx_web_preview: Option<Entity<WebPreviewView>>,/,
  );
  assert.match(source, /fn ensure_dx_web_preview\(/);
  assert.match(source, /preview\.load_onboarding_url\(&target\.url, window, cx\);/);
  assert.match(
    source,
    /#\[cfg\(any\(target_os = "windows", target_os = "macos", target_os = "linux"\)\)\]\s+fn render_web_preview_canvas\(/,
  );
  assert.match(source, /let preview = self\.ensure_dx_web_preview\(window, cx\);/);
  assert.match(
    source,
    /#\[cfg\(not\(any\(target_os = "windows", target_os = "macos", target_os = "linux"\)\)\)\]/,
  );
  assert.doesNotMatch(source, /Windows Web Preview runtime/);
});

for (const [name, path, cfg] of platformLibs) {
  test(`${name} web preview init registers actions and startup lifecycle`, () => {
    const source = read(path);

    assert.match(source, new RegExp(`#\\[cfg\\(${cfg.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\)\\]`));
    assert.match(source, /web_preview_view::WebPreviewView::register\(workspace, window, cx\);/);
    assert.match(source, /cx\.defer_in\(window, \|workspace, window, cx\| \{/);
    assert.match(
      source,
      /web_preview_view::WebPreviewView::ensure_startup_preview\(workspace, window, cx\);/,
    );
  });
}

for (const [name, path] of platformViews) {
  test(`${name} web preview exposes a startup hook without opening anything eagerly`, () => {
    const source = read(path);

    assert.match(source, /pub fn ensure_startup_preview\(\s*workspace: &mut Workspace,/);
    assert.match(source, /let _ = \(workspace, window, cx\);/);
  });
}

for (const [name, path] of platformViews) {
  test(`${name} web preview has browser tab parity`, () => {
    const source = read(path);

    assert.match(source, /TabContentParams/);
    assert.match(source, /WorkspaceScreenKind/);
    assert.match(source, /fn tab_content\(&self, params: TabContentParams, window: &Window,/);
    assert.match(source, /Label::new\(self\.current_tab_title\(\)\)/);
    assert.match(source, /fn tab_content_text\(&self, _detail: usize, _cx: &App\) -> SharedString \{\s+self\.current_tab_title\(\)/);
    assert.match(source, /fn screen_kind\(&self\) -> WorkspaceScreenKind \{\s+WorkspaceScreenKind::Browser\s+\}/);
    assert.match(source, /fn on_tab_click\(/);
    assert.match(source, /self\.activate_url_editor\(window, cx\);/);
    assert.match(source, /fn on_tab_confirm\(&mut self, window: &mut Window, cx: &mut Context<Self>\) -> bool/);
    assert.match(source, /self\.confirm_navigation\(&Confirm, window, cx\);/);
  });
}

for (const [name, path] of platformViews) {
  test(`${name} web preview uses pane tab controls instead of an in-body toolbar`, () => {
    const source = read(path);

    assert.match(source, /fn render_tab_bar_start_controls\(&self, cx: &mut Context<Self>\) -> AnyElement/);
    assert.match(source, /fn render_tab_bar_end_controls\(&self, cx: &mut Context<Self>\) -> AnyElement/);
    assert.match(source, /IconButton::new\("web-preview-tab-bar-add-trigger", IconName::Plus\)/);
    assert.match(source, /window\.dispatch_action\(NewWebPreview\.boxed_clone\(\), cx\);/);
    assert.match(source, /IconButton::new\("web-preview-tab-bar-back", IconName::ArrowLeft\)/);
    assert.match(source, /IconButton::new\("web-preview-tab-bar-forward", IconName::ArrowRight\)/);
    assert.match(source, /IconButton::new\("web-preview-tab-bar-reload", IconName::RotateCw\)/);
    assert.match(source, /IconButton::new\("web-preview-tab-bar-bookmark", bookmark_icon\)/);
    assert.match(source, /fn render_tab_bar_extensions_menu\(&self, entity: Entity<Self>\) -> impl IntoElement/);
    assert.match(source, /PopoverMenu::new\("web-preview-tab-bar-extensions-menu"\)/);
    assert.match(source, /fn render_tab_bar_more_menu\(&self, entity: Entity<Self>\) -> impl IntoElement/);
    assert.match(source, /PopoverMenu::new\("web-preview-tab-bar-more-menu"\)/);
    assert.match(source, /ContextMenuEntry::new\("Capture Screenshot"\)/);
    assert.match(source, /ContextMenuEntry::new\("Inspect Element"\)/);
    assert.match(source, /ContextMenuEntry::new\("Open DevTools"\)/);
    assert.match(source, /ContextMenuEntry::new\("Clear Cache"\)/);
    assert.match(source, /Some\(PaneTabBarControls::new\(\s+Some\(self\.render_tab_bar_start_controls\(cx\)\),\s+Some\(self\.render_tab_bar_end_controls\(cx\)\),\s+\)\)/);
    assert.doesNotMatch(source, /\.id\("web-preview-toolbar"\)/);
    assert.doesNotMatch(source, /fn render_toolbar_action_button\(/);
    assert.doesNotMatch(source, /web-preview-zoom-in|web-preview-zoom-out/);
  });
}

test("macOS web preview has a native host lifecycle contract", () => {
  const view = read("crates/web_preview_macos/src/web_preview_view.rs");
  const host = read("crates/web_preview_macos/src/macos_host.rs");

  assert.match(view, /crate::macos_host::MacPreviewHost::new\(window, \*host_bounds\.borrow\(\)\)\?/);
  assert.match(view, /\.with_accept_first_mouse\(true\)/);
  assert.match(view, /webview\.reparent\(host\.ns_window_ptr\(\)\)\?/);
  assert.match(view, /sync_macos_native_preview_target\(/);
  assert.match(view, /set_macos_native_preview_visible\(/);
  assert.match(host, /addChildWindow: initialized\s+ordered: NSWindowOrderingMode::Below/);
  assert.match(host, /orderWindow: NSWindowOrderingMode::Below/);
  assert.match(host, /pub\(crate\) fn focus_gpui_view\(&self\)/);
  assert.match(host, /pub\(crate\) fn capture_image\(&self\) -> Result<RgbaImage>/);
});

test("Linux web preview has X11 and Wayland native host contracts", () => {
  const view = read("crates/web_preview_linux/src/web_preview_view.rs");
  const x11Host = read("crates/web_preview_linux/src/x11_host.rs");
  const waylandHost = read("crates/web_preview_linux/src/wayland_host.rs");

  assert.match(view, /use gpui_linux::exported_wayland_window_handle;/);
  assert.match(view, /fn resolve_linux_native_preview_target\(window: &Window\) -> Result<LinuxNativePreviewTarget>/);
  assert.match(view, /ensure_linux_webview_runtime\(window_system\)\?/);
  assert.match(view, /create_native_preview_for_linux_x11_window\(/);
  assert.match(view, /create_native_preview_for_linux_wayland_window\(/);
  assert.match(view, /\.build_gtk\(host\.container\(\)\)/);
  assert.match(view, /fn pump_linux_webview_events\(\) -> bool/);
  assert.match(view, /sync_linux_native_preview_target\(/);
  assert.match(x11Host, /pub\(crate\) struct X11PreviewHost/);
  assert.match(x11Host, /attach_transient_parent\(&window, parent_xid\)\?/);
  assert.match(x11Host, /pub\(crate\) fn capture_image\(&self\) -> Result<RgbaImage>/);
  assert.match(waylandHost, /pub\(crate\) struct WaylandPreviewHost/);
  assert.match(waylandHost, /set_transient_for_exported\(exported_parent_handle\)/);
  assert.match(waylandHost, /pub\(crate\) fn capture_image\(&self\) -> Result<RgbaImage>/);
});

test("platform host support stays in focused files", () => {
  for (const path of platformHostFiles) {
    const source = read(path);
    assert.doesNotMatch(source, /WindowsVisualWebView|WebView2|CoreWebView2/);
  }
});
