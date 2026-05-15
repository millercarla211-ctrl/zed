use anyhow::{Context as _, Result};
use serde::Deserialize;
use std::{
    collections::{BTreeSet, HashSet},
    fs as std_fs,
    path::{Component, Path, PathBuf},
};
use workspace::{DraggedShadcnAsset, DraggedShadcnKind};

pub struct InstallReport {
    pub installed_path: String,
    pub added_dependencies: Vec<String>,
    pub css_path: String,
    pub wrote_theme_css: bool,
    pub package_install_command: Option<String>,
}

impl InstallReport {
    pub fn status_message(&self, title: &str) -> String {
        let mut message = format!("Installed {title}");
        if self.wrote_theme_css {
            message.push_str("; added shadcn theme CSS");
        }
        if !self.added_dependencies.is_empty() {
            message.push_str(&format!(
                "; added {} package deps",
                self.added_dependencies.len()
            ));
            if let Some(command) = &self.package_install_command {
                message.push_str(&format!("; run {command}"));
            }
        }
        message
    }
}

pub fn install_asset(asset: &DraggedShadcnAsset, project_root: &Path) -> Result<InstallReport> {
    let scaffold = ensure_scaffold(project_root, &asset.registry_root)?;

    let components_dir = components_dir(project_root);
    let mut dependency_specs = base_dependency_specs();

    match asset.kind {
        DraggedShadcnKind::Component => {
            if !install_registry_item_by_name(
                asset.id.as_ref(),
                project_root,
                &asset.registry_root,
                &mut dependency_specs,
                &mut HashSet::new(),
            )? {
                let destination = components_dir
                    .join("ui")
                    .join(asset.target_file_name.as_ref());
                copy_transformed_source_file(&asset.source_path, &destination)?;
            }
        }
        DraggedShadcnKind::Magic => {
            let destination = components_dir
                .join("magicui")
                .join(asset.target_file_name.as_ref());
            copy_transformed_source_file(&asset.source_path, &destination)?;
            if let Ok(content) = std_fs::read_to_string(&asset.source_path) {
                for dependency in project_ui_dependencies_from_content(&content) {
                    install_registry_item_by_name(
                        &dependency,
                        project_root,
                        &asset.registry_root,
                        &mut dependency_specs,
                        &mut HashSet::new(),
                    )?;
                }
            }
            for dependency in magic_dependency_specs(asset.id.as_ref()) {
                dependency_specs.insert(dependency.to_string());
            }
        }
        DraggedShadcnKind::Block => {
            if !install_registry_item_by_name(
                asset.id.as_ref(),
                project_root,
                &asset.registry_root,
                &mut dependency_specs,
                &mut HashSet::new(),
            )? {
                let destination = components_dir
                    .join("blocks")
                    .join(asset.target_file_name.as_ref());
                copy_transformed_source_dir(&asset.source_path, &destination)?;
            }
        }
    }

    let added_dependencies = ensure_package_json_dependencies(project_root, dependency_specs)?;

    Ok(InstallReport {
        installed_path: install_display(asset, project_root),
        package_install_command: if added_dependencies.is_empty() {
            None
        } else {
            package_install_command(project_root)
        },
        added_dependencies,
        css_path: project_relative_path(project_root, &scaffold.css_path),
        wrote_theme_css: scaffold.wrote_theme_css,
    })
}

pub fn install_display(asset: &DraggedShadcnAsset, project_root: &Path) -> String {
    let components_dir = components_dir(project_root);
    if matches!(
        asset.kind,
        DraggedShadcnKind::Component | DraggedShadcnKind::Block
    ) {
        if let Some(path) = registry_install_display_path(asset, project_root) {
            return path.to_string_lossy().replace('\\', "/");
        }
    }

    let path = match asset.kind {
        DraggedShadcnKind::Component => components_dir
            .join("ui")
            .join(asset.target_file_name.as_ref()),
        DraggedShadcnKind::Magic => components_dir
            .join("magicui")
            .join(asset.target_file_name.as_ref()),
        DraggedShadcnKind::Block => components_dir
            .join("blocks")
            .join(asset.target_file_name.as_ref()),
    };
    path.to_string_lossy().replace('\\', "/")
}

struct ScaffoldReport {
    css_path: PathBuf,
    wrote_theme_css: bool,
}

