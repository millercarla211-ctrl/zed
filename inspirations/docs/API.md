# Flow API Documentation

## Library Usage

Flow is designed to be embedded directly into Rust hosts.

Add to your `Cargo.toml`:

```toml
[dependencies]
flow = "0.1.0"
tokio = { version = "1.50", features = ["rt-multi-thread", "macros"] }
```

## Recommended Local Runtime API

The primary integration surface for editor hosts and desktop shells is `FlowLocalRuntime`.

```rust
use anyhow::Result;
use flow::FlowLocalRuntime;

#[tokio::main]
async fn main() -> Result<()> {
    let runtime = FlowLocalRuntime::detect()?;

    println!("chat model: {:?}", runtime.default_text_model_key());
    println!("stt ready: {}", runtime.stt_ready());
    println!("tts ready: {}", runtime.tts_ready());

    let reply = runtime.generate_text("Summarize why local inference matters.").await?;
    println!("{}", reply);

    Ok(())
}
```

## Full Local Speech Pipeline

```rust
use anyhow::Result;
use flow::FlowLocalRuntime;

#[tokio::main]
async fn main() -> Result<()> {
    let runtime = FlowLocalRuntime::detect()?;

    let cleanup = runtime
        .transcribe_and_clean_file("input.wav")
        .await?;

    let spoken = runtime
        .synthesize_text_to_file(&cleanup.cleaned_text, "output.wav")
        .await?;

    println!("raw: {}", cleanup.raw_transcript);
    println!("cleaned: {}", cleanup.cleaned_text);
    println!("samples: {}", spoken.samples.len());

    Ok(())
}
```

## DX Host Integration

`DxFlowRuntime` can construct the embeddable local runtime for a host:

```rust
use anyhow::Result;
use flow::DxFlowRuntime;

fn main() -> Result<()> {
    let dx = DxFlowRuntime::detect();
    let summary = dx.local_runtime_summary()?;

    println!("selected chat model: {:?}", summary.chat.model_key);
    Ok(())
}
```

`DxFlowRuntime` can also emit a production-ready configuration bundle for each supported host target:

```rust
use anyhow::Result;
use flow::{DxFlowRuntime, FlowIntegrationTarget};

fn main() -> Result<()> {
    let dx = DxFlowRuntime::detect();
    let json = dx.production_config_json(FlowIntegrationTarget::CodexFork)?;

    println!("{}", json);
    Ok(())
}
```

Additional DX host helpers:

- `DxFlowRuntime::all_production_configs()`
  - builds recommended production configs for all supported host targets in one call
- `DxFlowRuntime::production_bundle_manifest()`
  - builds a machine-specific production handoff manifest without writing files
- `DxFlowRuntime::export_production_bundle(output_dir)`
  - exports all production target configs plus `manifest.json` and `README.txt` into a directory for client delivery and downstream host integration
- `DxFlowRuntime::release_summary()`
  - builds a repo-level release handoff summary from the checked-in configs and packaged browser artifacts
- `DxFlowRuntime::export_release_summary(output_dir)`
  - exports `flow-release-summary.json` and `FLOW_RELEASE_HANDOFF.md` for client delivery

## Zed-Focused Integration

For a Zed fork, use `ZedFlowAdapter` instead of wiring prompt templates by hand:

```rust
use anyhow::Result;
use flow::{
    ZedAgentPanelRequest, ZedAgentProfile, ZedFlowAdapter, ZedToolPermissionMode,
};

#[tokio::main]
async fn main() -> Result<()> {
    let zed = ZedFlowAdapter::detect()?;

    let response = zed
        .agent_panel_reply(ZedAgentPanelRequest {
            prompt: "Refactor this module into smaller functions.".to_string(),
            profile: ZedAgentProfile::Write,
            working_directory: Some("F:/my-project".to_string()),
            language: Some("Rust".to_string()),
            buffer_path: Some("src/lib.rs".to_string()),
            selected_text: None,
            context_items: vec![],
            tool_permission_mode: ZedToolPermissionMode::Confirm,
        })
        .await?;

    println!("{}", response.text);
    Ok(())
}
```

