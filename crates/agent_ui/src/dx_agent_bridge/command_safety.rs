pub(crate) fn redact_action_scalar(value: &str) -> String {
    if is_secret_like_arg(value) {
        "<redacted>".to_string()
    } else {
        value.to_string()
    }
}

pub(crate) fn is_secret_like_arg(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    DX_AGENT_SECRET_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
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
    parts.extend(args.iter().cloned());
    parts.join(" ")
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
mod tests {
    use super::*;

    #[test]
    fn dx_agent_secret_marker_guard_covers_bridge_receipt_scalars() {
        for value in [
            "sk-should-not-render",
            "provider_key",
            "bearer should-not-render",
            "authorization header",
            "private-token-value",
            "refresh_token",
            "password",
        ] {
            assert!(is_secret_like_arg(value), "{value} should be secret-like");
        }

        assert!(!is_secret_like_arg("telegram"));
        assert!(!is_secret_like_arg("dx agents status --json"));
    }

    #[test]
    fn public_command_for_runtime_maps_legacy_dx_agents_commands() {
        assert_eq!(
            public_command_for_runtime("dx-agents agents status --json"),
            "dx agents status --json"
        );
        assert_eq!(
            public_command_for_runtime("dx-agents providers list --json"),
            "dx agents providers list --json"
        );
        assert_eq!(
            public_command_for_runtime("dx-agents models list --json"),
            "dx agents models list --json"
        );
        assert_eq!(
            public_command_for_runtime("dx agents status --json"),
            "dx agents status --json"
        );
    }

    #[test]
    fn safe_platform_args_refuse_secrets_and_shell_shapes() {
        assert!(is_safe_platform_arg("github"));
        assert!(is_safe_platform_arg("linear.app"));
        assert!(!is_safe_platform_arg(""));
        assert!(!is_safe_platform_arg(" bearer bad"));
        assert!(!is_safe_platform_arg("github;remove"));
    }

    #[test]
    fn public_command_guards_and_labels_are_explicit() {
        assert!(is_public_dx_agents_command("dx agents status --json"));
        assert!(!is_public_dx_agents_command(
            "dx-agents agents status --json"
        ));
        assert!(is_dx_agents_command(
            "dx-agents agents social list --json",
            "social list --json"
        ));
        assert!(is_dx_agents_command(
            "dx agents social list --json",
            "social list --json"
        ));
        assert_eq!(
            bridge_command_label(
                "dx",
                &[
                    "agents".to_string(),
                    "status".to_string(),
                    "--json".to_string()
                ]
            ),
            "dx agents status --json"
        );
    }

    #[test]
    fn redact_action_scalar_masks_secret_like_values_only() {
        assert_eq!(redact_action_scalar("provider_key"), "<redacted>");
        assert_eq!(
            redact_action_scalar("dx agents status --json"),
            "dx agents status --json"
        );
    }
}
