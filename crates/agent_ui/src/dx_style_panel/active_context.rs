use editor::Editor;
use gpui::{App, WeakEntity};
use multi_buffer::MultiBufferOffset;
use serde_json::{Value, json};
use util::paths::PathStyle;
use workspace::Workspace;

use super::{
    apply_gate::{StyleApplyGateInput, StyleApplyGateSnapshot, style_apply_gate},
    css_cursor_context::{css_style_hint, is_css_style_sheet_path},
    cursor_context::{CursorStyleToken, cursor_style_token, is_style_bearing_path},
    group_context::ActiveGroupContext,
    source_digest::active_source_digest,
};

const ACTIVE_STYLE_CONTEXT_SCHEMA: &str = "zed.dx_style.active_context.v1";
const MAX_ACTIVE_STYLE_CONTEXT_BYTES: usize = 256 * 1024;

pub(super) struct ActiveStyleContextSnapshot {
    pub(super) status: String,
    pub(super) detail: String,
    pub(super) source_path: Option<String>,
    pub(super) workspace_root: Option<String>,
    pub(super) source_state: Option<String>,
    pub(super) context_kind: Option<String>,
    pub(super) token: Option<String>,
    pub(super) css_property: Option<String>,
    pub(super) css_generator: Option<String>,
    pub(super) css_source_edit_safety: Option<String>,
    pub(super) attribute_tokens: Vec<String>,
    pub(super) group_context: ActiveGroupContext,
    pub(super) span: Option<String>,
    pub(super) span_start: Option<usize>,
    pub(super) span_end: Option<usize>,
    pub(super) source_digest: Option<String>,
    pub(super) source_len_bytes: Option<usize>,
    pub(super) apply_gate: StyleApplyGateSnapshot,
}