fn ensure_scaffold(project_root: &Path, registry_root: &Path) -> Result<ScaffoldReport> {
    let components_dir = components_dir(project_root);
    std_fs::create_dir_all(components_dir.join("ui"))
        .with_context(|| format!("creating {}", components_dir.join("ui").display()))?;
    let theme_css = ensure_theme_css(project_root)?;

    let utils_dir = lib_dir(project_root);
    std_fs::create_dir_all(&utils_dir)
        .with_context(|| format!("creating {}", utils_dir.display()))?;
    let utils_path = utils_dir.join("utils.ts");
    if !utils_path.exists() {
        let registry_utils = registry_root.join("lib").join("utils.ts");
        if registry_utils.is_file() {
            std_fs::copy(&registry_utils, &utils_path).with_context(|| {
                format!(
                    "copying shadcn utils from {} to {}",
                    registry_utils.display(),
                    utils_path.display()
                )
            })?;
        } else {
            std_fs::write(
                &utils_path,
                "import { clsx, type ClassValue } from \"clsx\"\nimport { twMerge } from \"tailwind-merge\"\n\nexport function cn(...inputs: ClassValue[]) {\n  return twMerge(clsx(inputs))\n}\n",
            )
            .with_context(|| format!("writing {}", utils_path.display()))?;
        }
    }

    let registry_hooks = registry_root.join("hooks");
    if registry_hooks.is_dir() {
        copy_transformed_source_dir(&registry_hooks, &hooks_dir(project_root))?;
    }

    let components_json = project_root.join("components.json");
    ensure_components_json(project_root, &components_json, &theme_css.path)?;

    Ok(ScaffoldReport {
        css_path: theme_css.path,
        wrote_theme_css: theme_css.wrote,
    })
}

fn ensure_components_json(
    project_root: &Path,
    components_json: &Path,
    css_path: &Path,
) -> Result<()> {
    let css = project_relative_path(project_root, css_path);
    if components_json.exists() {
        let text = std_fs::read_to_string(components_json)
            .with_context(|| format!("reading {}", components_json.display()))?;
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&text) else {
            return Ok(());
        };
        let Some(root) = value.as_object_mut() else {
            return Ok(());
        };

        let mut changed = false;
        let tailwind = root
            .entry("tailwind")
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if let Some(tailwind) = tailwind.as_object_mut() {
            if !matches!(
                tailwind
                .get("css")
                .and_then(|value| value.as_str())
                    .map(str::trim),
                Some(css) if !css.is_empty()
            ) {
                tailwind.insert("css".to_string(), serde_json::Value::String(css.clone()));
                changed = true;
            }
            if tailwind
                .get("cssVariables")
                .and_then(|value| value.as_bool())
                .is_none()
            {
                tailwind.insert("cssVariables".to_string(), serde_json::Value::Bool(true));
                changed = true;
            }
        }

        let aliases = root
            .entry("aliases")
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if let Some(aliases) = aliases.as_object_mut() {
            for (name, value) in [
                ("components", "@/components"),
                ("utils", "@/lib/utils"),
                ("ui", "@/components/ui"),
                ("lib", "@/lib"),
                ("hooks", "@/hooks"),
            ] {
                if !aliases.contains_key(name) {
                    aliases.insert(
                        name.to_string(),
                        serde_json::Value::String(value.to_string()),
                    );
                    changed = true;
                }
            }
        }

        if changed {
            let formatted = serde_json::to_string_pretty(&value)?;
            std_fs::write(components_json, format!("{formatted}\n"))
                .with_context(|| format!("writing {}", components_json.display()))?;
        }
        return Ok(());
    }

    let components = serde_json::json!({
        "$schema": "https://ui.shadcn.com/schema.json",
        "style": "new-york",
        "rsc": true,
        "tsx": true,
        "tailwind": {
            "config": "",
            "css": css,
            "baseColor": "zinc",
            "cssVariables": true
        },
        "aliases": {
            "components": "@/components",
            "utils": "@/lib/utils",
            "ui": "@/components/ui",
            "lib": "@/lib",
            "hooks": "@/hooks"
        }
    });
    let formatted = serde_json::to_string_pretty(&components)?;
    std_fs::write(components_json, format!("{formatted}\n"))
        .with_context(|| format!("writing {}", components_json.display()))?;

    Ok(())
}

