pub(crate) fn redact_action_scalar(value: &str) -> String {
    if is_secret_like_arg(value) {
        "<redacted>".to_string()
    } else {
        value.to_string()
    }
}

pub(crate) fn is_secret_like_arg(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let normalized = lower.replace('-', "_");
    DX_AGENT_SECRET_MARKERS
        .iter()
        .any(|marker| lower.contains(marker) || normalized.contains(marker))
}

pub(crate) fn public_command_for_runtime(command: &str) -> String {
    command
        .strip_prefix("dx-agents agents ")
        .map(|args| format!("dx agents {args}"))
        .or_else(|| {
            command
                .strip_prefix("dx-agents providers ")
                .map(|args| format!("dx agents providers {args}"))
        })
        .or_else(|| {
            command
                .strip_prefix("dx-agents models ")
                .map(|args| format!("dx agents models {args}"))
        })
        .unwrap_or_else(|| command.to_string())
}

pub(crate) fn is_public_dx_agents_command(command: &str) -> bool {
    command.starts_with("dx agents ")
}

pub(crate) fn is_dx_agents_command(command: &str, args: &str) -> bool {
    command == format!("dx-agents agents {args}") || command == format!("dx agents {args}")
}

pub(crate) fn is_safe_platform_arg(platform: &str) -> bool {
    !platform.trim().is_empty()
        && platform.len() <= 64
        && !is_secret_like_arg(platform)
        && platform
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

pub(crate) fn bridge_command_label(cli_path: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(cli_path.to_string());
    let mut redact_next = false;
    for arg in args {
        if redact_next {
            parts.push("<redacted>".to_string());
            redact_next = false;
            continue;
        }

        let redacted = redact_action_scalar(arg);
        redact_next = is_secret_flag_arg(arg);
        parts.push(redacted);
    }
    parts.join(" ")
}

fn is_secret_flag_arg(arg: &str) -> bool {
    arg.starts_with('-') && !arg.contains('=') && is_secret_like_arg(arg)
}

const DX_AGENT_SECRET_MARKERS: &[&str] = &[
    "sk-",
    "secret",
    "token",
    "password",
    "passwd",
    "cookie",
    "authorization",
    "bearer ",
    "api_key",
    "apikey",
    "provider_key",
    "access_key",
    "access_token",
    "refresh_token",
    "private-token",
    "xoxb-",
    "xoxp-",
];

#[cfg(test)]
#[path = "command_safety_tests.rs"]
mod tests;
