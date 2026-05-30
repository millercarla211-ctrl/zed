mod catalog;
mod controls;
mod css_declaration_dry_run_contract;
mod fixture;
mod group_context_contract;
mod recipes;
mod reverse_css_delta_contract;
mod script;
mod source_apply_contract;
mod style;

use catalog::dx_style_generator_catalog_json;
use controls::dx_style_generator_controls_json;
use css_declaration_dry_run_contract::dx_style_css_declaration_dry_run_contract_json;
use group_context_contract::dx_style_group_context_contract_json;
use recipes::dx_style_generator_recipes_json;
use reverse_css_delta_contract::dx_style_reverse_css_delta_contract_json;
use script::dx_style_generator_script;
use source_apply_contract::dx_style_source_apply_contract_json;
use style::dx_style_generator_css;

pub const DX_STYLE_GENERATOR_SURFACE_SCHEMA: &str = "zed.web_preview.dx_style_generator_surface.v1";
const ACTIVE_STYLE_CONTEXT_SCHEMA: &str = "zed.dx_style.active_context.v1";
const MAX_DX_STYLE_CONTEXT_JSON_BYTES: usize = 256 * 1024;

pub fn dx_style_generator_url() -> String {
    dx_style_generator_url_with_context(None)
}

pub fn dx_style_generator_url_with_context(source_context_json: Option<&str>) -> String {
    let source_context_json = bounded_source_context_json(source_context_json);
    let source_context_json = script_safe_json_string_literal(&source_context_json);
    let catalog_json = dx_style_generator_catalog_json();
    let controls_json = dx_style_generator_controls_json();
    let recipes_json = dx_style_generator_recipes_json();
    let source_apply_contract_json = dx_style_source_apply_contract_json();
    let css_declaration_dry_run_contract_json = dx_style_css_declaration_dry_run_contract_json();
    let group_context_contract_json = dx_style_group_context_contract_json();
    let reverse_css_delta_contract_json = dx_style_reverse_css_delta_contract_json();
    let generator_css = dx_style_generator_css();
    let generator_script = dx_style_generator_script()
        .replace("__DX_STYLE_GENERATOR_CATALOG_JSON__", &catalog_json)
        .replace("__DX_STYLE_GENERATOR_CONTROLS_JSON__", &controls_json)
        .replace("__DX_STYLE_GENERATOR_RECIPES_JSON__", &recipes_json)
        .replace(
            "__DX_STYLE_SOURCE_APPLY_CONTRACT_JSON__",
            &source_apply_contract_json,
        )
        .replace(
            "__DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_JSON__",
            &css_declaration_dry_run_contract_json,
        )
        .replace(
            "__DX_STYLE_GROUP_CONTEXT_CONTRACT_JSON__",
            &group_context_contract_json,
        )
        .replace(
            "__DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_JSON__",
            &reverse_css_delta_contract_json,
        )
        .replace("__DX_STYLE_CONTEXT_JSON_STRING__", &source_context_json);
    let html = DX_STYLE_GENERATOR_HTML
        .replace(
            "__DX_STYLE_GENERATOR_SURFACE_SCHEMA__",
            DX_STYLE_GENERATOR_SURFACE_SCHEMA,
        )
        .replace("__DX_STYLE_GENERATOR_CSS__", generator_css)
        .replace("__DX_STYLE_GENERATOR_SCRIPT__", &generator_script);
    format!(
        "data:text/html;charset=utf-8,{}",
        percent_encode_data_url(&html)
    )
}

const DX_STYLE_GENERATOR_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>DX Style Generators</title>
  <style>
__DX_STYLE_GENERATOR_CSS__
  </style>