#[derive(Deserialize)]
struct RegistryItem {
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(rename = "registryDependencies", default)]
    registry_dependencies: Vec<String>,
    #[serde(default)]
    files: Vec<RegistryFile>,
}

#[derive(Deserialize)]
struct RegistryFile {
    path: String,
    content: String,
    #[serde(rename = "type", default)]
    file_type: Option<String>,
    #[serde(default)]
    target: Option<String>,
}

fn install_registry_item_by_name(
    name: &str,
    project_root: &Path,
    registry_root: &Path,
    dependency_specs: &mut BTreeSet<String>,
    installed: &mut HashSet<String>,
) -> Result<bool> {
    if !installed.insert(name.to_string()) {
        return Ok(true);
    }

    let Some(item) = read_registry_item(name, registry_root)? else {
        return Ok(false);
    };

    for dependency in &item.dependencies {
        dependency_specs.insert(dependency.clone());
    }

    let mut registry_dependencies = BTreeSet::new();
    registry_dependencies.extend(item.registry_dependencies.iter().cloned());
    for file in &item.files {
        registry_dependencies.extend(registry_dependencies_from_content(&file.content));
    }

    for dependency in registry_dependencies {
        install_registry_item_by_name(
            &dependency,
            project_root,
            registry_root,
            dependency_specs,
            installed,
        )?;
    }

    for file in item.files {
        let Some(destination) = destination_for_registry_file(project_root, &file) else {
            continue;
        };
        write_transformed_content_if_absent(&destination, &file.content)?;
    }

    Ok(true)
}

fn read_registry_item(name: &str, registry_root: &Path) -> Result<Option<RegistryItem>> {
    let Some(v4_root) = registry_root.parent().and_then(|path| path.parent()) else {
        return Ok(None);
    };

    for style in ["new-york-v4", "new-york"] {
        let manifest_path = v4_root
            .join("public")
            .join("r")
            .join("styles")
            .join(style)
            .join(format!("{name}.json"));

        if !manifest_path.is_file() {
            continue;
        }

        let text = std_fs::read_to_string(&manifest_path)
            .with_context(|| format!("reading {}", manifest_path.display()))?;
        let item = serde_json::from_str::<RegistryItem>(&text)
            .with_context(|| format!("parsing {}", manifest_path.display()))?;
        return Ok(Some(item));
    }

    Ok(None)
}

fn registry_install_display_path(
    asset: &DraggedShadcnAsset,
    project_root: &Path,
) -> Option<PathBuf> {
    let item = read_registry_item(asset.id.as_ref(), &asset.registry_root)
        .ok()
        .flatten()?;
    let primary_file = primary_registry_file_for_install(&item, asset.kind)?;
    destination_for_registry_file(project_root, primary_file)
}

fn primary_registry_file_for_install(
    item: &RegistryItem,
    kind: DraggedShadcnKind,
) -> Option<&RegistryFile> {
    match kind {
        DraggedShadcnKind::Component => item
            .files
            .iter()
            .find(|file| registry_path_without_style_prefix(&file.path).starts_with("ui/"))
            .or_else(|| item.files.first()),
        DraggedShadcnKind::Block => item
            .files
            .iter()
            .find(|file| file.file_type.as_deref() == Some("registry:page"))
            .or_else(|| {
                item.files.iter().find(|file| {
                    let path = registry_path_without_style_prefix(&file.path);
                    path == "page.tsx" || path.ends_with("/page.tsx")
                })
            })
            .or_else(|| item.files.first()),
        DraggedShadcnKind::Magic => None,
    }
}

struct ThemeCssReport {
    path: PathBuf,
    wrote: bool,
}

fn ensure_theme_css(project_root: &Path) -> Result<ThemeCssReport> {
    let css_path =
        configured_css_path(project_root).unwrap_or_else(|| default_css_path(project_root));
    if let Some(parent) = css_path.parent() {
        std_fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }

    let existing = std_fs::read_to_string(&css_path).unwrap_or_default();
    if existing.contains("zed-shadcn-theme")
        || (existing.contains("--background:")
            && existing.contains("--foreground:")
            && existing.contains("--primary:")
            && existing.contains("--radius:"))
    {
        return Ok(ThemeCssReport {
            path: css_path,
            wrote: false,
        });
    }

    let uses_tailwind_v3 = existing.contains("@tailwind ");
    let include_tailwind_import =
        !existing.contains("@import \"tailwindcss\"") && !uses_tailwind_v3;
    let snippet = if uses_tailwind_v3 {
        shadcn_theme_css_v3()
    } else {
        shadcn_theme_css_v4(include_tailwind_import)
    };
    let next = if existing.trim().is_empty() {
        snippet
    } else {
        format!("{}\n\n{snippet}", existing.trim_end())
    };
    std_fs::write(&css_path, format!("{next}\n"))
        .with_context(|| format!("writing {}", css_path.display()))?;

    Ok(ThemeCssReport {
        path: css_path,
        wrote: true,
    })
}

