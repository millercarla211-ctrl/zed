use super::*;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn dx_agent_secret_marker_guard_covers_bridge_receipt_scalars() {
    for value in [
        "sk-should-not-render",
        "provider_key",
        "--provider-key",
        "--api-key=should-not-render",
        "--access-key",
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
    for (input, expected) in [
        ("dx-agents agents status --json", "dx agents status --json"),
        (
            "dx-agents providers list --json",
            "dx agents providers list --json",
        ),
        (
            "dx-agents models list --json",
            "dx agents models list --json",
        ),
        ("dx agents status --json", "dx agents status --json"),
    ] {
        assert_eq!(public_command_for_runtime(input), expected);
    }
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
        bridge_command_label("dx", &args(&["agents", "status", "--json"])),
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

#[test]
fn bridge_command_label_redacts_secret_like_args() {
    assert_eq!(
        bridge_command_label("dx", &args(&["agents", "run", "--token", "sk-value"])),
        "dx agents run <redacted> <redacted>"
    );
}

#[test]
fn bridge_command_label_redacts_secret_key_value_args() {
    assert_eq!(
        bridge_command_label(
            "dx",
            &args(&[
                "agents",
                "run",
                "--api-key=should-not-render",
                "--access-key",
                "plain-value",
                "--project",
                "demo",
            ]),
        ),
        "dx agents run <redacted> <redacted> <redacted> --project demo"
    );
}
