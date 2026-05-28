use std::{borrow::Cow, ops::Range, sync::Arc};

use editor::{
    Anchor, Editor, EditorSnapshot, ToOffset,
    display_map::{BlockContext, BlockPlacement, BlockProperties, BlockStyle},
    hover_popover::diagnostics_markdown_style,
};
use gpui::{AppContext, Entity, Focusable, WeakEntity};
use language::{BufferId, Diagnostic, DiagnosticEntryRef, LanguageRegistry};
use lsp::DiagnosticSeverity;
use markdown::{CopyButtonVisibility, Markdown, MarkdownElement};
use settings::Settings;
use text::Point;
use theme_settings::ThemeSettings;
use ui::{CopyButton, prelude::*};
use util::maybe;

use crate::toolbar_controls::DiagnosticsToolbarEditor;

const MAX_DIAGNOSTIC_BLOCKS_PER_GROUP: usize = 128;
const MAX_DIAGNOSTIC_HINT_LINKS_PER_GROUP: usize = 32;
const MAX_DIAGNOSTIC_MARKDOWN_CHARS: usize = 16_384;
const MAX_DIAGNOSTIC_INLINE_METADATA_CHARS: usize = 512;
const MAX_DIAGNOSTIC_COPY_MESSAGE_CHARS: usize = 16_384;
const DIAGNOSTIC_TRUNCATION_MARKER: &str = "\n\n[diagnostic output truncated]";

pub struct DiagnosticRenderer;

struct BoundedDiagnosticEntry<'a> {
    ix: usize,
    entry: DiagnosticEntryRef<'a, Point>,
}

impl DiagnosticRenderer {
    pub fn diagnostic_blocks_for_group(
        diagnostic_group: Vec<DiagnosticEntryRef<'_, Point>>,
        buffer_id: BufferId,
        diagnostics_editor: Option<Arc<dyn DiagnosticsToolbarEditor>>,
        language_registry: Option<Arc<LanguageRegistry>>,
        cx: &mut App,
    ) -> Vec<DiagnosticBlock> {
        let Some(primary_ix) = diagnostic_group
            .iter()
            .position(|d| d.diagnostic.is_primary)
        else {
            return Vec::new();
        };
        let bounded_entries = Self::bounded_diagnostic_group_entries(&diagnostic_group, primary_ix);
        let Some(primary_entry) = bounded_entries.iter().find(|entry| entry.ix == primary_ix)
        else {
            return Vec::new();
        };
        let primary = &primary_entry.entry;
        let group_id = primary.diagnostic.group_id;
        let mut results = Vec::with_capacity(bounded_entries.len());
        for entry in bounded_entries.iter() {
            let mut markdown = Self::bounded_diagnostic_markdown(entry.entry.diagnostic);
            if entry.entry.diagnostic.is_primary {
                let diagnostic = primary.diagnostic;
                if diagnostic.source.is_some() || diagnostic.code.is_some() {
                    markdown.push_str(" (");
                }
                if let Some(source) = diagnostic.source.as_ref() {
                    markdown.push_str(&Self::escaped_bounded_diagnostic_text(
                        source,
                        MAX_DIAGNOSTIC_INLINE_METADATA_CHARS,
                    ));
                }
                if diagnostic.source.is_some() && diagnostic.code.is_some() {
                    markdown.push(' ');
                }
                if let Some(code) = diagnostic.code.as_ref() {
                    let code = code.to_string();
                    if let Some(description) = diagnostic.code_description.as_ref() {
                        markdown.push('[');
                        markdown.push_str(&Self::escaped_bounded_diagnostic_text(
                            &code,
                            MAX_DIAGNOSTIC_INLINE_METADATA_CHARS,
                        ));
                        markdown.push_str("](");
                        markdown.push_str(&Self::escaped_bounded_diagnostic_text(
                            description.as_ref(),
                            MAX_DIAGNOSTIC_INLINE_METADATA_CHARS,
                        ));
                        markdown.push(')');
                    } else {
                        markdown.push_str(&Self::escaped_bounded_diagnostic_text(
                            &code,
                            MAX_DIAGNOSTIC_INLINE_METADATA_CHARS,
                        ));
                    }
                }
                if diagnostic.source.is_some() || diagnostic.code.is_some() {
                    markdown.push(')');
                }

                for entry in bounded_entries
                    .iter()
                    .filter(|entry| {
                        entry
                            .entry
                            .range
                            .start
                            .row
                            .abs_diff(primary.range.start.row)
                            >= 5
                    })
                    .take(MAX_DIAGNOSTIC_HINT_LINKS_PER_GROUP)
                {
                    markdown.push_str("\n- hint: [");
                    markdown.push_str(&Self::escaped_bounded_diagnostic_text(
                        &entry.entry.diagnostic.message,
                        MAX_DIAGNOSTIC_INLINE_METADATA_CHARS,
                    ));
                    let ix = entry.ix;
                    markdown.push_str(&format!(
                        "](file://#diagnostic-{buffer_id}-{group_id}-{ix})\n",
                    ))
                }

                results.push(DiagnosticBlock {
                    initial_range: primary.range.clone(),
                    severity: primary.diagnostic.severity,
                    diagnostics_editor: diagnostics_editor.clone(),
                    copy_message: Self::bounded_diagnostic_copy_message(
                        &primary.diagnostic.message,
                    ),
                    markdown: cx.new(|cx| {
                        Markdown::new(markdown.into(), language_registry.clone(), None, cx)
                    }),
                });
            } else {
                if entry
                    .entry
                    .range
                    .start
                    .row
                    .abs_diff(primary.range.start.row)
                    >= 5
                {
                    markdown.push_str(&format!(
                        " ([back](file://#diagnostic-{buffer_id}-{group_id}-{primary_ix}))"
                    ));
                }
                results.push(DiagnosticBlock {
                    initial_range: entry.entry.range.clone(),
                    severity: entry.entry.diagnostic.severity,
                    diagnostics_editor: diagnostics_editor.clone(),
                    copy_message: Self::bounded_diagnostic_copy_message(
                        &entry.entry.diagnostic.message,
                    ),
                    markdown: cx.new(|cx| {
                        Markdown::new(markdown.into(), language_registry.clone(), None, cx)
                    }),
                });
            }
        }

        results
    }