fn configured_css_path(project_root: &Path) -> Option<PathBuf> {
    let components_json = project_root.join("components.json");
    let text = std_fs::read_to_string(components_json).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&text).ok()?;
    let css = value
        .get("tailwind")
        .and_then(|tailwind| tailwind.get("css"))
        .and_then(|css| css.as_str())?;
    if css.trim().is_empty() {
        return None;
    }
    Some(project_root.join(css))
}

fn default_css_path(project_root: &Path) -> PathBuf {
    for candidate in [
        "src/app/globals.css",
        "app/globals.css",
        "src/index.css",
        "src/App.css",
        "src/styles/globals.css",
        "styles/globals.css",
        "app.css",
    ] {
        let path = project_root.join(candidate);
        if path.is_file() {
            return path;
        }
    }

    let src_app = project_root.join("src").join("app");
    if src_app.is_dir() {
        return src_app.join("globals.css");
    }
    let app = project_root.join("app");
    if app.is_dir() {
        return app.join("globals.css");
    }
    let src = project_root.join("src");
    if src.is_dir() {
        return src.join("index.css");
    }

    project_root.join("src").join("app").join("globals.css")
}

fn project_relative_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn shadcn_theme_css_v4(include_tailwind_import: bool) -> String {
    let mut css = String::new();
    if include_tailwind_import {
        css.push_str("@import \"tailwindcss\";\n\n");
    }
    css.push_str(
        r#"/* zed-shadcn-theme */
@custom-variant dark (&:is(.dark *));

@theme inline {
  --radius-sm: calc(var(--radius) - 4px);
  --radius-md: calc(var(--radius) - 2px);
  --radius-lg: var(--radius);
  --radius-xl: calc(var(--radius) + 4px);
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --color-card: var(--card);
  --color-card-foreground: var(--card-foreground);
  --color-popover: var(--popover);
  --color-popover-foreground: var(--popover-foreground);
  --color-primary: var(--primary);
  --color-primary-foreground: var(--primary-foreground);
  --color-secondary: var(--secondary);
  --color-secondary-foreground: var(--secondary-foreground);
  --color-muted: var(--muted);
  --color-muted-foreground: var(--muted-foreground);
  --color-accent: var(--accent);
  --color-accent-foreground: var(--accent-foreground);
  --color-destructive: var(--destructive);
  --color-border: var(--border);
  --color-input: var(--input);
  --color-ring: var(--ring);
  --color-chart-1: var(--chart-1);
  --color-chart-2: var(--chart-2);
  --color-chart-3: var(--chart-3);
  --color-chart-4: var(--chart-4);
  --color-chart-5: var(--chart-5);
}

:root {
  --radius: 0.625rem;
  --background: oklch(1 0 0);
  --foreground: oklch(0.145 0 0);
  --card: oklch(1 0 0);
  --card-foreground: oklch(0.145 0 0);
  --popover: oklch(1 0 0);
  --popover-foreground: oklch(0.145 0 0);
  --primary: oklch(0.205 0 0);
  --primary-foreground: oklch(0.985 0 0);
  --secondary: oklch(0.97 0 0);
  --secondary-foreground: oklch(0.205 0 0);
  --muted: oklch(0.97 0 0);
  --muted-foreground: oklch(0.556 0 0);
  --accent: oklch(0.97 0 0);
  --accent-foreground: oklch(0.205 0 0);
  --destructive: oklch(0.577 0.245 27.325);
  --border: oklch(0.922 0 0);
  --input: oklch(0.922 0 0);
  --ring: oklch(0.708 0 0);
  --chart-1: oklch(0.646 0.222 41.116);
  --chart-2: oklch(0.6 0.118 184.704);
  --chart-3: oklch(0.398 0.07 227.392);
  --chart-4: oklch(0.828 0.189 84.429);
  --chart-5: oklch(0.769 0.188 70.08);
}

.dark {
  --background: oklch(0.145 0 0);
  --foreground: oklch(0.985 0 0);
  --card: oklch(0.205 0 0);
  --card-foreground: oklch(0.985 0 0);
  --popover: oklch(0.205 0 0);
  --popover-foreground: oklch(0.985 0 0);
  --primary: oklch(0.922 0 0);
  --primary-foreground: oklch(0.205 0 0);
  --secondary: oklch(0.269 0 0);
  --secondary-foreground: oklch(0.985 0 0);
  --muted: oklch(0.269 0 0);
  --muted-foreground: oklch(0.708 0 0);
  --accent: oklch(0.269 0 0);
  --accent-foreground: oklch(0.985 0 0);
  --destructive: oklch(0.704 0.191 22.216);
  --border: oklch(1 0 0 / 10%);
  --input: oklch(1 0 0 / 15%);
  --ring: oklch(0.556 0 0);
  --chart-1: oklch(0.488 0.243 264.376);
  --chart-2: oklch(0.696 0.17 162.48);
  --chart-3: oklch(0.769 0.188 70.08);
  --chart-4: oklch(0.627 0.265 303.9);
  --chart-5: oklch(0.645 0.246 16.439);
}

@layer base {
  * {
    @apply border-border outline-ring/50;
  }

  body {
    @apply bg-background text-foreground;
  }
}
"#,
    );
    css
}

