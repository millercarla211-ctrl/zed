use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

mod active_context;
mod apply_gate;
mod css_cursor_context;
mod css_hint_catalog;
mod cursor_context;
mod cursor_context_tokens;
mod editor_write_bridge;
mod group_context;
mod group_context_token;
mod group_registry;
mod grouping_efficiency;
pub(crate) mod panel;
mod panel_metric;
mod panel_view;
mod readiness;
mod receipt_match;
mod receipt_review;
mod receipt_roots;
mod reverse_css_map;
mod source_digest;

use self::readiness::{DxStyleReadinessSnapshot, dx_style_readiness_snapshot};

const DX_STYLE_ROOT: &str = r"G:\Dx\style";
const DX_ZED_ROOT: &str = r"G:\Dx\zed";
const DX_STYLE_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_TEXT_BYTES: u64 = 128 * 1024;
const MAX_WEB_PREVIEW_HOST_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct DxStylePanelSnapshot {
    pub root: PathBuf,
    pub root_exists: bool,
    pub plan_path: PathBuf,
    pub plan_present: bool,
    pub grouped_contract_path: PathBuf,
    pub grouped_contract_present: bool,
    pub grouped_contract_ready: bool,
    pub generator_catalog_path: PathBuf,
    pub generator_catalog_present: bool,
    pub editor_contract_path: PathBuf,
    pub editor_contract_present: bool,
    pub web_preview_host_path: PathBuf,
    pub web_preview_host_present: bool,
    pub web_preview_bridge_ready: bool,
    pub visual_generator_count: usize,
    pub status: String,
    pub next_action: String,
    pub readiness: DxStyleReadinessSnapshot,
    pub rows: Vec<DxStylePanelRow>,
    pub warnings: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct DxStylePanelRow {
    pub label: String,
    pub state: String,
    pub detail: String,
}

static DX_STYLE_PANEL_CACHE: OnceLock<Mutex<Option<(Instant, DxStylePanelSnapshot)>>> =
    OnceLock::new();

pub(crate) fn dx_style_panel_snapshot() -> DxStylePanelSnapshot {
    let cache = DX_STYLE_PANEL_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, snapshot)) = cache.as_ref() {
            if now.duration_since(*cached_at) <= DX_STYLE_CACHE_TTL {
                return snapshot.clone();
            }
        }

        let snapshot = scan_dx_style_panel();
        *cache = Some((now, snapshot.clone()));
        return snapshot;
    }

    scan_dx_style_panel()
}