    fn bounded_diagnostic_group_entries<'a>(
        diagnostic_group: &[DiagnosticEntryRef<'a, Point>],
        primary_ix: usize,
    ) -> Vec<BoundedDiagnosticEntry<'a>> {
        let mut entries =
            Vec::with_capacity(diagnostic_group.len().min(MAX_DIAGNOSTIC_BLOCKS_PER_GROUP));
        for (ix, entry) in diagnostic_group.iter().enumerate() {
            if entries.len() >= MAX_DIAGNOSTIC_BLOCKS_PER_GROUP {
                break;
            }
            entries.push(BoundedDiagnosticEntry {
                ix,
                entry: entry.clone(),
            });
        }

        if !entries.iter().any(|entry| entry.ix == primary_ix)
            && let Some(primary) = diagnostic_group.get(primary_ix)
        {
            if entries.len() >= MAX_DIAGNOSTIC_BLOCKS_PER_GROUP {
                entries.pop();
            }
            entries.push(BoundedDiagnosticEntry {
                ix: primary_ix,
                entry: primary.clone(),
            });
            entries.sort_by_key(|entry| entry.ix);
        }

        entries
    }

    fn bounded_diagnostic_markdown(diagnostic: &Diagnostic) -> String {
        if let Some(md) = &diagnostic.markdown {
            Self::bounded_diagnostic_text(md, MAX_DIAGNOSTIC_MARKDOWN_CHARS).into_owned()
        } else {
            Self::escaped_bounded_diagnostic_text(
                &diagnostic.message,
                MAX_DIAGNOSTIC_MARKDOWN_CHARS,
            )
        }
    }

    fn bounded_diagnostic_copy_message(message: &str) -> SharedString {
        Self::bounded_diagnostic_text(message, MAX_DIAGNOSTIC_COPY_MESSAGE_CHARS)
            .into_owned()
            .into()
    }

    fn escaped_bounded_diagnostic_text(text: &str, max_chars: usize) -> String {
        Markdown::escape(Self::bounded_diagnostic_text(text, max_chars).as_ref()).into_owned()
    }

    fn bounded_diagnostic_text(text: &str, max_chars: usize) -> Cow<'_, str> {
        let Some((byte_limit, _)) = text.char_indices().nth(max_chars) else {
            return Cow::Borrowed(text);
        };

        let mut truncated = String::with_capacity(byte_limit + DIAGNOSTIC_TRUNCATION_MARKER.len());
        truncated.push_str(&text[..byte_limit]);
        truncated.push_str(DIAGNOSTIC_TRUNCATION_MARKER);
        Cow::Owned(truncated)
    }
}

