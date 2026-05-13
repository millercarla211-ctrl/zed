#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSymbolKind {
    Variable,
    Function,
    Class,
    File,
    Url,
    Ticket,
    Branch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSymbol {
    pub label: String,
    pub kind: EditorSymbolKind,
    pub confidence: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTagReference {
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowEditorAssistPlan {
    pub variable_candidates: Vec<EditorSymbol>,
    pub file_tags: Vec<FileTagReference>,
    pub command_mode_hint: &'static str,
    pub preferred_shortcuts: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowEditorAssistPlanner {
    pub product_name: &'static str,
    pub prefer_symbol_disambiguation: bool,
    pub prefer_file_tagging: bool,
}

impl FlowEditorAssistPlanner {
    pub fn coding_default() -> Self {
        Self {
            product_name: "flow-coding",
            prefer_symbol_disambiguation: true,
            prefer_file_tagging: true,
        }
    }

    pub fn build_plan(
        &self,
        symbols: impl IntoIterator<Item = EditorSymbol>,
        files: impl IntoIterator<Item = FileTagReference>,
    ) -> FlowEditorAssistPlan {
        FlowEditorAssistPlan {
            variable_candidates: symbols.into_iter().collect(),
            file_tags: files.into_iter().collect(),
            command_mode_hint: "Use command mode when the user says rewrite, refactor, explain, or open file.",
            preferred_shortcuts: vec!["Ctrl+Alt+Space", "Ctrl+Shift+Space", "Alt+`"],
        }
    }

    pub fn attach_symbol(label: impl Into<String>, kind: EditorSymbolKind) -> EditorSymbol {
        EditorSymbol {
            label: label.into(),
            kind,
            confidence: 90,
        }
    }

    pub fn attach_file(label: impl Into<String>, path: impl Into<String>) -> FileTagReference {
        FileTagReference {
            label: label.into(),
            path: path.into(),
        }
    }
}