</head>
<body>
  <main data-dx-style-generator-schema="__DX_STYLE_GENERATOR_SURFACE_SCHEMA__">
    <aside>
      <header>
        <h1>DX Style</h1>
        <h2>Web Preview generator surface</h2>
      </header>
      <label class="catalog-search">
        <span>Generator filter</span>
        <input id="generatorSearch" type="search" autocomplete="off" spellcheck="false" placeholder="gradient, shadow, grid..." aria-label="Filter visual generators">
      </label>
      <div id="catalog" class="catalog" aria-label="Visual generator catalog"></div>
    </aside>
    <section class="workspace">
      <div class="toolbar">
        <select id="generatorSelect" aria-label="Generator"></select>
        <button id="copyClassButton" type="button">Copy class</button>
        <button id="copyCssButton" type="button">Copy CSS</button>
        <button id="copyReviewButton" type="button">Copy review</button>
        <button id="reviewApplyButton" type="button" disabled>Review source</button>
        <button id="applyButton" type="button" disabled>Apply</button>
        <div id="sourceStatus" class="status">Source apply is gated by trusted spans.</div>
      </div>
      <div class="canvas">
        <div id="controls" class="controls"></div>
        <div class="preview-wrap">
          <div class="preview"><div id="sample" class="sample">DX Style Preview</div></div>
          <div id="patchReview" class="patch-review"></div>
          <pre id="output"></pre>
        </div>
      </div>
      <footer>
        <span class="pill">GPUI shell</span>
        <span class="pill">Web Preview canvas</span>
        <span id="metadataStatus" class="pill">Metadata checking</span>
        <span class="pill blocked">Apply disabled until dry-run patch receipts are trusted</span>
      </footer>
    </section>
  </main>
  <script>
__DX_STYLE_GENERATOR_SCRIPT__
  </script>
</body>
</html>"##;

fn percent_encode_data_url(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push(hex_digit(byte >> 4));
            encoded.push(hex_digit(byte & 0x0f));
        }
    }
    encoded
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + value - 10) as char,
        _ => unreachable!("hex digit nibble must be in range"),
    }
}

fn bounded_source_context_json(source_context_json: Option<&str>) -> String {
    let Some(source_context_json) = source_context_json else {
        return String::new();
    };
    if source_context_json.is_empty() {
        return String::new();
    }
    if source_context_json.len() > MAX_DX_STYLE_CONTEXT_JSON_BYTES {
        return blocked_source_context_json(
            "context payload too large",
            format!(
                "DX Style source context is {} byte(s), above the {} byte Web Preview limit.",
                source_context_json.len(),
                MAX_DX_STYLE_CONTEXT_JSON_BYTES
            ),
        );
    }

    match serde_json::from_str::<serde_json::Value>(source_context_json) {
        Ok(serde_json::Value::Object(_)) => source_context_json.to_string(),
        Ok(_) => blocked_source_context_json(
            "invalid context payload",
            "DX Style source context must be a JSON object.",
        ),
        Err(_) => blocked_source_context_json(
            "invalid context payload",
            "DX Style source context could not be parsed as JSON.",
        ),
    }
}

fn blocked_source_context_json(status: &str, detail: impl Into<String>) -> String {
    serde_json::json!({
        "schema": ACTIVE_STYLE_CONTEXT_SCHEMA,
        "status": status,
        "detail": detail.into(),
        "source_path": serde_json::Value::Null,
        "source_state": "source apply disabled until a valid editor context is available",
        "context_kind": serde_json::Value::Null,
        "token": serde_json::Value::Null,
        "css_property": serde_json::Value::Null,
        "css_generator": serde_json::Value::Null,
        "css_source_edit_safety": serde_json::Value::Null,
        "attribute_tokens": [],
        "group_context": serde_json::Value::Null,
        "span": serde_json::Value::Null,
        "source_span": serde_json::Value::Null,
        "source_digest": serde_json::Value::Null,
        "apply_gate": {
            "state": "needs_valid_style_context",
            "reason": "The Web Preview generator received an invalid or oversized editor context.",
            "can_enable_apply": false,
        },
        "source_apply": "disabled_until_valid_style_context",
    })
    .to_string()
}

fn script_safe_json_string_literal(value: &str) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"\"".into())
        .replace("</", "<\\/")
}
