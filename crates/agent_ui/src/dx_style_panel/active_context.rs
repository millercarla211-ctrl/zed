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
    pub(super) apply_gate: StyleApplyGateSnapshot,
}

impl ActiveStyleContextSnapshot {
    fn new(status: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            detail: detail.into(),
            source_path: None,
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

    fn with_token(
        mut self,
        token: impl Into<String>,
        start: usize,
        end: usize,
        source_path: &str,
        source_digest: String,
        attribute_tokens: Vec<String>,
    ) -> Self {
        let token = token.into();
        self.group_context = ActiveGroupContext::from_tokens(
            Some(&token),
            attribute_tokens.as_slice(),
            Some(source_path),
        );
        self.apply_gate = style_apply_gate(Some(StyleApplyGateInput {
            token: &token,
            source_path,
            span_start: start,
            span_end: end,
            source_digest: Some(&source_digest),
        }));
        self.source_path = Some(source_path.to_string());
        self.context_kind = Some("class_token".to_string());
        self.token = Some(token);
        self.attribute_tokens = attribute_tokens;
        self.span = Some(format!("{start}..{end}"));
        self.span_start = Some(start);
        self.span_end = Some(end);
        self.source_digest = Some(source_digest);
        self
    }

    fn with_attribute_tokens(
        mut self,
        attribute_tokens: Vec<String>,
        source_path: &str,
        source_digest: String,
    ) -> Self {
        self.group_context =
            ActiveGroupContext::from_tokens(None, attribute_tokens.as_slice(), Some(source_path));
        self.source_path = Some(source_path.to_string());
        self.source_digest = Some(source_digest);
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

    let path = project_path.path.display(PathStyle::local()).into_owned();
    if !is_style_bearing_path(&path) {
        return ActiveStyleContextSnapshot::new("non-style file", path);
    }

    let Some(editor) = active_item.act_as::<Editor>(cx) else {
        return ActiveStyleContextSnapshot::new("style file", path.clone())
            .with_source_path(path)
            .with_source_state("active item is not an editor buffer");
    };
    let editor = editor.read(cx);
    let display_snapshot = editor.display_snapshot(cx);
    let cursor = editor
        .selections
        .newest::<MultiBufferOffset>(&display_snapshot)
        .head()
        .0;
    let source = editor.text(cx);
    if source.len() > MAX_ACTIVE_STYLE_CONTEXT_BYTES {
        return ActiveStyleContextSnapshot::new("style file too large", path.clone())
            .with_source_path(path)
            .with_source_state("cursor token scan skipped for large active file");
    }
    let source_digest = active_source_digest(&source);

    match cursor_style_token(&source, cursor) {
        CursorStyleToken::Token {
            token,
            start,
            end,
            attribute_tokens,
        } => ActiveStyleContextSnapshot::new("style token", path.clone())
            .with_source_state("static class/className token under cursor")
            .with_token(token, start, end, &path, source_digest, attribute_tokens),
        CursorStyleToken::StaticAttribute { attribute_tokens } => {
            ActiveStyleContextSnapshot::new("static class list", path.clone())
                .with_source_state("cursor is inside a static class/className attribute")
                .with_attribute_tokens(attribute_tokens, &path, source_digest)
        }
        CursorStyleToken::DynamicAttribute => {
            ActiveStyleContextSnapshot::new("dynamic className", path.clone())
                .with_source_path(path)
                .with_source_state(
                    "dynamic expressions are read-only until DX Style returns trusted spans",
                )
        }
        CursorStyleToken::NonLiteralAttribute => {
            ActiveStyleContextSnapshot::new("non-literal class", path.clone())
                .with_source_path(path)
                .with_source_state("non-literal class values are read-only")
        }
        CursorStyleToken::UnterminatedAttribute => {
            ActiveStyleContextSnapshot::new("unterminated class", path.clone())
                .with_source_path(path)
                .with_source_state("class literal must be valid before Style tools can read it")
        }
        CursorStyleToken::Outside if is_css_style_sheet_path(&path) => {
            if let Some(hint) = css_style_hint(&source, cursor) {
                return ActiveStyleContextSnapshot::new("css declaration", path.clone())
                    .with_css_hint(
                        hint.token,
                        hint.property,
                        hint.generator_id,
                        hint.source_edit_safety,
                        hint.start,
                        hint.end,
                        &path,
                        source_digest,
                    );
            }
            ActiveStyleContextSnapshot::new("style-relevant file", path.clone())
                .with_source_path(path)
                .with_source_state("cursor is outside a recognized CSS declaration")
        }
        CursorStyleToken::Outside => {
            ActiveStyleContextSnapshot::new("style-relevant file", path.clone())
                .with_source_path(path)
                .with_source_state("cursor is outside a class/className attribute")
        }
    }
}