## Codex-Focused Integration

For a Codex fork or Codex-style host surface, use `CodexFlowAdapter` instead of inventing
your own prompt and approval-mode mapping layer:

```rust
use anyhow::Result;
use flow::{
    CodexApprovalMode, CodexExecutionTarget, CodexFlowAdapter, CodexReasoningEffort,
    CodexSurface, CodexTaskKind, CodexTaskRequest,
};

#[tokio::main]
async fn main() -> Result<()> {
    let codex = CodexFlowAdapter::detect()?;

    let response = codex
        .run_task(CodexTaskRequest {
            prompt: "Refactor the parser into smaller helpers without changing behavior."
                .to_string(),
            kind: CodexTaskKind::Refactor,
            surface: CodexSurface::IdePanel,
            approval_mode: CodexApprovalMode::AutoEdit,
            reasoning_effort: CodexReasoningEffort::Medium,
            execution_target: CodexExecutionTarget::LocalWorkspace,
            working_directory: Some("F:/my-project".to_string()),
            repository_root: Some("F:/my-project".to_string()),
            active_file: Some("src/parser.rs".to_string()),
            selected_text: None,
            context_items: vec![],
            attachments: vec![],
            terminal_summary: None,
            browser_summary: None,
            requested_candidates: 2,
        })
        .await?;

    println!("{}", response.primary.text);
    Ok(())
}
```

## ZeroClaw-Focused Integration

For a ZeroClaw fork or ZeroClaw-style host surface, use `ZeroClawFlowAdapter` so autonomy,
channel, memory, and gateway metadata are carried through natively:

```rust
use anyhow::Result;
use flow::{
    ZeroClawAutonomyLevel, ZeroClawChannel, ZeroClawExecutionTarget, ZeroClawFlowAdapter,
    ZeroClawSurface, ZeroClawTaskRequest, ZeroClawToolClass, ZeroClawToolPolicy,
};

#[tokio::main]
async fn main() -> Result<()> {
    let zeroclaw = ZeroClawFlowAdapter::detect()?;

    let response = zeroclaw
        .run_task(ZeroClawTaskRequest {
            prompt: "Plan the safest local fix for the broken build and tell the gateway what to do next."
                .to_string(),
            autonomy_level: ZeroClawAutonomyLevel::Supervised,
            surface: ZeroClawSurface::GatewayDashboard,
            execution_target: ZeroClawExecutionTarget::GatewaySession,
            channel: Some(ZeroClawChannel::Dashboard),
            working_directory: Some("F:/my-project".to_string()),
            session_id: Some("gateway-session-1".to_string()),
            active_file: Some("src/main.rs".to_string()),
            selected_text: None,
            context_items: vec![],
            tool_policies: vec![ZeroClawToolPolicy {
                class: ZeroClawToolClass::Shell,
                enabled: true,
                note: Some("Require explicit approval for commands".to_string()),
            }],
            memory_summary: Some("User prefers short operational summaries.".to_string()),
            identity_summary: None,
            user_profile_summary: None,
            terminal_summary: None,
            browser_summary: None,
            requested_candidates: 2,
        })
        .await?;

    println!("{}", response.primary.text);
    Ok(())
}
```

## Main Types

### `flow::FlowLocalRuntime`

- `detect()` builds a device-aware local runtime from the current machine
- `for_device_profile(profile)` builds a runtime for a supplied host profile
- `warm_text_model()` initializes the selected local chat model
- `generate_text(prompt)` runs local text generation
- `generate_text_with_metrics(prompt)` returns text plus generation metrics
- `transcribe_file(path)` runs local STT through Moonshine
- `transcribe_samples(samples)` runs local STT over 16 kHz mono samples
- `clean_transcription(raw)` runs local transcript cleanup through the local chat model
- `transcribe_and_clean_file(path)` runs STT and cleanup together
- `synthesize_text(text)` runs local TTS through Kokoro
- `synthesize_text_to_file(text, path)` runs TTS and writes a WAV file
- `transcribe_clean_and_synthesize_to_file(input, output)` runs the end-to-end local speech pipeline