fn scan_dx_style_panel() -> DxStylePanelSnapshot {
    let root = PathBuf::from(DX_STYLE_ROOT);
    let plan_path = root.join("PLAN.md");
    let grouped_contract_path = root
        .join("src")
        .join("core")
        .join("engine")
        .join("grouped_class_contract.rs");
    let generator_catalog_path = root
        .join("src")
        .join("core")
        .join("engine")
        .join("visual_generator_catalog.rs");
    let editor_contract_path = root
        .join("src")
        .join("core")
        .join("engine")
        .join("grouped_class_read_model.rs");
    let cursor_context_path = root
        .join("src")
        .join("core")
        .join("engine")
        .join("grouped_class_cursor_context.rs");
    let dry_run_receipt_path = root
        .join("src")
        .join("core")
        .join("engine")
        .join("grouped_class_dry_run_receipt.rs");
    let source_apply_contract_path = root
        .join("src")
        .join("core")
        .join("engine")
        .join("grouped_class_source_apply.rs");
    let group_web_preview_context_path = root
        .join("src")
        .join("core")
        .join("engine")
        .join("grouped_class_web_preview_context.rs");
    let group_registry_receipt_path = root
        .join("src")
        .join("core")
        .join("engine")
        .join("grouped_class_registry_receipt.rs");
    let group_reverse_css_map_path = root
        .join("src")
        .join("core")
        .join("engine")
        .join("grouped_class_reverse_css_map.rs");
    let group_reverse_css_delta_path = root
        .join("src")
        .join("core")
        .join("engine")
        .join("grouped_class_reverse_css_delta.rs");
    let source_apply_contract_fixture_path = root
        .join("fixtures")
        .join("grouped-class-source-apply-contract.json");
    let group_registry_path = root.join("src").join("core").join("group").join("mod.rs");
    let zed_root = PathBuf::from(DX_ZED_ROOT);
    let web_preview_host_path = zed_root
        .join("crates")
        .join("web_preview")
        .join("src")
        .join("web_preview_view.rs");
    let dx_style_generator_surface_path = zed_root
        .join("crates")
        .join("web_preview")
        .join("src")
        .join("dx_style_generator_surface.rs");
    let dx_style_generator_script_path = zed_root
        .join("crates")
        .join("web_preview")
        .join("src")
        .join("dx_style_generator_surface")
        .join("script.rs");
    let dx_style_css_declaration_dry_run_script_path = zed_root
        .join("crates")
        .join("web_preview")
        .join("src")
        .join("dx_style_generator_surface")
        .join("css_declaration_dry_run_script.rs");
    let dx_style_source_apply_session_script_path = zed_root
        .join("crates")
        .join("web_preview")
        .join("src")
        .join("dx_style_generator_surface")
        .join("source_apply_session_script.rs");
    let dx_style_source_apply_path = zed_root
        .join("crates")
        .join("web_preview")
        .join("src")
        .join("dx_style_source_apply.rs");

    let root_exists = root.is_dir();
    let plan_present = plan_path.is_file();
    let grouped_contract_present = grouped_contract_path.is_file();
    let generator_catalog_present = generator_catalog_path.is_file();
    let editor_contract_present = editor_contract_path.is_file();
    let web_preview_host_present = web_preview_host_path.is_file();
    let readiness = dx_style_readiness_snapshot(&root, root_exists);

    let plan_text = read_text_limited(&plan_path);
    let grouped_contract_text = read_text_limited(&grouped_contract_path);
    let group_registry_text = read_text_limited(&group_registry_path);
    let editor_contract_text = read_text_limited(&editor_contract_path);
    let cursor_context_text = read_text_limited(&cursor_context_path);
    let dry_run_receipt_text = read_text_limited(&dry_run_receipt_path);
    let source_apply_contract_text = read_text_limited(&source_apply_contract_path);
    let group_web_preview_context_text = read_text_limited(&group_web_preview_context_path);
    let group_registry_receipt_text = read_text_limited(&group_registry_receipt_path);
    let group_reverse_css_map_text = read_text_limited(&group_reverse_css_map_path);
    let group_reverse_css_delta_text = read_text_limited(&group_reverse_css_delta_path);
    let source_apply_contract_fixture_text = read_text_limited(&source_apply_contract_fixture_path);
    let dx_style_generator_surface_text = read_text_limited(&dx_style_generator_surface_path);
    let dx_style_generator_script_text = read_text_limited(&dx_style_generator_script_path);
    let dx_style_css_declaration_dry_run_script_text =
        read_text_limited(&dx_style_css_declaration_dry_run_script_path);
    let dx_style_source_apply_session_script_text =
        read_text_limited(&dx_style_source_apply_session_script_path);
    let dx_style_source_apply_text = read_text_limited(&dx_style_source_apply_path);
    let visual_generator_count = plan_text
        .as_deref()
        .map(count_declared_visual_generators)
        .unwrap_or_default();
    let grouped_contract_ready = grouped_contract_text
        .as_deref()
        .map(|text| {
            text.contains("GROUPED_CLASS_CONTRACT_SCHEMA")
                && text.contains("dx.style.group-two-way-contract")
                && text.contains("GroupTwoWayInvertibility")
        })
        .unwrap_or_default()
        && group_registry_text
            .as_deref()
            .map(|text| text.contains("GroupTwoWayContract") && text.contains("two_way_contract"))
            .unwrap_or_default();
    let editor_contract_present = editor_contract_present
        && editor_contract_text
            .as_deref()
            .map(|text| {
                text.contains("GroupedClassSourceSpan")
                    && text.contains("GroupedClassDryRunPatchPreview")
                    && text.contains("cursor_token_context: true")
                    && text.contains("broad_tsx_ast_rewrites: false")
            })
            .unwrap_or_default();
    let cursor_context_present = cursor_context_text
        .as_deref()
        .map(|text| {
            text.contains("GROUPED_CLASS_CURSOR_CONTEXT_SCHEMA")
                && text.contains("grouped_class_cursor_context")
                && text.contains("GroupedClassCursorToken")
        })
        .unwrap_or_default();
    let dry_run_receipt_present = dry_run_receipt_text
        .as_deref()
        .map(|text| {
            text.contains("GROUPED_CLASS_DRY_RUN_RECEIPT_SCHEMA")
                && text.contains("GroupedClassDryRunReceipt")
                && text.contains("source_digest_verified")
        })
        .unwrap_or_default();
    let source_apply_contract_ready = source_apply_contract_text
        .as_deref()
        .map(|text| {
            text.contains("GROUPED_CLASS_SOURCE_APPLY_CONTRACT_SCHEMA")
                && text.contains("GROUPED_CLASS_SOURCE_APPLY_IPC_KIND")
                && text.contains("GROUPED_CLASS_SOURCE_APPLY_CONTRACT_VERSION")
                && text.contains("GROUPED_CLASS_SOURCE_APPLY_SCOPE")
                && text.contains("source_mutation_enabled: false")
                && text.contains("reverse CSS map receipt match")
                && text.contains("generated CSS declaration delta validation")
        })
        .unwrap_or_default()
        && source_apply_contract_fixture_text
            .as_deref()
            .map(|text| {
                text.contains("dx.style.grouped-class-source-apply-contract")
                    && text.contains("\"schema_version\": 1")
                    && text.contains("\"scope\":")
                    && text.contains("\"source_mutation_enabled\": false")
            })
            .unwrap_or_default();
    let group_web_preview_context_ready = group_web_preview_context_text
        .as_deref()
        .map(|text| {
            text.contains("GROUPED_CLASS_WEB_PREVIEW_CONTEXT_SCHEMA")
                && text.contains("dx.style.grouped-class-web-preview-context")
                && text.contains("source_mutation_enabled: false")
        })
        .unwrap_or_default();
    let group_registry_receipt_ready = group_registry_receipt_text
        .as_deref()
        .map(|text| {
            text.contains("GROUPED_CLASS_REGISTRY_RECEIPT_SCHEMA")
                && text.contains("dx.style.grouped-class-registry-receipt")
                && text.contains("registry_entries_verified")
                && text.contains("source_owned")
        })
        .unwrap_or_default();
    let group_reverse_css_map_ready = group_reverse_css_map_text
        .as_deref()
        .map(|text| {
            text.contains("GROUPED_CLASS_REVERSE_CSS_MAP_SCHEMA")
                && text.contains("dx.style.grouped-class-reverse-css-map")
                && text.contains("source_mutation_enabled: false")
                && text.contains("editor_write_bridge_required: true")
        })
        .unwrap_or_default();
    let group_reverse_css_delta_ready = group_reverse_css_delta_text
        .as_deref()
        .map(|text| {
            text.contains("GROUPED_CLASS_REVERSE_CSS_DELTA_SCHEMA")
                && text.contains("dx.style.grouped-class-reverse-css-delta-contract")
                && text.contains("source_mutation_enabled: false")
                && text.contains("generated CSS declaration delta validation")
        })
        .unwrap_or_default();
    let web_preview_bridge_ready = web_preview_host_present
        && file_contains_all_markers_limited(
            &web_preview_host_path,
            &["OpenGeneratorPreviewForContext", "dx-style-source-apply"],
            MAX_WEB_PREVIEW_HOST_BYTES,
        )
        && dx_style_generator_surface_text
            .as_deref()
            .map(|text| {
                text.contains("DX_STYLE_GENERATOR_SURFACE_SCHEMA")
                    && text.contains("dx_style_generator_url_with_context_and_source_apply_session")
                    && text.contains("dx_style_source_apply_contract_json")
            })
            .unwrap_or_default()
        && dx_style_generator_script_text
            .as_deref()
            .map(|text| {
                text.contains("renderCssDeclarationDryRunContractReview")
                    && text.contains("dx_style_css_declaration_dry_run_review_script")
                    && text.contains("generatorForContext")
                    && text.contains("orderedCatalog")
                    && text.contains("suggested_generator")
                    && text.contains("Review source")
                    && text.contains("Apply gated")
            })
            .unwrap_or_default()
        && dx_style_css_declaration_dry_run_script_text
            .as_deref()
            .map(|text| {
                text.contains("cssDeclarationDryRunPreview")
                    && text.contains("css_declaration_dry_run_contract_missing")
                    && text.contains("generatedCssDeclarations")
            })
            .unwrap_or_default()
        && dx_style_source_apply_session_script_text
            .as_deref()
            .map(|text| {
                text.contains("sourceApplySessionToken")
                    && text.contains("source_apply_session")
                    && text.contains("window.__DX_STYLE_SOURCE_APPLY__")
            })
            .unwrap_or_default()
        && dx_style_source_apply_text
            .as_deref()
            .map(|text| {
                text.contains("DX_STYLE_SOURCE_APPLY_RECEIPT_SCHEMA")
                    && text.contains("DX_STYLE_SOURCE_APPLY_SESSION_KIND")
                    && text.contains("source_apply_review_receipt")
            })
            .unwrap_or_default();

    let mut warnings = Vec::new();
    if root_exists {
        warnings.push(
            "Read-only: DX Style contracts and plans are available without launching DX Style."
                .to_string(),
        );
        warnings.push(
            "Generator host: Web Preview owns visual controls while GPUI keeps the native shell."
                .to_string(),
        );
        warnings.push(
            "Apply requires trusted dry-run receipts, source identity, and the editor write bridge."
                .to_string(),
        );
    }

    let (status, next_action) = if !root_exists {
        (
            "Missing dx-style root".to_string(),
            format!("Create or mount {DX_STYLE_ROOT} before enabling the Style panel"),
        )
    } else if !grouped_contract_ready {
        (
            "Needs grouped-class contract".to_string(),
            "Finish dx.style.group-two-way-contract before enabling source-backed edits"
                .to_string(),
        )
    } else if !generator_catalog_present {
        (
            "Read model ready".to_string(),
            "Add the visual generator catalog read model for the Web Preview Style cockpit"
                .to_string(),
        )
    } else if !editor_contract_present || !cursor_context_present || !dry_run_receipt_present {
        (
            "Catalog ready".to_string(),
            "Add cursor token context and trusted dry-run receipts before enabling apply actions"
                .to_string(),
        )
    } else if !web_preview_bridge_ready {
        (
            "Read-only contracts ready".to_string(),
            "Wire the Style sidebar to a Web Preview generator surface before adding visual controls"
                .to_string(),
        )
    } else {
        (
            "Style Web Preview source-ready".to_string(),
            "Open context-aware Web Preview generators; source writes stay receipt-gated"
                .to_string(),
        )
    };

    let rows = vec![
        DxStylePanelRow {
            label: "Root".to_string(),
            state: if root_exists { "present" } else { "missing" }.to_string(),
            detail: root.display().to_string(),
        },
        DxStylePanelRow {
            label: "Plan".to_string(),
            state: if plan_present { "present" } else { "missing" }.to_string(),
            detail: format!("{visual_generator_count} visual generator(s) declared"),
        },
        DxStylePanelRow {
            label: "Groups".to_string(),
            state: if grouped_contract_ready {
                "two-way contract"
            } else if grouped_contract_present {
                "contract file present"
            } else {
                "missing"
            }
            .to_string(),
            detail: relative_or_display(&root, &grouped_contract_path),
        },
        DxStylePanelRow {
            label: "Generators".to_string(),
            state: if generator_catalog_present {
                "25-generator catalog"
            } else {
                "planned"
            }
            .to_string(),
            detail: relative_or_display(&root, &generator_catalog_path),
        },
        DxStylePanelRow {
            label: "Source Edits".to_string(),
            state: if editor_contract_present {
                "span/cursor contract"
            } else {
                "read-only"
            }
            .to_string(),
            detail: relative_or_display(&root, &editor_contract_path),
        },
        DxStylePanelRow {
            label: "Dry Run".to_string(),
            state: if dry_run_receipt_present {
                "trusted receipt contract"
            } else {
                "apply gated"
            }
            .to_string(),
            detail: relative_or_display(&root, &dry_run_receipt_path),
        },
        DxStylePanelRow {
            label: "Source Apply".to_string(),
            state: if source_apply_contract_ready {
                "contract review-only"
            } else if source_apply_contract_path.is_file() {
                "contract present"
            } else {
                "missing"
            }
            .to_string(),
            detail: relative_or_display(&root, &source_apply_contract_path),
        },
        DxStylePanelRow {
            label: "Group Context".to_string(),
            state: if group_web_preview_context_ready {
                "web preview contract"
            } else if group_web_preview_context_path.is_file() {
                "contract present"
            } else {
                "missing"
            }
            .to_string(),
            detail: relative_or_display(&root, &group_web_preview_context_path),
        },
        DxStylePanelRow {
            label: "Group Registry".to_string(),
            state: if group_registry_receipt_ready {
                "trusted receipt contract"
            } else if group_registry_receipt_path.is_file() {
                "contract present"
            } else {
                "missing"
            }
            .to_string(),
            detail: relative_or_display(&root, &group_registry_receipt_path),
        },
        DxStylePanelRow {
            label: "Reverse CSS Map".to_string(),
            state: if group_reverse_css_map_ready {
                "review-only contract"
            } else if group_reverse_css_map_path.is_file() {
                "contract present"
            } else {
                "missing"
            }
            .to_string(),
            detail: relative_or_display(&root, &group_reverse_css_map_path),
        },
        DxStylePanelRow {
            label: "Reverse CSS Delta".to_string(),
            state: if group_reverse_css_delta_ready {
                "web preview review contract"
            } else if group_reverse_css_delta_path.is_file() {
                "contract present"
            } else {
                "missing"
            }
            .to_string(),
            detail: relative_or_display(&root, &group_reverse_css_delta_path),
        },
        DxStylePanelRow {
            label: "Web Preview Host".to_string(),
            state: if web_preview_bridge_ready {
                "generator host ready"
            } else if web_preview_host_present {
                "host present"
            } else {
                "missing"
            }
            .to_string(),
            detail: format!(
                "{} + {}",
                relative_or_display(&zed_root, &web_preview_host_path),
                relative_or_display(&zed_root, &dx_style_generator_surface_path),
            ),
        },
        DxStylePanelRow {
            label: "Native Sidebar".to_string(),
            state: "host shell only".to_string(),
            detail: "Visual CSS generators render in Web Preview, not hand-built GPUI controls"
                .to_string(),
        },
    ];

    DxStylePanelSnapshot {
        root,
        root_exists,
        plan_path,
        plan_present,
        grouped_contract_path,
        grouped_contract_present,
        grouped_contract_ready,
        generator_catalog_path,
        generator_catalog_present,
        editor_contract_path,
        editor_contract_present,
        web_preview_host_path,
        web_preview_host_present,
        web_preview_bridge_ready,
        visual_generator_count,
        status,
        next_action,
        readiness,
        rows,
        warnings,
    }
}

