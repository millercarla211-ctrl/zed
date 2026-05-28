use std::borrow::Cow;

use client::telemetry;
use extension_host::{ExtensionIndexEntry, ExtensionStore};
use gpui::{App, ClipboardItem, PromptLevel, actions};
use system_specs::{CopySystemSpecsIntoClipboard, SystemSpecs};
use util::ResultExt;
use workspace::Workspace;
use zed_actions::feedback::{EmailZed, FileBugReport, RequestFeature};

actions!(
    zed,
    [
        /// Opens the Zed repository on GitHub.
        OpenZedRepo,
        /// Copies installed extensions to the clipboard for bug reports.
        CopyInstalledExtensionsIntoClipboard
    ]
);

const ZED_REPO_URL: &str = "https://github.com/zed-industries/zed";

const REQUEST_FEATURE_URL: &str = "https://github.com/zed-industries/zed/discussions/new/choose";

const MAX_INSTALLED_EXTENSIONS_FOR_BUG_REPORT: usize = 512;
const MAX_INSTALLED_EXTENSION_FIELD_CHARS: usize = 512;
const MAX_INSTALLED_EXTENSIONS_PROMPT_CHARS: usize = 16 * 1024;

fn file_bug_report_url(specs: &SystemSpecs) -> String {
    format!(
        concat!(
            "https://github.com/zed-industries/zed/issues/new",
            "?",
            "template=10_bug_report.yml",
            "&",
            "environment={}"
        ),
        urlencoding::encode(&specs.to_string())
    )
}

fn email_zed_url(specs: &SystemSpecs) -> String {
    format!(
        concat!("mailto:hi@zed.dev", "?", "body={}"),
        email_body(specs)
    )
}

fn email_body(specs: &SystemSpecs) -> String {
    let body = format!("\n\nSystem Information:\n\n{}", specs);
    urlencoding::encode(&body).to_string()
}

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _, _| {
        workspace
            .register_action(|_, _: &CopySystemSpecsIntoClipboard, window, cx| {
                let specs =
                    SystemSpecs::new(window, cx, telemetry::os_name(), telemetry::os_version());

                cx.spawn_in(window, async move |_, cx| {
                    let specs = specs.await.to_string();

                    cx.update(|_, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(specs.clone()))
                    })
                    .log_err();

                    cx.prompt(
                        PromptLevel::Info,
                        "Copied into clipboard",
                        Some(&specs),
                        &["OK"],
                    )
                    .await
                })
                .detach();
            })
            .register_action(|_, _: &CopyInstalledExtensionsIntoClipboard, window, cx| {
                let clipboard_text = format_installed_extensions_for_clipboard(cx);
                cx.write_to_clipboard(ClipboardItem::new_string(clipboard_text.clone()));
                let prompt_text = installed_extensions_prompt_text(&clipboard_text);
                drop(window.prompt(
                    PromptLevel::Info,
                    "Copied into clipboard",
                    Some(prompt_text.as_ref()),
                    &["OK"],
                    cx,
                ));
            })
            .register_action(|_, _: &RequestFeature, _, cx| {
                cx.open_url(REQUEST_FEATURE_URL);
            })
            .register_action(move |_, _: &FileBugReport, window, cx| {
                let specs =
                    SystemSpecs::new(window, cx, telemetry::os_name(), telemetry::os_version());
                cx.spawn_in(window, async move |_, cx| {
                    let specs = specs.await;
                    cx.update(|_, cx| {
                        cx.open_url(&file_bug_report_url(&specs));
                    })
                    .log_err();
                })
                .detach();
            })
            .register_action(move |_, _: &EmailZed, window, cx| {
                let specs =
                    SystemSpecs::new(window, cx, telemetry::os_name(), telemetry::os_version());
                cx.spawn_in(window, async move |_, cx| {
                    let specs = specs.await;
                    cx.update(|_, cx| {
                        cx.open_url(&email_zed_url(&specs));
                    })
                    .log_err();
                })
                .detach();
            })
            .register_action(move |_, _: &OpenZedRepo, _, cx| {
                cx.open_url(ZED_REPO_URL);
            });
    })
    .detach();
}

fn format_installed_extensions_for_clipboard(cx: &mut App) -> String {
    let store = ExtensionStore::global(cx);
    let store = store.read(cx);
    let extension_count = store.extension_index.extensions.len();
    let line_limit = extension_count.min(MAX_INSTALLED_EXTENSIONS_FOR_BUG_REPORT);
    let mut lines = Vec::with_capacity(line_limit);

    for (extension_id, entry) in store
        .extension_index
        .extensions
        .iter()
        .take(MAX_INSTALLED_EXTENSIONS_FOR_BUG_REPORT)
    {
        lines.push(format_installed_extension_line(
            extension_id.as_ref(),
            entry,
        ));
    }

    lines.sort();

    if lines.is_empty() {
        return "No extensions installed.".to_string();
    }

    if extension_count > line_limit {
        lines.push(format_installed_extension_overflow_notice(
            extension_count - line_limit,
        ));
    }

    let heading = if extension_count == line_limit {
        format!("Installed extensions ({}):", extension_count)
    } else {
        format!(
            "Installed extensions ({} of {}):",
            line_limit, extension_count
        )
    };

    format!("{}\n{}", heading, lines.join("\n"))
}

fn format_installed_extension_line(extension_id: &str, entry: &ExtensionIndexEntry) -> String {
    let name = installed_extension_report_field(entry.manifest.name.as_str());
    let extension_id = installed_extension_report_field(extension_id.as_ref());
    let version = installed_extension_report_field(entry.manifest.version.as_ref());

    format!(
        "- {} ({}) v{}{}",
        name,
        extension_id,
        version,
        if entry.dev { " (dev)" } else { "" }
    )
}

fn format_installed_extension_overflow_notice(hidden_extension_count: usize) -> String {
    let noun = if hidden_extension_count == 1 {
        "extension"
    } else {
        "extensions"
    };

    format!("- ... and {hidden_extension_count} more {noun} not shown")
}

fn installed_extension_report_field(value: &str) -> Cow<'_, str> {
    capped_display_text(value, MAX_INSTALLED_EXTENSION_FIELD_CHARS)
}

fn installed_extensions_prompt_text(clipboard_text: &str) -> Cow<'_, str> {
    capped_display_text(clipboard_text, MAX_INSTALLED_EXTENSIONS_PROMPT_CHARS)
}

fn capped_display_text(value: &str, max_chars: usize) -> Cow<'_, str> {
    let Some((boundary, _)) = value.char_indices().nth(max_chars) else {
        return Cow::Borrowed(value);
    };

    Cow::Owned(format!("{}...", &value[..boundary]))
}