impl editor::DiagnosticRenderer for DiagnosticRenderer {
    fn render_group(
        &self,
        diagnostic_group: Vec<DiagnosticEntryRef<'_, Point>>,
        buffer_id: BufferId,
        snapshot: EditorSnapshot,
        editor: WeakEntity<Editor>,
        language_registry: Option<Arc<LanguageRegistry>>,
        cx: &mut App,
    ) -> Vec<BlockProperties<Anchor>> {
        let blocks = Self::diagnostic_blocks_for_group(
            diagnostic_group,
            buffer_id,
            None,
            language_registry,
            cx,
        );

        blocks
            .into_iter()
            .map(|block| {
                let editor = editor.clone();
                BlockProperties {
                    placement: BlockPlacement::Near(
                        snapshot
                            .buffer_snapshot()
                            .anchor_after(block.initial_range.start),
                    ),
                    height: Some(1),
                    style: BlockStyle::Flex,
                    render: Arc::new(move |bcx| block.render_block(editor.clone(), bcx)),
                    priority: 1,
                }
            })
            .collect()
    }

    fn render_hover(
        &self,
        diagnostic_group: Vec<DiagnosticEntryRef<'_, Point>>,
        range: Range<Point>,
        buffer_id: BufferId,
        language_registry: Option<Arc<LanguageRegistry>>,
        cx: &mut App,
    ) -> Option<Entity<Markdown>> {
        let blocks = Self::diagnostic_blocks_for_group(
            diagnostic_group,
            buffer_id,
            None,
            language_registry,
            cx,
        );
        blocks
            .into_iter()
            .find_map(|block| (block.initial_range == range).then(|| block.markdown))
    }

    fn open_link(
        &self,
        editor: &mut Editor,
        link: SharedString,
        window: &mut Window,
        cx: &mut Context<Editor>,
    ) {
        DiagnosticBlock::open_link(editor, &None, link, window, cx);
    }
}

#[derive(Clone)]
pub(crate) struct DiagnosticBlock {
    pub(crate) initial_range: Range<Point>,
    pub(crate) severity: DiagnosticSeverity,
    pub(crate) markdown: Entity<Markdown>,
    pub(crate) diagnostics_editor: Option<Arc<dyn DiagnosticsToolbarEditor>>,
    pub(crate) copy_message: SharedString,
}