fn read_text_limited(path: &Path) -> Option<String> {
    read_text_limited_to(path, MAX_TEXT_BYTES)
}

fn read_text_limited_to(path: &Path, max_bytes: u64) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > max_bytes {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn file_contains_all_markers_limited(path: &Path, markers: &[&str], max_bytes: u64) -> bool {
    if markers.is_empty() {
        return true;
    }
    let Ok(mut file) = File::open(path) else {
        return false;
    };

    let mut found = vec![false; markers.len()];
    let keep_bytes = markers
        .iter()
        .map(|marker| marker.len())
        .max()
        .unwrap_or_default()
        .saturating_sub(1);
    let mut scanned = 0u64;
    let mut carry = Vec::new();
    let mut buffer = [0u8; 8192];

    loop {
        let Ok(count) = file.read(&mut buffer) else {
            return false;
        };
        if count == 0 {
            return found.iter().all(|is_found| *is_found);
        }
        scanned = scanned.saturating_add(count as u64);
        if scanned > max_bytes {
            return false;
        }

        carry.extend_from_slice(&buffer[..count]);
        for (index, marker) in markers.iter().enumerate() {
            if !found[index] && contains_subslice(&carry, marker.as_bytes()) {
                found[index] = true;
            }
        }
        if found.iter().all(|is_found| *is_found) {
            return true;
        }

        if keep_bytes == 0 {
            carry.clear();
        } else if carry.len() > keep_bytes {
            let split_at = carry.len() - keep_bytes;
            carry.drain(..split_at);
        }
    }
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty()
        || haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn count_declared_visual_generators(plan: &str) -> usize {
    let Some(start) = plan.find("## First 25 Visual Generators") else {
        return 0;
    };
    let section = &plan[start..];
    let end = section.find("\n## ").unwrap_or(section.len());

    section[..end]
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            let Some((prefix, rest)) = trimmed.split_once(". ") else {
                return false;
            };
            !rest.trim().is_empty() && prefix.chars().all(|ch| ch.is_ascii_digit())
        })
        .count()
}

fn relative_or_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_first_visual_generator_section_only() {
        let plan = r#"
## First 25 Visual Generators

1. linear gradient
2. radial gradient
3. conic gradient

## Guardrails

1. not a generator
"#;

        assert_eq!(count_declared_visual_generators(plan), 3);
    }
}
