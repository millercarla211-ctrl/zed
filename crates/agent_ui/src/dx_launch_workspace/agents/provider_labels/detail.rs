use super::text::nonblank_or;

pub(crate) fn provider_detail_label(id: &str, compatibility: &[String]) -> String {
    let id = nonblank_or(id, "unknown-provider");
    let compatibility = compatibility_label(compatibility);

    if compatibility.is_empty() {
        id
    } else {
        format!("{id} - {compatibility}")
    }
}

pub(crate) fn model_detail_label(provider_id: &str, id: &str, compatibility: &[String]) -> String {
    let provider_id = nonblank_or(provider_id, "unknown-provider");
    let id = nonblank_or(id, "unknown-model");
    let compatibility = compatibility_label(compatibility);

    if compatibility.is_empty() {
        format!("{provider_id} / {id}")
    } else {
        format!("{provider_id} / {id} - {compatibility}")
    }
}

fn compatibility_label(compatibility: &[String]) -> String {
    let values = compatibility
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let shown = values
        .iter()
        .take(3)
        .copied()
        .collect::<Vec<_>>()
        .join(", ");

    match values.len().saturating_sub(3) {
        0 => shown,
        hidden => format!("{shown} (+{hidden} more)"),
    }
}
