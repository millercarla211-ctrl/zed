use serde_json::Value;

pub(super) fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

pub(super) fn usize_field(value: &Value, field: &str) -> Option<usize> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| value.try_into().ok())
}

pub(super) fn bool_field(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_bool).unwrap_or(false)
}

pub(super) fn pointer_string<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

pub(super) fn pointer_usize(value: &Value, pointer: &str) -> Option<usize> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .and_then(|value| value.try_into().ok())
}

pub(super) fn packet_status(packet: &Value) -> String {
    string_field(packet, "status")
        .unwrap_or("unknown")
        .to_string()
}
