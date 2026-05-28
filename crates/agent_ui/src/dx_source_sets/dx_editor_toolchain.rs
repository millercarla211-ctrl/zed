use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use super::{
    DxSourceItem, DxSourceKind, DxSourceSet,
    formatting::{display_name, source_set_status},
};

const MAX_DX_CONFIG_BYTES: u64 = 128 * 1024;
const MAX_SCAN_ENTRIES: usize = 256;

pub(super) fn dx_editor_toolchain_set(workspace_roots: &[PathBuf]) -> DxSourceSet {
    let mut sources = workspace_roots
        .iter()
        .filter_map(dx_editor_toolchain_source)
        .collect::<Vec<_>>();
    sources.truncate(4);

    DxSourceSet {
        label: "DX Config",
        status: source_set_status(workspace_roots, &sources, "No extensionless dx config"),
        sources,
    }
}

fn dx_editor_toolchain_source(root: &PathBuf) -> Option<DxSourceItem> {
    let config_path = root.join("dx");
    if !config_path.is_file() {
        return None;
    }

    let config = read_bounded_utf8(&config_path)?;
    let serializer_dir = root.join(".dx").join("serializer");
    let machine_count = count_files_with_extension(&serializer_dir, "machine");
    let sr_count = count_sr_files(&root.join(".dx"));
    let is_www = config_has_section(&config, "www")
        || config.contains("runtime=dx-www")
        || config.contains("foundation=dx-www");

    let mut proofs = vec!["extensionless dx config detected".to_string()];
    proofs.extend(
        save_triggers_for_config(is_www)
            .into_iter()
            .map(|trigger| trigger.summary()),
    );

    if is_www {
        proofs.push("www output rooted under .dx/www/output".to_string());
    }

    if config.contains("tools[") && config.contains("serializer") {
        proofs.push("serializer tool declared in dx config".to_string());
    }
    if config.contains("score_scale=500") {
        proofs.push("dx check 500-point score scale declared".to_string());
    }
    if config.contains("lighthouse=true") || config.contains("dx check web-perf") {
        proofs.push("lighthouse score path declared through dx check".to_string());
    }

    let mut warnings = Vec::new();
    if !serializer_dir.is_dir() {
        warnings.push(".dx/serializer machine cache is not present yet".to_string());
    } else if machine_count == 0 {
        warnings.push(".dx/serializer has no .machine files yet".to_string());
    }
    if is_www && !config_declares_tsx_style_watch(&config) {
        warnings.push("tsx style/icon watch plan is not declared in dx config".to_string());
    }
    if is_www && !config.contains(".dx/www/output") {
        warnings.push("www output_dir is not visibly rooted at .dx/www/output".to_string());
    }
    if is_www && !config.contains("score_scale=500") {
        warnings.push("dx check 500-point score scale is not visible in dx config".to_string());
    }

    Some(DxSourceItem {
        label: display_name(root),
        detail: format!(
            "{} config - {machine_count} machine cache(s) - {sr_count} .sr file(s)",
            if is_www { "www" } else { "dx" },
        ),
        path: config_path.display().to_string(),
        kind: DxSourceKind::DxToolchainConfig,
        receipt_drilldowns: Vec::new(),
        proofs,
        warnings,
    })
}

struct DxEditorSaveTrigger {
    file_rule: &'static str,
    content_rule: &'static str,
    command: &'static str,
}

impl DxEditorSaveTrigger {
    fn summary(&self) -> String {
        format!(
            "{} when {} -> {}",
            self.file_rule, self.content_rule, self.command
        )
    }
}

fn save_triggers_for_config(is_www: bool) -> Vec<DxEditorSaveTrigger> {
    let mut triggers = vec![DxEditorSaveTrigger {
        file_rule: "dx or .sr edit",
        content_rule: "serializer source changes",
        command: "dx serializer",
    }];

    if is_www {
        triggers.extend([
            DxEditorSaveTrigger {
                file_rule: ".tsx edit",
                content_rule: "className/class tokens change",
                command: "dx style build",
            },
            DxEditorSaveTrigger {
                file_rule: ".tsx edit",
                content_rule: "<icon ...> tag appears",
                command: "dx icons sync",
            },
        ]);
    }

    triggers
}

fn read_bounded_utf8(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(MAX_DX_CONFIG_BYTES + 1)
        .read_to_end(&mut buffer)
        .ok()?;
    if buffer.len() as u64 > MAX_DX_CONFIG_BYTES {
        return None;
    }
    String::from_utf8(buffer).ok()
}

fn config_has_section(config: &str, section_name: &str) -> bool {
    config
        .lines()
        .map(str::trim_start)
        .any(|line| line.starts_with(section_name) && line.contains('('))
}

fn config_declares_tsx_style_watch(config: &str) -> bool {
    config.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("style ") && trimmed.contains("tsx")
    }) && config.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("icons ") && trimmed.contains("tsx")
    })
}

fn count_files_with_extension(dir: &Path, extension: &str) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };

    entries
        .flatten()
        .take(MAX_SCAN_ENTRIES)
        .filter(|entry| {
            entry.path().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        })
        .count()
}

fn count_sr_files(root: &Path) -> usize {
    let mut count = 0;
    let mut stack = vec![root.to_path_buf()];
    let mut visited = 0;

    while let Some(dir) = stack.pop() {
        if visited >= MAX_SCAN_ENTRIES {
            break;
        }
        visited += 1;

        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten().take(MAX_SCAN_ENTRIES) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("sr"))
            {
                count += 1;
            }
        }
    }

    count
}