impl ActiveStyleContextSnapshot {
    fn new(status: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            detail: detail.into(),
            source_path: None,
            workspace_root: None,
            source_state: None,
            context_kind: None,
            token: None,
            css_property: None,
            css_generator: None,
            css_source_edit_safety: None,
            attribute_tokens: Vec::new(),
            group_context: ActiveGroupContext::none(),
            span: None,
            span_start: None,
            span_end: None,
            source_digest: None,
            source_len_bytes: None,
            apply_gate: style_apply_gate(None),
        }
    }

    fn with_source_state(mut self, state: impl Into<String>) -> Self {
        self.source_state = Some(state.into());
        self
    }

    fn with_source_path(mut self, source_path: impl Into<String>) -> Self {
        self.source_path = Some(source_path.into());
        self
    }

    fn with_workspace_root(mut self, workspace_root: Option<&str>) -> Self {
        self.workspace_root = workspace_root.map(str::to_string);
        self
    }

    fn with_source_len(mut self, source_len_bytes: usize) -> Self {
        self.source_len_bytes = Some(source_len_bytes);
        self
    }

    fn with_token(
        mut self,
        token: impl Into<String>,
        start: usize,
        end: usize,
        source_path: &str,
        workspace_root: Option<&str>,
        source_digest: String,
        source_len_bytes: usize,
        attribute_tokens: Vec<String>,
    ) -> Self {
        let token = token.into();
        self.group_context = ActiveGroupContext::from_tokens(
            Some(&token),
            attribute_tokens.as_slice(),
            Some(source_path),
            workspace_root,
        );
        self.apply_gate = style_apply_gate(Some(StyleApplyGateInput {
            token: &token,
            source_path,
            workspace_root,
            span_start: start,
            span_end: end,
            source_digest: Some(&source_digest),
        }));
        self.source_path = Some(source_path.to_string());
        self.workspace_root = workspace_root.map(str::to_string);
        self.context_kind = Some("class_token".to_string());
        self.token = Some(token);
        self.attribute_tokens = attribute_tokens;
        self.span = Some(format!("{start}..{end}"));
        self.span_start = Some(start);
        self.span_end = Some(end);
        self.source_digest = Some(source_digest);
        self.source_len_bytes = Some(source_len_bytes);
        self
    }

    fn with_attribute_tokens(
        mut self,
        attribute_tokens: Vec<String>,
        source_path: &str,
        workspace_root: Option<&str>,
        source_digest: String,
        source_len_bytes: usize,
    ) -> Self {
        self.group_context = ActiveGroupContext::from_tokens(
            None,
            attribute_tokens.as_slice(),
            Some(source_path),
            workspace_root,
        );
        self.source_path = Some(source_path.to_string());
        self.workspace_root = workspace_root.map(str::to_string);
        self.source_digest = Some(source_digest);
        self.source_len_bytes = Some(source_len_bytes);
        self.context_kind = Some("class_list".to_string());
        self.attribute_tokens = attribute_tokens;
        self
    }

    fn with_css_hint(
        mut self,
        token: impl Into<String>,
        property: impl Into<String>,
        generator_id: impl Into<String>,
        source_edit_safety: impl Into<String>,
        start: usize,
        end: usize,
        source_path: &str,
        source_digest: String,
        source_len_bytes: usize,
    ) -> Self {
        self.source_path = Some(source_path.to_string());
        self.source_state = Some("CSS declaration generator hint is read-only".to_string());
        self.context_kind = Some("css_declaration".to_string());
        self.token = Some(token.into());
        self.css_property = Some(property.into());
        self.css_generator = Some(generator_id.into());
        self.css_source_edit_safety = Some(source_edit_safety.into());
        self.span = Some(format!("{start}..{end}"));
        self.span_start = Some(start);
        self.span_end = Some(end);
        self.source_digest = Some(source_digest);
        self.source_len_bytes = Some(source_len_bytes);
        self
    }

    fn source_span_json(&self) -> Option<Value> {
        Some(json!({
            "start_byte": self.span_start?,
            "end_byte": self.span_end?,
        }))
    }

    pub(super) fn web_preview_context_json(&self) -> String {
        json!({
            "schema": ACTIVE_STYLE_CONTEXT_SCHEMA,
            "status": self.status,
            "detail": self.detail,
            "source_path": self.source_path,
            "workspace_root": self.workspace_root,
            "source_state": self.source_state,
            "context_kind": self.context_kind,
            "token": self.token,
            "css_property": self.css_property,
            "css_generator": self.css_generator,
            "css_source_edit_safety": self.css_source_edit_safety,
            "attribute_tokens": self.attribute_tokens,
            "group_context": self.group_context.to_json(),
            "span": self.span,
            "source_span": self.source_span_json(),
            "source_digest": self.source_digest,
            "source_len_bytes": self.source_len_bytes,
            "apply_gate": self.apply_gate.to_json(),
            "source_apply": "disabled_until_trusted_grouped_class_source_span_and_dry_run_receipt",
        })
        .to_string()
    }

    pub(super) fn can_open_generator(&self) -> bool {
        !matches!(
            self.status.as_str(),
            "workspace unavailable" | "no active file" | "non-style file"
        )
    }

    pub(super) fn span_byte_range(&self) -> Option<String> {
        Some(format!("{}..{}", self.span_start?, self.span_end?))
    }
}