fn shadcn_theme_css_v3() -> String {
    r#"/* zed-shadcn-theme */
@layer base {
  :root {
    --radius: 0.625rem;
    --background: 0 0% 100%;
    --foreground: 240 10% 3.9%;
    --card: 0 0% 100%;
    --card-foreground: 240 10% 3.9%;
    --popover: 0 0% 100%;
    --popover-foreground: 240 10% 3.9%;
    --primary: 240 5.9% 10%;
    --primary-foreground: 0 0% 98%;
    --secondary: 240 4.8% 95.9%;
    --secondary-foreground: 240 5.9% 10%;
    --muted: 240 4.8% 95.9%;
    --muted-foreground: 240 3.8% 46.1%;
    --accent: 240 4.8% 95.9%;
    --accent-foreground: 240 5.9% 10%;
    --destructive: 0 84.2% 60.2%;
    --destructive-foreground: 0 0% 98%;
    --border: 240 5.9% 90%;
    --input: 240 5.9% 90%;
    --ring: 240 5.9% 10%;
    --chart-1: 12 76% 61%;
    --chart-2: 173 58% 39%;
    --chart-3: 197 37% 24%;
    --chart-4: 43 74% 66%;
    --chart-5: 27 87% 67%;
  }

  .dark {
    --background: 240 10% 3.9%;
    --foreground: 0 0% 98%;
    --card: 240 10% 3.9%;
    --card-foreground: 0 0% 98%;
    --popover: 240 10% 3.9%;
    --popover-foreground: 0 0% 98%;
    --primary: 0 0% 98%;
    --primary-foreground: 240 5.9% 10%;
    --secondary: 240 3.7% 15.9%;
    --secondary-foreground: 0 0% 98%;
    --muted: 240 3.7% 15.9%;
    --muted-foreground: 240 5% 64.9%;
    --accent: 240 3.7% 15.9%;
    --accent-foreground: 0 0% 98%;
    --destructive: 0 62.8% 30.6%;
    --destructive-foreground: 0 0% 98%;
    --border: 240 3.7% 15.9%;
    --input: 240 3.7% 15.9%;
    --ring: 240 4.9% 83.9%;
    --chart-1: 220 70% 50%;
    --chart-2: 160 60% 45%;
    --chart-3: 30 80% 55%;
    --chart-4: 280 65% 60%;
    --chart-5: 340 75% 55%;
  }
}
"#
    .to_string()
}

fn destination_for_registry_file(project_root: &Path, file: &RegistryFile) -> Option<PathBuf> {
    if let Some(destination) = file
        .target
        .as_deref()
        .and_then(|target| project_relative_target_path(project_root, target))
    {
        return Some(destination);
    }

    destination_for_registry_path(project_root, &file.path)
}

