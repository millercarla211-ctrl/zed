use serde_json::Value;

pub(super) fn command_fanout_count(value: &Value) -> usize {
    match value {
        Value::Array(items) => items.iter().map(command_fanout_count).sum(),
        Value::Object(object) => {
            let here = if object
                .get("command_fanout")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                1
            } else {
                0
            };
            here + object.values().map(command_fanout_count).sum::<usize>()
        }
        _ => 0,
    }
}

pub(super) fn redaction_requires_review(value: &Value) -> bool {
    let Some(redaction) = value.get("redaction") else {
        return true;
    };

    [
        "exports_source_file_contents",
        "exports_source_file_paths",
        "exports_secret_values",
        "exports_receipt_bodies",
        "exports_prompts",
        "exports_transcripts",
        "exports_command_payloads",
    ]
    .into_iter()
    .any(|field| {
        redaction
            .get(field)
            .and_then(Value::as_bool)
            .unwrap_or(true)
    })
}