### `flow::ZedFlowAdapter`

- `detect()` builds a Zed-oriented adapter on top of the local runtime
- `local_model_status()` reports chat/STT/TTS readiness for editor integration
- `warm_for_zed()` preloads the local chat model for a lower-latency first response
- `agent_panel_reply(request)` maps to a Zed Agent Panel-style request/response flow
- `inline_assist(request)` maps to a Zed Inline Assistant-style selection rewrite
- `edit_prediction(request)` maps to a Zed edit-prediction-style insertion request
- `transcribe_voice_note(path)` maps local STT + cleanup into editor-ready inserted text

### `flow::CodexFlowAdapter`

- `detect()` builds a Codex-oriented adapter on top of the local runtime
- `local_model_status()` reports local readiness for CLI, IDE, review, browser-context, and follow-up work
- `warm_for_codex()` preloads the local text model for a lower-latency first task
- `run_task(request)` maps to a Codex-style local coding task with approval-mode and reasoning-effort hints
- `follow_up(request)` carries previous answer state plus fresh diff/terminal updates into a follow-up pass
- `review_pull_request(request)` maps local text generation into a Codex-style PR review with summary, findings, and suggested tests

### `flow::ZeroClawFlowAdapter`

- `detect()` builds a ZeroClaw-oriented adapter on top of the local runtime
- `local_model_status()` reports local readiness for agent CLI, gateway, daemon, channel, memory, and skill-runner work
- `warm_for_zeroclaw()` preloads the local text model for a lower-latency first turn
- `run_task(request)` maps to a ZeroClaw-style local task with autonomy, channel, session, memory, and tool-policy hints
- `follow_up(request)` carries prior answer state plus fresh memory/browser/terminal updates into the next turn

### `flow::FlowLocalRuntimeSummary`

- `device_profile`
- `chat`
- `speech_to_text`
- `text_to_speech`

Each modality selection reports:

- selected model key
- local artifact path
- selected runtime kind
- local readiness
- broker reasoning

### `flow::FlowProductionConfig`

- `recommended_for_target(target, summary)` builds a host-targeted production config from the selected local runtime summary
- `to_pretty_json()` serializes the production config for handoff, embedding, or external config files

### `flow::FlowProductionBundleManifest`

- captures the machine-specific production handoff state for an exported config bundle
- records selected local models, local readiness, missing model paths, validated commands, and browser release artifacts
- is produced by `DxFlowRuntime::production_bundle_manifest()` and `DxFlowRuntime::export_production_bundle(...)`

### `flow::FlowReleaseSummary`

- captures the repo-level release handoff state for the current machine and workspace
- records production bundle file readiness, browser artifact readiness, validated commands, and external release tasks
- is produced by `DxFlowRuntime::release_summary()` and `DxFlowRuntime::export_release_summary(...)`

Supported targets:

- `FlowIntegrationTarget::DxDesktop`
- `FlowIntegrationTarget::BrowserExtension`
- `FlowIntegrationTarget::ZedFork`
- `FlowIntegrationTarget::CodexFork`
- `FlowIntegrationTarget::ZeroClawFork`

## Model Defaults

Current local defaults are device-aware:

- low-end devices prefer `qwen3-0.6b` for local text generation
- STT defaults to `moonshine-tiny`
- TTS defaults to `kokoro-int8`

## Lower-Level Model APIs

Flow also exposes the lower-level engines directly:

- `MoonshineSTT`
- `LocalLlm`
- `KokoroTTS`

Use those directly only when a host wants to manage all orchestration manually. For most editor or app integrations, prefer `FlowLocalRuntime`.