fn destination_for_registry_path(project_root: &Path, registry_path: &str) -> Option<PathBuf> {
    const UI_PREFIX: &str = "ui/";
    const BLOCK_PREFIX: &str = "blocks/";
    const CHART_PREFIX: &str = "charts/";
    const EXAMPLE_PREFIX: &str = "examples/";
    const INTERNAL_PREFIX: &str = "internal/";
    const HOOK_PREFIX: &str = "hooks/";
    const LIB_PREFIX: &str = "lib/";
    let registry_path = registry_path_without_style_prefix(registry_path);

    if let Some(rest) = registry_path.strip_prefix(UI_PREFIX) {
        return Some(components_dir(project_root).join("ui").join(rest));
    }
    if let Some(rest) = registry_path.strip_prefix(BLOCK_PREFIX) {
        return Some(components_dir(project_root).join("blocks").join(rest));
    }
    if let Some(rest) = registry_path.strip_prefix(CHART_PREFIX) {
        return Some(
            components_dir(project_root)
                .join("blocks")
                .join("charts")
                .join(rest),
        );
    }
    if let Some(rest) = registry_path.strip_prefix(EXAMPLE_PREFIX) {
        return Some(
            components_dir(project_root)
                .join("blocks")
                .join("examples")
                .join(rest),
        );
    }
    if let Some(rest) = registry_path.strip_prefix(INTERNAL_PREFIX) {
        return Some(
            components_dir(project_root)
                .join("blocks")
                .join("internal")
                .join(rest),
        );
    }
    if let Some(rest) = registry_path.strip_prefix(HOOK_PREFIX) {
        return Some(hooks_dir(project_root).join(rest));
    }
    if let Some(rest) = registry_path.strip_prefix(LIB_PREFIX) {
        return Some(lib_dir(project_root).join(rest));
    }

    None
}

fn project_relative_target_path(project_root: &Path, target: &str) -> Option<PathBuf> {
    let target = target.trim();
    if target.is_empty() {
        return None;
    }

    let mut relative_path = PathBuf::new();
    for component in Path::new(target).components() {
        match component {
            Component::Normal(segment) => relative_path.push(segment),
            _ => return None,
        }
    }

    if relative_path.as_os_str().is_empty() {
        return None;
    }

    Some(project_root.join(relative_path))
}

fn registry_path_without_style_prefix(path: &str) -> &str {
    path.strip_prefix("registry/new-york-v4/")
        .or_else(|| path.strip_prefix("registry/new-york/"))
        .or_else(|| path.strip_prefix("registry/default/"))
        .unwrap_or(path)
}

fn write_transformed_content_if_absent(destination: &Path, content: &str) -> Result<()> {
    if destination.exists() {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        std_fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }

    std_fs::write(destination, rewrite_imports(content))
        .with_context(|| format!("writing {}", destination.display()))?;

    Ok(())
}

fn registry_dependencies_from_content(content: &str) -> BTreeSet<String> {
    let mut dependencies = BTreeSet::new();
    for prefix in [
        "@/registry/new-york-v4/ui/",
        "@/registry/new-york/ui/",
        "@/registry/default/ui/",
        "@/registry/new-york-v4/blocks/",
        "@/registry/new-york/blocks/",
        "@/registry/default/blocks/",
        "@/registry/new-york-v4/hooks/",
        "@/registry/new-york/hooks/",
        "@/registry/default/hooks/",
    ] {
        collect_registry_dependency_prefix(content, prefix, &mut dependencies);
    }
    dependencies
}

fn project_ui_dependencies_from_content(content: &str) -> BTreeSet<String> {
    let mut dependencies = BTreeSet::new();
    collect_registry_dependency_prefix(content, "@/components/ui/", &mut dependencies);
    dependencies
}

fn collect_registry_dependency_prefix(
    content: &str,
    prefix: &str,
    dependencies: &mut BTreeSet<String>,
) {
    let mut remaining = content;
    while let Some(start) = remaining.find(prefix) {
        let after = &remaining[start + prefix.len()..];
        let dependency = after
            .split(|character| matches!(character, '/' | '"' | '\'' | '`'))
            .next()
            .unwrap_or_default();
        if !dependency.is_empty() {
            dependencies.insert(dependency.to_string());
        }
        remaining = after;
    }
}