impl DiagnosticBlock {
    pub fn render_block(&self, editor: WeakEntity<Editor>, bcx: &BlockContext) -> AnyElement {
        let cx = &bcx.app;
        let status_colors = cx.theme().status();

        let max_width = bcx.em_width * 120.;

        let (background_color, border_color) = match self.severity {
            DiagnosticSeverity::ERROR => (status_colors.error_background, status_colors.error),
            DiagnosticSeverity::WARNING => {
                (status_colors.warning_background, status_colors.warning)
            }
            DiagnosticSeverity::INFORMATION => (status_colors.info_background, status_colors.info),
            DiagnosticSeverity::HINT => (status_colors.hint_background, status_colors.hint),
            _ => (status_colors.ignored_background, status_colors.ignored),
        };
        let settings = ThemeSettings::get_global(cx);
        let editor_line_height = (settings.line_height() * settings.buffer_font_size(cx)).round();
        let line_height = editor_line_height;
        let diagnostics_editor = self.diagnostics_editor.clone();

        let copy_button_id = format!(
            "copy-diagnostic-{}-{}-{}-{}",
            self.initial_range.start.row,
            self.initial_range.start.column,
            self.initial_range.end.row,
            self.initial_range.end.column
        );

        h_flex()
            .max_w(max_width)
            .pl_1p5()
            .pr_0p5()
            .items_start()
            .gap_1()
            .border_l_2()
            .line_height(line_height)
            .bg(background_color)
            .border_color(border_color)
            .child(
                div().flex_1().min_w_0().child(
                    MarkdownElement::new(
                        self.markdown.clone(),
                        diagnostics_markdown_style(bcx.window, cx),
                    )
                    .code_block_renderer(markdown::CodeBlockRenderer::Default {
                        copy_button_visibility: CopyButtonVisibility::Hidden,
                        wrap_button_visibility: markdown::WrapButtonVisibility::Hidden,
                        border: false,
                    })
                    .on_url_click({
                        move |link, window, cx| {
                            editor
                                .update(cx, |editor, cx| {
                                    Self::open_link(editor, &diagnostics_editor, link, window, cx)
                                })
                                .ok();
                        }
                    }),
                ),
            )
            .child(
                CopyButton::new(copy_button_id, self.copy_message.clone())
                    .tooltip_label("Copy Diagnostic"),
            )
            .into_any_element()
    }

    pub fn open_link(
        editor: &mut Editor,
        diagnostics_editor: &Option<Arc<dyn DiagnosticsToolbarEditor>>,
        link: SharedString,
        window: &mut Window,
        cx: &mut Context<Editor>,
    ) {
        let Some(diagnostic_link) = link.strip_prefix("file://#diagnostic-") else {
            editor::hover_popover::open_markdown_url(link, window, cx);
            return;
        };
        let Some((buffer_id, group_id, ix)) = maybe!({
            let mut parts = diagnostic_link.split('-');
            let buffer_id: u64 = parts.next()?.parse().ok()?;
            let group_id: usize = parts.next()?.parse().ok()?;
            let ix: usize = parts.next()?.parse().ok()?;
            Some((BufferId::new(buffer_id).ok()?, group_id, ix))
        }) else {
            return;
        };

        if let Some(diagnostics_editor) = diagnostics_editor {
            if let Some(diagnostic) = diagnostics_editor
                .get_diagnostics_for_buffer(buffer_id, cx)
                .into_iter()
                .filter(|d| d.diagnostic.group_id == group_id)
                .nth(ix)
            {
                let multibuffer = editor.buffer().read(cx);
                if let Some(anchor_range) = multibuffer
                    .snapshot(cx)
                    .buffer_anchor_range_to_anchor_range(diagnostic.range)
                {
                    Self::jump_to(editor, anchor_range, window, cx);
                    return;
                }
            }
        } else if let Some(diagnostic) = editor
            .snapshot(window, cx)
            .buffer_snapshot()
            .diagnostic_group(buffer_id, group_id)
            .nth(ix)
        {
            Self::jump_to(editor, diagnostic.range, window, cx)
        };
    }

    fn jump_to<I: ToOffset>(
        editor: &mut Editor,
        range: Range<I>,
        window: &mut Window,
        cx: &mut Context<Editor>,
    ) {
        let snapshot = &editor.buffer().read(cx).snapshot(cx);
        let range = range.start.to_offset(snapshot)..range.end.to_offset(snapshot);

        editor.unfold_ranges(&[range.start..range.end], true, false, cx);
        editor.change_selections(Default::default(), window, cx, |s| {
            s.select_ranges([range.start..range.start]);
        });
        window.focus(&editor.focus_handle(cx), cx);
    }
}