pub(super) fn active_style_context(
    workspace: &WeakEntity<Workspace>,
    cx: &App,
) -> ActiveStyleContextSnapshot {
    let Some(workspace) = workspace.upgrade() else {
        return ActiveStyleContextSnapshot::new("workspace unavailable", "No active workspace");
    };
    let workspace = workspace.read(cx);
    let Some(active_item) = workspace.active_item(cx) else {
        return ActiveStyleContextSnapshot::new("no active file", "Open a style-bearing file");
    };
    let Some(project_path) = active_item.project_path(cx) else {
        return ActiveStyleContextSnapshot::new("no active file", "Open a style-bearing file");
    };

    let display_path = project_path.path.display(PathStyle::local()).into_owned();
    let project = workspace.project().read(cx);
    let source_path = project
        .absolute_path(&project_path, cx)
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| display_path.clone());
    let workspace_root = project
        .get_workspace_root(&project_path, cx)
        .map(|path| path.display().to_string());
    drop(project);
    if !is_style_bearing_path(&source_path) {
        return ActiveStyleContextSnapshot::new("non-style file", source_path);
    }

    let Some(editor) = active_item.act_as::<Editor>(cx) else {
        return ActiveStyleContextSnapshot::new("style file", source_path.clone())
            .with_source_path(source_path)
            .with_workspace_root(workspace_root.as_deref())
            .with_source_state("active item is not an editor buffer");
    };
    let editor = editor.read(cx);
    let source_len = editor.buffer().read(cx).len(cx).0;
    if source_len > MAX_ACTIVE_STYLE_CONTEXT_BYTES {
        return ActiveStyleContextSnapshot::new("style file too large", source_path.clone())
            .with_source_path(source_path)
            .with_workspace_root(workspace_root.as_deref())
            .with_source_len(source_len)
            .with_source_state("cursor token scan skipped for large active file");
    }

    let display_snapshot = editor.display_snapshot(cx);
    let cursor = editor
        .selections
        .newest::<MultiBufferOffset>(&display_snapshot)
        .head()
        .0;
    let source = editor.text(cx);
    if source.len() > MAX_ACTIVE_STYLE_CONTEXT_BYTES {
        return ActiveStyleContextSnapshot::new("style file too large", source_path.clone())
            .with_source_path(source_path)
            .with_workspace_root(workspace_root.as_deref())
            .with_source_len(source.len())
            .with_source_state("cursor token scan skipped for large active file");
    }
    match cursor_style_token(&source, cursor) {
        CursorStyleToken::Token {
            token,
            start,
            end,
            attribute_tokens,
        } => {
            let source_digest = active_source_digest(&source);
            ActiveStyleContextSnapshot::new("style token", source_path.clone())
                .with_source_state("static class/className token under cursor")
                .with_token(
                    token,
                    start,
                    end,
                    &source_path,
                    workspace_root.as_deref(),
                    source_digest,
                    source.len(),
                    attribute_tokens,
                )
        }
        CursorStyleToken::StaticAttribute { attribute_tokens } => {
            let source_digest = active_source_digest(&source);
            ActiveStyleContextSnapshot::new("static class list", source_path.clone())
                .with_source_state("cursor is inside a static class/className attribute")
                .with_attribute_tokens(
                    attribute_tokens,
                    &source_path,
                    workspace_root.as_deref(),
                    source_digest,
                    source.len(),
                )
        }
        CursorStyleToken::DynamicAttribute => {
            ActiveStyleContextSnapshot::new("dynamic className", source_path.clone())
                .with_source_path(source_path)
                .with_workspace_root(workspace_root.as_deref())
                .with_source_state(
                    "dynamic expressions are read-only until DX Style returns trusted spans",
                )
        }
        CursorStyleToken::NonLiteralAttribute => {
            ActiveStyleContextSnapshot::new("non-literal class", source_path.clone())
                .with_source_path(source_path)
                .with_workspace_root(workspace_root.as_deref())
                .with_source_state("non-literal class values are read-only")
        }
        CursorStyleToken::UnterminatedAttribute => {
            ActiveStyleContextSnapshot::new("unterminated class", source_path.clone())
                .with_source_path(source_path)
                .with_workspace_root(workspace_root.as_deref())
                .with_source_state("class literal must be valid before Style tools can read it")
        }
        CursorStyleToken::Outside if is_css_style_sheet_path(&source_path) => {
            if let Some(hint) = css_style_hint(&source, cursor) {
                let source_digest = active_source_digest(&source);
                return ActiveStyleContextSnapshot::new("css declaration", source_path.clone())
                    .with_css_hint(
                        hint.token,
                        hint.property,
                        hint.generator_id,
                        hint.source_edit_safety,
                        hint.start,
                        hint.end,
                        &source_path,
                        source_digest,
                        source.len(),
                    )
                    .with_workspace_root(workspace_root.as_deref());
            }
            ActiveStyleContextSnapshot::new("style-relevant file", source_path.clone())
                .with_source_path(source_path)
                .with_workspace_root(workspace_root.as_deref())
                .with_source_state("cursor is outside a recognized CSS declaration")
        }
        CursorStyleToken::Outside => {
            ActiveStyleContextSnapshot::new("style-relevant file", source_path.clone())
                .with_source_path(source_path)
                .with_workspace_root(workspace_root.as_deref())
                .with_source_state("cursor is outside a class/className attribute")
        }
    }
}