fn ensure_package_json_dependencies(
    project_root: &Path,
    dependency_specs: BTreeSet<String>,
) -> Result<Vec<String>> {
    let package_json = project_root.join("package.json");
    if !package_json.is_file() {
        return Ok(Vec::new());
    }

    let text = std_fs::read_to_string(&package_json)
        .with_context(|| format!("reading {}", package_json.display()))?;
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Ok(Vec::new());
    };
    let Some(root) = value.as_object_mut() else {
        return Ok(Vec::new());
    };

    let dependencies = root
        .entry("dependencies")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let Some(dependencies) = dependencies.as_object_mut() else {
        return Ok(Vec::new());
    };

    let mut changed = false;
    let mut added_dependencies = Vec::new();
    for spec in dependency_specs {
        let (name, version) = split_package_spec(&spec);
        let version = version.unwrap_or_else(|| default_dependency_version(name));
        if !dependencies.contains_key(name) {
            dependencies.insert(
                name.to_string(),
                serde_json::Value::String(version.into_owned()),
            );
            added_dependencies.push(name.to_string());
            changed = true;
        }
    }

    if changed {
        let formatted = serde_json::to_string_pretty(&value)?;
        std_fs::write(&package_json, format!("{formatted}\n"))
            .with_context(|| format!("writing {}", package_json.display()))?;
    }

    Ok(added_dependencies)
}

fn base_dependency_specs() -> BTreeSet<String> {
    [
        "class-variance-authority",
        "clsx",
        "lucide-react",
        "radix-ui",
        "tailwind-merge",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn magic_dependency_specs(name: &str) -> &'static [&'static str] {
    match name {
        "bento-grid" => &["@radix-ui/react-icons"],
        "rainbow-button" => &["@radix-ui/react-slot", "class-variance-authority"],
        "magic-card" => &["motion"],
        _ => &[],
    }
}

fn split_package_spec(spec: &str) -> (&str, Option<std::borrow::Cow<'static, str>>) {
    if spec.starts_with('@') {
        if let Some(package_separator) = spec.find('/') {
            if let Some(version_separator) = spec[package_separator + 1..].rfind('@') {
                let split = package_separator + 1 + version_separator;
                return (
                    &spec[..split],
                    Some(std::borrow::Cow::Owned(spec[split + 1..].to_string())),
                );
            }
        }
    } else if let Some(index) = spec.rfind('@') {
        if index > 0 {
            return (
                &spec[..index],
                Some(std::borrow::Cow::Owned(spec[index + 1..].to_string())),
            );
        }
    }

    (spec, None)
}

fn default_dependency_version(name: &str) -> std::borrow::Cow<'static, str> {
    match name {
        "@dnd-kit/core" => "^6.3.1".into(),
        "@dnd-kit/modifiers" => "^9.0.0".into(),
        "@dnd-kit/sortable" => "^10.0.0".into(),
        "@dnd-kit/utilities" => "^3.2.2".into(),
        "@hookform/resolvers" => "^3.10.0".into(),
        "@radix-ui/react-icons" => "^1.3.2".into(),
        "@radix-ui/react-slot" => "^1.2.3".into(),
        "@tabler/icons-react" => "^3.31.0".into(),
        "@tanstack/react-table" => "^8.9.1".into(),
        "class-variance-authority" => "^0.7.1".into(),
        "clsx" => "^2.1.1".into(),
        "cmdk" => "^1.1.1".into(),
        "date-fns" => "^4.1.0".into(),
        "embla-carousel-react" => "8.5.2".into(),
        "input-otp" => "^1.4.2".into(),
        "lucide-react" => "0.474.0".into(),
        "motion" => "^12.12.1".into(),
        "next-themes" => "0.4.6".into(),
        "radix-ui" => "^1.4.3".into(),
        "react-day-picker" => "^9.7.0".into(),
        "react-hook-form" => "^7.62.0".into(),
        "react-resizable-panels" => "^4".into(),
        "recharts" => "3.8.0".into(),
        "sonner" => "^2.0.0".into(),
        "tailwind-merge" => "^3.0.1".into(),
        "vaul" => "1.1.2".into(),
        "zod" => "^3.25.76".into(),
        _ => "latest".into(),
    }
}

