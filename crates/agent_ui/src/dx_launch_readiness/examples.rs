use super::{DxLaunchReadinessExample, DxLaunchReadinessSnapshot};
use serde_json::Value;

pub(super) fn push_recovery_commands(packet: &Value, snapshot: &mut DxLaunchReadinessSnapshot) {
    let Some(commands) = packet.get("recovery_commands").and_then(Value::as_object) else {
        return;
    };

    for command in commands.values().filter_map(Value::as_str) {
        push_unique(&mut snapshot.recovery_commands, command.to_string());
    }
}

pub(super) fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

pub(super) fn balanced_examples(
    examples: &[DxLaunchReadinessExample],
) -> Vec<DxLaunchReadinessExample> {
    let mut balanced = Vec::new();
    for prefix in ["Import ", "Gate ", "Fallback "] {
        if let Some(example) = examples
            .iter()
            .find(|example| example.label.starts_with(prefix))
        {
            balanced.push(example.clone());
        }
    }

    for example in examples {
        if balanced.len() >= 6 {
            break;
        }
        if !balanced
            .iter()
            .any(|existing| existing.label == example.label)
        {
            balanced.push(example.clone());
        }
    }

    balanced
}