fn package_install_command(project_root: &Path) -> Option<String> {
    if !project_root.join("package.json").is_file() {
        return None;
    }

    if project_root.join("bun.lockb").is_file() || project_root.join("bun.lock").is_file() {
        return Some("bun install".to_string());
    }
    if project_root.join("pnpm-lock.yaml").is_file() {
        return Some("pnpm install".to_string());
    }
    if project_root.join("yarn.lock").is_file() {
        return Some("yarn install".to_string());
    }
    if project_root.join("package-lock.json").is_file() {
        return Some("npm install".to_string());
    }

    Some("npm install".to_string())
}

fn copy_transformed_source_dir(source_dir: &Path, destination_dir: &Path) -> Result<()> {
    std_fs::create_dir_all(destination_dir)
        .with_context(|| format!("creating {}", destination_dir.display()))?;
    for entry in std_fs::read_dir(source_dir)
        .with_context(|| format!("reading {}", source_dir.display()))?
        .flatten()
    {
        let source = entry.path();
        let destination = destination_dir.join(entry.file_name());
        if source.is_dir() {
            copy_transformed_source_dir(&source, &destination)?;
        } else {
            copy_transformed_source_file(&source, &destination)?;
        }
    }

    Ok(())
}

fn copy_transformed_source_file(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        std_fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }

    let extension = source.extension().and_then(|extension| extension.to_str());
    if matches!(
        extension,
        Some("ts" | "tsx" | "js" | "jsx" | "css" | "mdx" | "json")
    ) {
        let text = std_fs::read_to_string(source)
            .with_context(|| format!("reading {}", source.display()))?;
        std_fs::write(destination, rewrite_imports(&text))
            .with_context(|| format!("writing {}", destination.display()))?;
    } else {
        std_fs::copy(source, destination).with_context(|| {
            format!(
                "copying shadcn asset from {} to {}",
                source.display(),
                destination.display()
            )
        })?;
    }

    Ok(())
}

fn rewrite_imports(text: &str) -> String {
    text.replace("@/registry/new-york-v4/ui/", "@/components/ui/")
        .replace("@/registry/new-york/ui/", "@/components/ui/")
        .replace("@/registry/default/ui/", "@/components/ui/")
        .replace("@/registry/new-york-v4/lib/", "@/lib/")
        .replace("@/registry/new-york/lib/", "@/lib/")
        .replace("@/registry/default/lib/", "@/lib/")
        .replace("@/registry/new-york-v4/hooks/", "@/hooks/")
        .replace("@/registry/new-york/hooks/", "@/hooks/")
        .replace("@/registry/default/hooks/", "@/hooks/")
        .replace("@/registry/new-york-v4/blocks/", "@/components/blocks/")
        .replace("@/registry/new-york/blocks/", "@/components/blocks/")
        .replace("@/registry/default/blocks/", "@/components/blocks/")
        .replace(
            "@/registry/new-york-v4/charts/",
            "@/components/blocks/charts/",
        )
        .replace("@/registry/new-york/charts/", "@/components/blocks/charts/")
        .replace("@/registry/default/charts/", "@/components/blocks/charts/")
        .replace(
            "@/registry/new-york-v4/examples/",
            "@/components/blocks/examples/",
        )
        .replace(
            "@/registry/new-york/examples/",
            "@/components/blocks/examples/",
        )
        .replace(
            "@/registry/default/examples/",
            "@/components/blocks/examples/",
        )
        .replace(
            "@/registry/new-york-v4/internal/",
            "@/components/blocks/internal/",
        )
        .replace(
            "@/registry/new-york/internal/",
            "@/components/blocks/internal/",
        )
        .replace(
            "@/registry/default/internal/",
            "@/components/blocks/internal/",
        )
        .replace("@/registry/bases/radix/ui/", "@/components/ui/")
        .replace("@/registry/bases/radix/lib/", "@/lib/")
        .replace("@/registry/magicui/", "@/components/magicui/")
}

fn components_dir(project_root: &Path) -> PathBuf {
    let src_dir = project_root.join("src");
    if src_dir.is_dir() {
        src_dir.join("components")
    } else {
        project_root.join("components")
    }
}

fn lib_dir(project_root: &Path) -> PathBuf {
    let src_dir = project_root.join("src");
    if src_dir.is_dir() {
        src_dir.join("lib")
    } else {
        project_root.join("lib")
    }
}

fn hooks_dir(project_root: &Path) -> PathBuf {
    let src_dir = project_root.join("src");
    if src_dir.is_dir() {
        src_dir.join("hooks")
    } else {
        project_root.join("hooks")
    }
}
