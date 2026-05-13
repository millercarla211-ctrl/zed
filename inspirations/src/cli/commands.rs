use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hf_hub::api::sync::Api;
use serde_json::Value;

use crate::audio::{NoiseGateVAD, WakeWordDetector};
use crate::browser::{
    BrowserHostFlavor, BrowserTask, FlowBrowserEngine, default_browser_pack_catalog,
};
use crate::cli::Command;
use crate::competitive::default_competitive_scorecard;
use crate::config::FlowIntegrationTarget;
use crate::embed::{FlowEmbeddingRegistry, HostSurface};
use crate::experience::{FlowAutomationBridge, NativeSelectionBridge, OperatingSystemFamily};
use crate::models::{GlmOcr, KokoroTTS, LocalLlm, LocalSttEngine};
use crate::runtime::{
    BrokerRequest, ExecutionPlan, Modality, RuntimeBroker, wake_command_definitions,
};
use crate::workspace::dx_project_statuses;
use crate::writing::HarperGrammarChecker;

const DEFAULT_UI_MODEL_KEY: &str = "qwendean-4b-q4km";
const QWEN3_06B_MODEL_KEY: &str = "qwen3-0.6b";
const QWEN3_06B_MODEL_REPO: &str = "jc-builds/Qwen3-0.6B-Q4_K_M-GGUF";
const QWEN3_06B_MODEL_FILE: &str = "Qwen3-0.6B-Q4_K_M.gguf";
const QWEN3_06B_MODEL_PATH: &str = "models/llm/Qwen3-0.6B-Q4_K_M.gguf";
const WEBGEN_MODEL_KEY: &str = "webgen-4b-preview-i1-q4km";
const WEBGEN_MODEL_REPO: &str = "mradermacher/WEBGEN-4B-Preview-i1-GGUF";
const WEBGEN_MODEL_FILE: &str = "WEBGEN-4B-Preview.i1-Q4_K_M.gguf";
const WEBGEN_MODEL_PATH: &str = "models/llm/WEBGEN-4B-Preview.i1-Q4_K_M.gguf";
const QWENDEAN_MODEL_KEY: &str = "qwendean-4b-q4km";
const QWENDEAN_MODEL_REPO: &str = "iamdyeus/qwendean-4b-GGUF";
const QWENDEAN_MODEL_FILE: &str = "Qwendean-4B.Q4_K_M.gguf";
const QWENDEAN_MODEL_PATH: &str = "models/llm/Qwendean-4B.Q4_K_M.gguf";
const QWEN35_9B_MODEL_KEY: &str = "qwen35-9b-q4km";
const QWEN35_9B_MODEL_REPO: &str = "jc-builds/Qwen3.5-9B-Q4_K_M-GGUF";
const QWEN35_9B_MODEL_FILE: &str = "Qwen3.5-9B-Q4_K_M.gguf";
const QWEN35_9B_MODEL_PATH: &str = "models/llm/Qwen3.5-9B-Q4_K_M.gguf";
const QWEN35_4B_REVISED_MODEL_KEY: &str = "qwen35-4b-revised-q4km";
const QWEN35_4B_REVISED_MODEL_REPO: &str = "Smoffyy/Qwen3.5-4B-Instruct-Revised-GGUF";
const QWEN35_4B_REVISED_MODEL_FILE: &str = "Qwen3.5-4B-q4_k_m.gguf";
const QWEN35_4B_REVISED_MODEL_PATH: &str = "models/llm/Qwen3.5-4B-q4_k_m.gguf";
const XLAM2_3B_TOOL_MODEL_KEY: &str = "xlam2-3b-fc-r-q4km";
const XLAM2_3B_TOOL_MODEL_REPO: &str = "Salesforce/xLAM-2-3b-fc-r-gguf";
const XLAM2_3B_TOOL_MODEL_FILE: &str = "xLAM-2-3B-fc-r-Q4_K_M.gguf";
const XLAM2_3B_TOOL_MODEL_PATH: &str = "models/llm/xLAM-2-3B-fc-r-Q4_K_M.gguf";
const MINISTRAL3_3B_MODEL_KEY: &str = "ministral3-3b-instruct-q4km";
const MINISTRAL3_3B_MODEL_REPO: &str = "unsloth/Ministral-3-3B-Instruct-2512-GGUF";
const MINISTRAL3_3B_MODEL_FILE: &str = "Ministral-3-3B-Instruct-2512-Q4_K_M.gguf";
const MINISTRAL3_3B_MODEL_PATH: &str = "models/llm/Ministral-3-3B-Instruct-2512-Q4_K_M.gguf";
const GRANITE4_H_MICRO_MODEL_KEY: &str = "granite4-h-micro-q4km";
const GRANITE4_H_MICRO_MODEL_REPO: &str = "ibm-granite/granite-4.0-h-micro-GGUF";
const GRANITE4_H_MICRO_MODEL_FILE: &str = "granite-4.0-h-micro-Q4_K_M.gguf";
const GRANITE4_H_MICRO_MODEL_PATH: &str = "models/llm/granite-4.0-h-micro-Q4_K_M.gguf";
const PHI4_MINI_MODEL_KEY: &str = "phi4-mini-instruct-q4km";
const PHI4_MINI_MODEL_REPO: &str = "DuoNeural/Phi-4-mini-instruct-GGUF";
const PHI4_MINI_MODEL_FILE: &str = "Phi-4-mini-instruct-Q4_K_M.gguf";
const PHI4_MINI_MODEL_PATH: &str = "models/llm/Phi-4-mini-instruct-Q4_K_M.gguf";
const SMOLLM3_3B_MODEL_KEY: &str = "smollm3-3b-q4km";
const SMOLLM3_3B_MODEL_REPO: &str = "ggml-org/SmolLM3-3B-GGUF";
const SMOLLM3_3B_MODEL_FILE: &str = "SmolLM3-Q4_K_M.gguf";
const SMOLLM3_3B_MODEL_PATH: &str = "models/llm/SmolLM3-Q4_K_M.gguf";
const GEMMA4_FRONTEND_MODEL_KEY: &str = "gemma4-e4b-frontend-q4km";
const GEMMA4_FRONTEND_MODEL_REPO: &str = "DuoNeural/Gemma-4-E4B-Frontend-GGUF";
const GEMMA4_FRONTEND_MODEL_FILE: &str = "gemma-4-E4B-it.Q4_K_M.gguf";
const GEMMA4_FRONTEND_MODEL_PATH: &str = "models/llm/gemma-4-E4B-it.Q4_K_M.gguf";
const GEMMA4_FRONTEND_MMPROJ_FILE: &str = "gemma-4-E4B-it.BF16-mmproj.gguf";
const GEMMA4_FRONTEND_MMPROJ_PATH: &str = "models/llm/gemma-4-E4B-it.BF16-mmproj.gguf";
const UIGEN_FX_MODEL_KEY: &str = "uigen-fx-4b-preview-q4km";
const UIGEN_FX_MODEL_REPO: &str = "QuantFactory/UIGEN-FX-4B-Preview-GGUF";
const UIGEN_FX_MODEL_FILE: &str = "UIGEN-FX-4B-Preview.Q4_K_M.gguf";
const UIGEN_FX_MODEL_PATH: &str = "models/llm/UIGEN-FX-4B-Preview.Q4_K_M.gguf";
const UIMODEL_GOOGLE_OUTPUT_ROOT: &str = "tmp/uigen-google";
const UIVISION_GOOGLE_OUTPUT_ROOT: &str = "tmp/uigen-vision-google";
const PARAKEET_STT_KEY: &str = "parakeet-tdt-0.6b-v3-int8";
const PARAKEET_STT_PATH: &str = "models/stt/parakeet-tdt-0.6b-v3-int8/encoder.int8.onnx";
const GOOGLE_URL: &str = "https://www.google.com/?hl=en&gl=US&pws=0";
const GOOGLE_HOMEPAGE_PROMPT: &str = "\
Make a responsive Google.com homepage look-alike for screenshot evaluation only.
Do not use Google image assets, fonts, scripts, CDNs, or external assets.
Use a plain text multicolor wordmark spelling Google centered in the first viewport, a 582px rounded search box with a subtle border and shadow, two light gray buttons labeled Google Search and I'm Feeling Lucky, top-right text navigation with Gmail, Images, an apps icon placeholder, and an avatar circle, and a two-row bottom footer with realistic links.
Desktop layout: large whitespace, centered search composition around the vertical middle, footer pinned to bottom, top nav at the top-right.
Mobile layout: same centered search composition, narrower search field, footer links wrapping cleanly.
Keep it simple, accurate, complete, and screenshot-stable.
";
const GOOGLE_VISION_PROMPT: &str = "\
Use the screenshot as the source of truth and recreate the visible search homepage as a complete standalone HTML/CSS file for evaluation only.
Return only HTML code.
Do not use external scripts, external fonts, CDNs, images, or brand assets.
Use semantic HTML and inline CSS in one <style> tag.
Keep CSS compact and do not repeat selectors.
Keep the whole file under 180 lines.
Match the visible layout: top navigation, centered logo/search composition, search controls, bottom footer, spacing, and mobile responsiveness.
Include </body> and </html>.
";

/// Execute CLI command
pub async fn execute(command: Command) -> Result<()> {
    match command {
        Command::Transcribe { file } => {
            let broker = RuntimeBroker::detect();
            let mut stt = LocalSttEngine::from_broker(&broker)?;
            let text = stt.transcribe(&file)?;
            println!("[stt] {}", text);
        }

        Command::Wispr { file } => {
            println!("Flow Wispr Mode");
            println!("================");
            println!("[input] {}", file);

            let broker = RuntimeBroker::detect();
            let mut stt = LocalSttEngine::from_broker(&broker)?;
            let raw_text = stt.transcribe(&file)?;
            println!("[stt/raw] {}", raw_text);

            let llm = load_chat_llm(&broker).await?;
            let cleaned = llm.clean_speech(&raw_text).await?;

            println!("\n[cleaned]");
            println!("{}", cleaned);
        }

        Command::Speak { text } => {
            let mut tts = KokoroTTS::new_async().await?;
            tts.speak(&text)?;
            println!("[tts] saved to output.wav");
        }

        Command::Live => {
            run_live_mode().await?;
        }

        Command::Dictate => {
            run_dictation_mode().await?;
        }

        Command::Interactive => {
            print_interactive_help();
        }

        Command::Chat { model } => {
            crate::cli::chat::run_chat(model).await?;
        }

        Command::ToolAgent { tools, request } => {
            run_tool_agent(tools.as_deref(), &request).await?;
        }

        Command::Ocr { image, prompt } => {
            println!("Flow OCR");
            println!("========");

            let ocr = GlmOcr::new()?;
            let result = if let Some(custom_prompt) = prompt {
                ocr.ocr_with_prompt(&image, &custom_prompt)?
            } else {
                ocr.ocr_image(&image)?
            };

            println!("{}", result);
        }

        Command::Profile => {
            print_profile(&RuntimeBroker::detect());
        }

        Command::Projects => {
            print_projects();
        }

        Command::Scorecard => {
            print_scorecard();
        }

        Command::Models { modality } => {
            print_models(&RuntimeBroker::detect(), modality.as_deref())?;
        }

        Command::InstallModel { model } => {
            install_model_cli(&model)?;
        }

        Command::UiModelCandidates => {
            print_ui_model_candidates()?;
        }

        Command::ToolModelCandidates => {
            print_tool_model_candidates()?;
        }

        Command::ModelRoles => {
            print_model_roles();
        }

        Command::Uigen {
            model,
            output,
            prompt,
        } => {
            run_uigen(model.as_deref(), &output, &prompt).await?;
        }

        Command::UigenGoogle { model } => {
            run_uigen_google(model.as_deref()).await?;
        }

        Command::UigenVision {
            screenshot,
            output,
            prompt,
        } => {
            run_uigen_vision(&screenshot, &output, &prompt)?;
        }

        Command::UigenVisionGoogle => {
            run_uigen_vision_google()?;
        }

        Command::Plan { modality, model } => {
            let modality = parse_modality(&modality)?;
            let broker = RuntimeBroker::detect();
            let plan = broker.build_plan(BrokerRequest::new(modality).with_model(model));
            print_plan(&broker, &plan);
        }

        Command::Blueprint { host } => {
            let host = parse_host_surface(&host)?;
            print_blueprint(host);
        }

        Command::BrowserProfile { flavor } => {
            let flavor = parse_browser_flavor(&flavor)?;
            print_browser_profile(flavor);
        }

        Command::BrowserPlan {
            flavor,
            task,
            modality,
            model,
            remote_fallback,
        } => {
            let flavor = parse_browser_flavor(&flavor)?;
            let task = parse_browser_task(&task)?;
            let modality = parse_modality(&modality)?;
            print_browser_plan(flavor, task, modality, model, remote_fallback);
        }

        Command::BrowserPacks => {
            print_browser_packs();
        }

        Command::Grammar { text, fix } => {
            run_grammar(&text, fix)?;
        }

        Command::WakeWords => {
            print_wake_words(&RuntimeBroker::detect());
        }

        Command::ProductionConfig { target } => {
            let target = parse_integration_target(&target)?;
            print_production_config(target)?;
        }

        Command::ExportProductionBundle { output_dir } => {
            export_production_bundle_cli(&output_dir)?;
        }

        Command::ReleaseSummary => {
            print_release_summary()?;
        }

        Command::ExportReleaseSummary { output_dir } => {
            export_release_summary_cli(&output_dir)?;
        }
    }

    Ok(())
}

fn print_interactive_help() {
    println!("Flow");
    println!("====\n");
    println!("Commands:");
    println!("  --transcribe <file>      Transcribe an audio file");
    println!("  --wispr <file>           STT plus local cleanup");
    println!("  --speak <text>           Speak text with Kokoro");
    println!("  --live                   Live microphone mode");
    println!("  --dictate                Wake/hotkey dictation into focused input");
    println!("  --chat [model]           Interactive local chat");
    println!("  --tool-agent <prompt>    Run one bounded local tool-agent prompt");
    println!("  --tool-agent-tools <tools.json> <request>");
    println!("                           Run tool routing with a tools JSON file");
    println!("  --ocr <image> [prompt]   OCR with GLM-OCR");
    println!("  --profile                Show device profile and activation config");
    println!("  --projects               Show the DX project stack");
    println!("  --scorecard              Show the Flow competitive scorecard");
    println!("  --models [modality]      Show broker model catalog");
    println!("  --install-model <key>    Download a known local model artifact");
    println!("  --ui-model-candidates    Show ranked local UI model options");
    println!("  --tool-model-candidates  Show ranked local tool-calling model options");
    println!("  --model-roles            Show Flow's local model routing policy");
    println!("  --uigen <out.html> <prompt>");
    println!("                           Generate a single-file UI with the default UI model");
    println!("  --uigen-model <key> <out.html> <prompt>");
    println!("                           Generate with a selected UI model");
    println!("  --uigen-google [key]     Generate the Google homepage clone eval file");
    println!("  --uigen-vision <screenshot.png> <out.html> <prompt>");
    println!("                           Generate a UI from a screenshot with Gemma frontend");
    println!("  --uigen-vision-google    Capture Google and generate a vision UI clone");
    println!("  --plan <modality> [key]  Show runtime broker execution plan");
    println!("  --blueprint [host]       Show the embedding blueprint for a host");
    println!("  --browser-profile [flavor]");
    println!("                           Show default browser capability profile");
    println!("  --browser-plan <flavor> <task> <modality> [model] [--remote]");
    println!("                           Show browser execution plan");
    println!("  --browser-packs          Show registered browser-ready model packs");
    println!("  --grammar [--fix] <text> Analyze or correct text");
    println!("  --wakewords              Show local wake-word models");
    println!("  --production-config [target]");
    println!("                           Print the recommended production config JSON");
    println!("  --export-production-bundle [dir]");
    println!("                           Export all production configs and a manifest");
    println!("  --release-summary        Print the current release summary JSON");
    println!("  --export-release-summary [dir]");
    println!("                           Export release summary and handoff markdown");
    println!();
    println!("Examples:");
    println!("  cargo run --bin flow -- --profile");
    println!("  cargo run --bin flow -- --projects");
    println!("  cargo run --bin flow -- --scorecard");
    println!("  cargo run --bin flow -- --models chat");
    println!("  cargo run --release --bin flow -- --install-model qwen3-0.6b");
    println!("  cargo run --release --bin flow -- --install-model qwen35-4b-revised-q4km");
    println!("  cargo run --release --bin flow -- --install-model xlam2-3b-fc-r-q4km");
    println!("  cargo run --release --bin flow -- --install-model qwen35-9b-q4km");
    println!("  cargo run --release --bin flow -- --chat qwen35-4b-revised-q4km");
    println!("  cargo run --release --bin flow -- --tool-agent \"choose a tool for this request\"");
    println!(
        "  cargo run --release --bin flow -- --tool-agent-tools examples/tool-agent/weather-tools.json \"weather in Dhaka tomorrow\""
    );
    println!("  cargo run --release --bin flow -- --install-model webgen-4b-preview-i1-q4km");
    println!("  cargo run --release --bin flow -- --install-model qwendean-4b-q4km");
    println!("  cargo run --release --bin flow -- --install-model gemma4-e4b-frontend-q4km");
    println!("  cargo run --release --bin flow -- --ui-model-candidates");
    println!("  cargo run --release --bin flow -- --tool-model-candidates");
    println!("  cargo run --release --bin flow -- --model-roles");
    println!("  cargo run --release --bin flow -- --models ui");
    println!("  cargo run --release --bin flow -- --models vlm");
    println!(
        "  cargo run --release --bin flow -- --uigen tmp/uigen-output/index.html \"make a shadcn-style Google homepage clone\""
    );
    println!("  cargo run --release --bin flow -- --uigen-google webgen-4b-preview-i1-q4km");
    println!("  cargo run --release --bin flow -- --uigen-google qwendean-4b-q4km");
    println!("  cargo run --release --bin flow -- --uigen-vision-google");
    println!("  cargo run --bin flow -- --plan chat qwen3-0.6b");
    println!("  cargo run --bin flow -- --blueprint dx");
    println!("  cargo run --bin flow -- --browser-profile chromium");
    println!("  cargo run --bin flow -- --browser-plan chromium rewrite-selection chat");
    println!("  cargo run --bin flow -- --browser-packs");
    println!("  cargo run --bin flow -- --grammar --fix \"This is an test.\"");
    println!("  cargo run --bin flow -- --production-config codex-fork");
    println!("  cargo run --bin flow -- --export-production-bundle configs/production");
    println!("  cargo run --bin flow -- --release-summary");
    println!("  cargo run --bin flow -- --export-release-summary release");
}

fn print_profile(broker: &RuntimeBroker) {
    let profile = broker.device_profile();
    println!("Flow Device Profile");
    println!("===================");
    println!("OS: {} / {}", profile.os, profile.arch);
    println!("CPU: {}", profile.cpu_model);
    println!(
        "Cores: {} physical / {} logical",
        profile.physical_cores, profile.logical_cores
    );
    println!(
        "Memory: {:.1} GB total / {:.1} GB available",
        profile.total_memory_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
        profile.available_memory_bytes as f64 / 1024.0 / 1024.0 / 1024.0
    );
    println!("Tier: {:?}", profile.tier);
    println!();

    if profile.graphics.is_empty() {
        println!("Graphics: none detected");
    } else {
        println!("Graphics:");
        for gpu in &profile.graphics {
            let vram = gpu
                .vram_bytes
                .map(format_bytes)
                .unwrap_or_else(|| "unknown".to_string());
            println!(
                "  - {} ({}, {}, backends: {})",
                gpu.name,
                gpu.vendor.as_deref().unwrap_or("unknown"),
                vram,
                gpu.backends
                    .iter()
                    .map(|backend| format!("{backend:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    println!();
    println!(
        "Push-to-talk: {}",
        shortcut_label(
            &broker.activation().push_to_talk.modifiers,
            &broker.activation().push_to_talk.key
        )
    );
    println!(
        "Hands-free toggle: {}",
        shortcut_label(
            &broker.activation().hands_free_toggle.modifiers,
            &broker.activation().hands_free_toggle.key
        )
    );

    if broker.activation().wake_words.is_empty() {
        println!("Wake words: none detected in models/wake_words");
    } else {
        println!("Wake words:");
        for item in &broker.activation().wake_words {
            let aliases = if item.aliases.is_empty() {
                "-".to_string()
            } else {
                item.aliases.join(", ")
            };
            println!(
                "  - {} ({}) (aliases: {}, threshold: {}%, model: {})",
                item.command_key, item.phrase, aliases, item.threshold, item.model_path
            );
        }
    }
}

fn print_projects() {
    println!("DX Project Stack");
    println!("================");
    for project in dx_project_statuses() {
        println!(
            "- {}: {} ({}%)",
            project.key, project.role, project.completeness_score
        );
    }
}

fn print_scorecard() {
    let scorecard = default_competitive_scorecard();

    println!("Flow Competitive Scorecard");
    println!("==========================");
    println!("Measured on: {}", scorecard.measured_on);
    println!("Overall: {} / 100", scorecard.overall_score_out_of_100);
    println!(
        "Wispr replacement: {} / 100",
        scorecard.wispr_replacement_score_out_of_100
    );
    println!(
        "Grammarly replacement: {} / 100",
        scorecard.grammarly_replacement_score_out_of_100
    );
    println!(
        "Flow-native advantage: {} / 100",
        scorecard.flow_native_advantage_score_out_of_100
    );
    println!();
    println!("Top gaps:");
    for gap in scorecard.top_gaps {
        println!("  - {}", gap);
    }
}

fn print_models(broker: &RuntimeBroker, modality_filter: Option<&str>) -> Result<()> {
    let filter = modality_filter.map(parse_modality).transpose()?;
    println!("Flow Model Catalog");
    println!("==================");

    for manifest in broker.catalog() {
        if filter.is_some() && filter != Some(manifest.modality) {
            continue;
        }

        let local = if matches!(manifest.modality, Modality::SpeechToText) {
            LocalSttEngine::model_files_ready(&manifest.key, manifest.local_path.as_deref())
        } else {
            model_artifact_ready(manifest)
        };
        println!("{} - {}", manifest.key, manifest.display_name);
        println!(
            "  modality={:?}, runtime={:?}, format={:?}, min_mem={}",
            manifest.modality,
            manifest.preferred_runtime,
            manifest.artifact_format,
            format_bytes(manifest.minimum_memory_bytes)
        );
        println!(
            "  repo={}, local={}, conversion={}",
            manifest.repo_id,
            if local { "present" } else { "missing" },
            manifest
                .conversion_lanes
                .iter()
                .map(|lane| format!("{lane:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        if let Some(path) = &manifest.local_path {
            println!("  path={}", path);
        }
        if !manifest.tags.is_empty() {
            println!("  tags={}", manifest.tags.join(", "));
        }
        println!();
    }

    Ok(())
}

fn print_ui_model_candidates() -> Result<()> {
    println!("Flow UI Model Candidates");
    println!("========================");
    println!(
        "Device note: CPU-first is recommended on this machine class; tiny iGPU VRAM is not enough for meaningful offload."
    );
    println!();

    let candidates = [
        (
            "1",
            GEMMA4_FRONTEND_MODEL_KEY,
            "DuoNeural/Gemma-4-E4B-Frontend-GGUF",
            "image-text-to-text GGUF + mmproj",
            "apache-2.0",
            "Yes",
            "Best local visual candidate, but initial Google clone eval failed complete HTML; not proven.",
        ),
        (
            "2",
            "zai-org-ui2code-n",
            "zai-org/UI2Code_N",
            "image-text-to-text safetensors",
            "mit",
            "Yes",
            "Best quality candidate, but not GGUF-simple and likely too heavy locally.",
        ),
        (
            "3",
            "allenai-molmoweb-4b",
            "allenai/MolmoWeb-4B",
            "image-text-to-text safetensors/custom code",
            "apache-2.0",
            "Yes",
            "Useful as a visual web evaluator/agent, not the direct HTML generator.",
        ),
        (
            "4",
            "uigen-t3-8b-preview-q4km",
            "QuantFactory/UIGEN-T3-8B-Preview-GGUF",
            "text-only GGUF",
            "check-before-production",
            "No",
            "Text-only fallback only; do not use first for screenshot cloning.",
        ),
        (
            "5",
            QWENDEAN_MODEL_KEY,
            QWENDEAN_MODEL_REPO,
            "text-only GGUF",
            "apache-2.0",
            "No",
            "Already tested: complete HTML but poor visual clone.",
        ),
        (
            "6",
            WEBGEN_MODEL_KEY,
            WEBGEN_MODEL_REPO,
            "text-only GGUF",
            "apache-2.0",
            "No",
            "Already tested: incomplete standalone HTML in this runtime.",
        ),
    ];

    for (rank, key, repo, runtime, license, screenshot, recommendation) in candidates {
        let local = download_spec_for_model(key)
            .map(|spec| model_download_spec_ready(&spec))
            .unwrap_or(false);
        println!("{rank}. {key}");
        println!("   repo={repo}");
        println!("   runtime={runtime}");
        println!("   license={license}");
        println!("   screenshot_support={screenshot}");
        println!("   local={}", if local { "present" } else { "missing" });
        println!("   recommendation={recommendation}");
        println!();
    }

    Ok(())
}

fn print_tool_model_candidates() -> Result<()> {
    println!("Flow Tool-Calling Model Candidates");
    println!("===================================");
    println!("Ranking is for local CPU-first agent routing, JSON/tool calls, and reasoning.");
    println!("Commercial-safe means Apache/MIT-style licensing for future product use.");
    println!();

    let candidates = [
        (
            "1",
            XLAM2_3B_TOOL_MODEL_KEY,
            XLAM2_3B_TOOL_MODEL_REPO,
            "specialist function-calling GGUF",
            "cc-by-nc-4.0",
            "No",
            "Best small pure tool-calling candidate; install for local research/eval, not commercial default.",
        ),
        (
            "2",
            MINISTRAL3_3B_MODEL_KEY,
            MINISTRAL3_3B_MODEL_REPO,
            "general instruct + native tool/JSON GGUF",
            "apache-2.0",
            "Yes",
            "Best commercial-safe small general agent/chat replacement candidate.",
        ),
        (
            "3",
            GRANITE4_H_MICRO_MODEL_KEY,
            GRANITE4_H_MICRO_MODEL_REPO,
            "low-latency structured-output GGUF",
            "apache-2.0",
            "Yes",
            "Best tiny commercial-safe router candidate for strict JSON/function-call workflows.",
        ),
        (
            "4",
            PHI4_MINI_MODEL_KEY,
            PHI4_MINI_MODEL_REPO,
            "reasoning-focused instruct GGUF",
            "mit",
            "Yes",
            "Strong small reasoning backup with documented function-calling format.",
        ),
        (
            "5",
            "qwen35-4b-revised-q4km",
            QWEN35_4B_REVISED_MODEL_REPO,
            "general smart/coding GGUF",
            "apache-2.0",
            "Yes",
            "Already installed and smart, but not the cleanest dedicated tool-call model.",
        ),
        (
            "6",
            SMOLLM3_3B_MODEL_KEY,
            SMOLLM3_3B_MODEL_REPO,
            "fast general small GGUF",
            "apache-2.0",
            "Yes",
            "Good small fallback; weaker tool specialization than xLAM, Ministral, or Granite.",
        ),
    ];

    for (rank, key, repo, runtime, license, commercial_safe, recommendation) in candidates {
        let local = download_spec_for_model(key)
            .map(|spec| model_download_spec_ready(&spec))
            .unwrap_or_else(|| {
                LocalLlm::model_path_for_key(key)
                    .map(|path| Path::new(&path).exists())
                    .unwrap_or(false)
            });
        println!("{rank}. {key}");
        println!("   repo={repo}");
        println!("   runtime={runtime}");
        println!("   license={license}");
        println!("   commercial_safe={commercial_safe}");
        println!("   local={}", if local { "present" } else { "missing" });
        println!("   recommendation={recommendation}");
        println!();
    }

    println!("Install the top research model:");
    println!("  cargo run --release --bin flow -- --install-model {XLAM2_3B_TOOL_MODEL_KEY}");
    println!("Commercial-safe runner-up:");
    println!("  cargo run --release --bin flow -- --install-model {MINISTRAL3_3B_MODEL_KEY}");

    Ok(())
}

async fn run_tool_agent(tools_path: Option<&str>, request: &str) -> Result<()> {
    let spec = download_spec_for_model(XLAM2_3B_TOOL_MODEL_KEY)
        .context("No built-in tool-agent model is registered")?;
    if !model_download_spec_ready(&spec) {
        return Err(anyhow::anyhow!(
            "Tool-agent model '{}' is missing. Run: cargo run --release --bin flow -- --install-model {}",
            XLAM2_3B_TOOL_MODEL_KEY,
            XLAM2_3B_TOOL_MODEL_KEY
        ));
    }

    let tools_json = match tools_path {
        Some(path) => read_tool_agent_tools(path)?,
        None => "[]".to_string(),
    };

    let llm = LocalLlm::for_tool_agent();
    let started = Instant::now();
    llm.initialize().await?;
    let load_time_ms = started.elapsed().as_millis();
    let (response, metrics) = llm
        .generate_tool_call_with_metrics(&tools_json, request)
        .await?;

    let json_state = if response.trim_start().starts_with('[') {
        match serde_json::from_str::<Value>(&response) {
            Ok(_) => "valid",
            Err(_) => "invalid",
        }
    } else {
        "not-json"
    };

    println!("{}", response);
    println!();
    println!(
        "[tool-agent] json={} load={:.2}s prompt_tokens={} generated_tokens={} total={:.2}s gen={:.2}s speed={:.2} tok/s",
        json_state,
        load_time_ms as f64 / 1000.0,
        metrics.prompt_tokens,
        metrics.generated_tokens,
        metrics.total_time_ms as f64 / 1000.0,
        metrics.generation_time_ms as f64 / 1000.0,
        metrics.tokens_per_second
    );

    Ok(())
}

fn read_tool_agent_tools(path: &str) -> Result<String> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read tool schema file: {path}"))?;
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("Invalid JSON in tool schema: {path}"))?;
    if !value.is_array() {
        return Err(anyhow::anyhow!(
            "Tool schema must be a JSON array of tool definitions: {path}"
        ));
    }
    serde_json::to_string(&value).context("Failed to compact tool schema JSON")
}

fn print_model_roles() {
    println!("Flow Local Model Roles");
    println!("======================");
    println!("This machine policy keeps local GGUF models active by purpose:");
    println!();

    for role in LocalLlm::model_roles() {
        let path = Path::new(role.model_path);
        let state = if path.exists() { "present" } else { "missing" };
        println!("{} - {}", role.role, role.model_key);
        println!("  path={}", role.model_path);
        println!("  local={state}");
        println!("  purpose={}", role.purpose);
        println!();
    }

    println!("Commands:");
    println!("  cargo run --release --bin flow -- --chat qwen3-0.6b");
    println!("  cargo run --release --bin flow -- --tool-agent \"choose a tool for this request\"");
    println!("  cargo run --release --bin flow -- --chat qwen35-4b-revised-q4km");
    println!("  cargo run --release --bin flow -- --chat qwen35-9b-q4km");
}

fn install_model_cli(model_key: &str) -> Result<()> {
    let spec = download_spec_for_model(model_key)
        .with_context(|| format!("No built-in installer is registered for model '{model_key}'"))?;

    if model_download_spec_ready(&spec) {
        println!(
            "Model '{}' is already installed ({} file(s)).",
            spec.model_key,
            spec.files.len()
        );
        return Ok(());
    }

    for file in spec.files {
        install_model_file(&spec, file)?;
    }

    println!(
        "Installed '{}' ({} file(s)).",
        spec.model_key,
        spec.files.len()
    );
    Ok(())
}

fn install_model_file(spec: &ModelDownloadSpec, file: &ModelDownloadFile) -> Result<()> {
    let local_path = PathBuf::from(file.local_path);

    if local_path.exists() && fs::metadata(&local_path)?.len() >= file.expected_bytes {
        println!(
            "{} is already installed at {}",
            file.filename,
            local_path.display()
        );
        return Ok(());
    }

    if let Some(parent) = local_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create model directory {}", parent.display()))?;
    }

    if local_path.exists() {
        println!(
            "{} has a partial local file at {} ({} / {} bytes); resuming.",
            file.filename,
            local_path.display(),
            fs::metadata(&local_path)?.len(),
            file.expected_bytes
        );
    }

    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        spec.repo_id, file.filename
    );
    println!("Downloading {} from {}", file.filename, spec.repo_id);
    println!("Expected size: {}", format_bytes(file.expected_bytes));

    let script_path = Path::new("scripts/download_hf_file_resume.ps1");
    if cfg!(windows) && script_path.exists() {
        let log_path = format!(
            "tmp/downloads/{}-{}.log",
            sanitize_model_key_for_path(spec.model_key),
            sanitize_model_key_for_path(file.filename)
        );
        let status = std::process::Command::new("powershell")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(script_path)
            .arg("-Url")
            .arg(&url)
            .arg("-Output")
            .arg(file.local_path)
            .arg("-ExpectedBytes")
            .arg(file.expected_bytes.to_string())
            .arg("-LogPath")
            .arg(&log_path)
            .status()
            .with_context(|| {
                format!("Failed to start resumable downloader for {}", file.filename)
            })?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "Resumable download failed for {}. See {}",
                file.filename,
                log_path
            ));
        }
    } else {
        let api = Api::new().context("Failed to initialize Hugging Face Hub API")?;
        let repo = api.model(spec.repo_id.to_string());
        let cached_path = repo
            .get(file.filename)
            .with_context(|| format!("Failed to download {}", file.filename))?;

        fs::copy(&cached_path, &local_path).with_context(|| {
            format!(
                "Failed to copy {} to {}",
                cached_path.display(),
                local_path.display()
            )
        })?;
    }

    let bytes = fs::metadata(&local_path)
        .with_context(|| format!("Downloaded file not found: {}", local_path.display()))?
        .len();
    if bytes < file.expected_bytes {
        return Err(anyhow::anyhow!(
            "{} is incomplete: {} / {} bytes",
            file.filename,
            bytes,
            file.expected_bytes
        ));
    }

    println!("Installed {} at {}", file.filename, local_path.display());
    Ok(())
}

fn run_uigen_output_path(output: &str) -> Result<PathBuf> {
    let output_path = PathBuf::from(output);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }
    Ok(output_path)
}

fn default_ui_model_key() -> String {
    std::env::var("FLOW_UIGEN_MODEL").unwrap_or_else(|_| DEFAULT_UI_MODEL_KEY.to_string())
}

fn sanitize_model_key_for_path(model_key: &str) -> String {
    model_key
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn resolve_ui_model_spec(model_key: Option<&str>) -> Result<ModelDownloadSpec> {
    let owned_default;
    let key = match model_key {
        Some(key) if !key.trim().is_empty() => key.trim(),
        _ => {
            owned_default = default_ui_model_key();
            owned_default.as_str()
        }
    };

    download_spec_for_model(key).with_context(|| {
        format!(
            "No built-in UI model is registered for '{key}'. Known UI models: {}, {}",
            WEBGEN_MODEL_KEY, QWENDEAN_MODEL_KEY
        )
    })
}

async fn run_uigen_google(model_key: Option<&str>) -> Result<()> {
    let spec = resolve_ui_model_spec(model_key)?;
    let output = format!(
        "{}/{}/index.html",
        UIMODEL_GOOGLE_OUTPUT_ROOT,
        sanitize_model_key_for_path(spec.model_key)
    );
    run_uigen(Some(spec.model_key), &output, GOOGLE_HOMEPAGE_PROMPT).await
}

async fn run_uigen(model_key: Option<&str>, output: &str, user_prompt: &str) -> Result<()> {
    let spec = resolve_ui_model_spec(model_key)?;
    let model_file = spec.primary_file();
    let model_path = PathBuf::from(model_file.local_path);
    if !model_path.exists() {
        return Err(anyhow::anyhow!(
            "UI model '{}' is missing at {}. Run: cargo run --release --bin flow -- --install-model {}",
            spec.model_key,
            model_path.display(),
            spec.model_key
        ));
    }
    if !model_download_spec_ready(&spec) {
        return Err(anyhow::anyhow!(
            "UI model '{}' is incomplete. Run: cargo run --release --bin flow -- --install-model {}",
            spec.model_key,
            spec.model_key
        ));
    }

    let output_path = run_uigen_output_path(output)?;
    let prompt = build_uigen_prompt(spec.model_key, user_prompt);
    let llm = LocalLlm::with_config(
        model_path.to_string_lossy().into_owned(),
        crate::models::LocalLlmConfig::uigen(),
    );

    println!(
        "Loading {} ({}) from {}",
        spec.display_name,
        spec.model_key,
        model_path.display()
    );
    llm.initialize().await?;
    println!("Generating UI into {}", output_path.display());

    let (raw, metrics) = llm.generate_ui_with_metrics(&prompt).await?;
    let html = strip_script_blocks(&clean_generated_code(&raw));
    validate_generated_html(&html).with_context(|| {
        let partial_path = output_path.with_extension("partial.html");
        let _ = fs::write(&partial_path, &html);
        format!(
            "{} returned incomplete HTML. Partial output saved to {}",
            spec.display_name,
            partial_path.display()
        )
    })?;
    fs::write(&output_path, html)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    println!(
        "Wrote {} ({} tokens in {:.2}s @ {:.1} tok/s)",
        output_path.display(),
        metrics.generated_tokens,
        metrics.total_time_ms as f64 / 1000.0,
        metrics.tokens_per_second
    );
    println!(
        "For screenshots, run: powershell -ExecutionPolicy Bypass -File scripts\\uigen_google_eval.ps1 -ModelKey {}",
        spec.model_key
    );
    Ok(())
}

fn run_uigen_vision_google() -> Result<()> {
    let output_dir = format!(
        "{}/{}",
        UIVISION_GOOGLE_OUTPUT_ROOT,
        sanitize_model_key_for_path(GEMMA4_FRONTEND_MODEL_KEY)
    );
    fs::create_dir_all(&output_dir).with_context(|| format!("Failed to create {}", output_dir))?;
    let screenshot = format!("{}/google-desktop.png", output_dir);
    let output = format!("{}/index.html", output_dir);
    capture_browser_screenshot(GOOGLE_URL, &screenshot, "1365,768")?;
    run_uigen_vision(&screenshot, &output, GOOGLE_VISION_PROMPT)
}

fn run_uigen_vision(screenshot: &str, output: &str, user_prompt: &str) -> Result<()> {
    let spec = download_spec_for_model(GEMMA4_FRONTEND_MODEL_KEY)
        .context("Gemma frontend vision model spec is missing")?;
    if !model_download_spec_ready(&spec) {
        return Err(anyhow::anyhow!(
            "Vision UI model '{}' is missing or incomplete. Run: cargo run --release --bin flow -- --install-model {}",
            spec.model_key,
            spec.model_key
        ));
    }

    let screenshot_path = PathBuf::from(screenshot);
    if !screenshot_path.exists() {
        return Err(anyhow::anyhow!(
            "Screenshot not found: {}",
            screenshot_path.display()
        ));
    }

    let output_path = run_uigen_output_path(output)?;
    let raw_path = output_path.with_extension("raw.txt");
    let model_file = spec
        .files
        .iter()
        .find(|file| file.role == ModelFileRole::Model)
        .context("Gemma frontend model file is missing from spec")?;
    let mmproj_file = spec
        .files
        .iter()
        .find(|file| file.role == ModelFileRole::Mmproj)
        .context("Gemma frontend mmproj file is missing from spec")?;

    println!(
        "Loading {} through llama-cpp-python vision bridge",
        spec.display_name
    );
    println!("Screenshot: {}", screenshot_path.display());
    println!("Output: {}", output_path.display());

    let output = std::process::Command::new("python")
        .arg("scripts/uigen_vision_llama_cpp.py")
        .arg("--model")
        .arg(model_file.local_path)
        .arg("--mmproj")
        .arg(mmproj_file.local_path)
        .arg("--image")
        .arg(&screenshot_path)
        .arg("--prompt")
        .arg(build_uigen_vision_prompt(user_prompt))
        .arg("--max-tokens")
        .arg(std::env::var("FLOW_UIGEN_VISION_MAX_TOKENS").unwrap_or_else(|_| "1800".to_string()))
        .arg("--ctx")
        .arg(std::env::var("FLOW_UIGEN_VISION_CTX").unwrap_or_else(|_| "8192".to_string()))
        .arg("--threads")
        .arg(uigen_thread_count().to_string())
        .output()
        .context("Failed to run scripts/uigen_vision_llama_cpp.py")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Vision UI generation failed. {}",
            stderr.trim()
        ));
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    fs::write(&raw_path, &raw)
        .with_context(|| format!("Failed to write {}", raw_path.display()))?;
    let html = strip_script_blocks(&clean_generated_code(&raw));
    validate_generated_html(&html).with_context(|| {
        let partial_path = output_path.with_extension("partial.html");
        let _ = fs::write(&partial_path, &html);
        format!(
            "{} returned incomplete HTML. Partial output saved to {}",
            spec.display_name,
            partial_path.display()
        )
    })?;
    fs::write(&output_path, html)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    println!("Wrote {}", output_path.display());
    Ok(())
}

fn build_uigen_vision_prompt(user_prompt: &str) -> String {
    format!(
        "{}\n\nOutput contract:\n- Return only a complete HTML document.\n- One <style> tag, no Markdown fences.\n- No external URLs, fonts, scripts, CDNs, or image assets.\n- Preserve the screenshot layout, spacing, and visual hierarchy.\n",
        user_prompt
    )
}

fn capture_browser_screenshot(url: &str, screenshot_path: &str, window_size: &str) -> Result<()> {
    let browser = resolve_headless_browser()
        .context("No headless Edge or Chrome executable was found for screenshot capture")?;
    let full_path = PathBuf::from(screenshot_path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let status = std::process::Command::new(&browser)
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--hide-scrollbars")
        .arg(format!("--window-size={window_size}"))
        .arg(format!("--screenshot={}", full_path.display()))
        .arg(url)
        .status()
        .with_context(|| format!("Failed to launch {}", browser.display()))?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "Browser screenshot failed for {url} with status {status}"
        ));
    }
    Ok(())
}

fn resolve_headless_browser() -> Option<PathBuf> {
    let candidates = [
        std::env::var("ProgramFiles")
            .ok()
            .map(|base| PathBuf::from(base).join("Microsoft/Edge/Application/msedge.exe")),
        std::env::var("ProgramFiles(x86)")
            .ok()
            .map(|base| PathBuf::from(base).join("Microsoft/Edge/Application/msedge.exe")),
        std::env::var("ProgramFiles")
            .ok()
            .map(|base| PathBuf::from(base).join("Google/Chrome/Application/chrome.exe")),
        std::env::var("ProgramFiles(x86)")
            .ok()
            .map(|base| PathBuf::from(base).join("Google/Chrome/Application/chrome.exe")),
    ];

    candidates
        .into_iter()
        .flatten()
        .find(|candidate| candidate.exists())
}

fn uigen_thread_count() -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get().saturating_sub(1).max(1))
        .unwrap_or(4)
}

fn build_uigen_prompt(_model_key: &str, user_prompt: &str) -> String {
    format!(
        "Create a complete single-file HTML document for this UI request:\n\n{}\n\nRequirements:\n- Return only HTML code.\n- Include all CSS in one <style> tag.\n- Do not use external images, scripts, CDNs, fonts, Tailwind, React imports, or Google assets.\n- Do not use @import or external URLs.\n- Keep CSS under 120 lines and the whole file under 220 lines.\n- Emit <body> content immediately after </style>.\n- Include </body> and </html>.\n- If you prefer React or shadcn/ui, translate that design sense into plain standalone HTML/CSS for this local screenshot eval.\n- Use a shadcn/ui-like product UI sense: calm spacing, clean borders, accessible controls, responsive layout.\n- Preserve the requested visual structure closely.\n",
        user_prompt
    )
}

fn clean_generated_code(raw: &str) -> String {
    let trimmed = LocalLlm::strip_thinking_tags(raw).trim().to_string();
    if let Some(start) = trimmed.find("```") {
        let after_start = &trimmed[start + 3..];
        let after_lang = after_start
            .strip_prefix("html")
            .or_else(|| after_start.strip_prefix("HTML"))
            .unwrap_or(after_start)
            .trim_start_matches(['\r', '\n']);
        if let Some(end) = after_lang.find("```") {
            return after_lang[..end].trim().to_string();
        }
    }
    if let Some(end) = trimmed.to_ascii_lowercase().find("</html>") {
        let closing_end = end + "</html>".len();
        return trimmed[..closing_end].trim().to_string();
    }
    trimmed
}

fn validate_generated_html(html: &str) -> Result<()> {
    let lower = html.to_ascii_lowercase();
    let has_document = lower.contains("<!doctype html") || lower.contains("<html");
    if !has_document
        || !lower.contains("<body")
        || !lower.contains("</body>")
        || !lower.contains("</html>")
    {
        return Err(anyhow::anyhow!(
            "generated output does not contain a complete HTML document"
        ));
    }

    if lower.contains("<script src")
        || lower.contains("cdn.tailwindcss.com")
        || lower.contains("@import")
        || lower.contains("fonts.googleapis.com")
    {
        return Err(anyhow::anyhow!(
            "generated output contains external scripts, fonts, imports, or CDN dependencies"
        ));
    }

    Ok(())
}

fn strip_script_blocks(html: &str) -> String {
    let mut output = html.to_string();
    loop {
        let lower = output.to_ascii_lowercase();
        let Some(start) = lower.find("<script") else {
            break;
        };
        let Some(relative_end) = lower[start..].find("</script>") else {
            output.truncate(start);
            break;
        };
        let end = start + relative_end + "</script>".len();
        output.replace_range(start..end, "");
    }
    output.trim().to_string()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModelFileRole {
    Model,
    Mmproj,
}

#[derive(Clone, Copy)]
struct ModelDownloadFile {
    filename: &'static str,
    local_path: &'static str,
    expected_bytes: u64,
    role: ModelFileRole,
}

#[derive(Clone, Copy)]
struct ModelDownloadSpec {
    model_key: &'static str,
    display_name: &'static str,
    repo_id: &'static str,
    files: &'static [ModelDownloadFile],
}

impl ModelDownloadSpec {
    fn primary_file(&self) -> &'static ModelDownloadFile {
        self.files
            .iter()
            .find(|file| file.role == ModelFileRole::Model)
            .unwrap_or(&self.files[0])
    }
}

fn download_spec_for_model(model_key: &str) -> Option<ModelDownloadSpec> {
    const QWEN3_06B_FILES: &[ModelDownloadFile] = &[ModelDownloadFile {
        filename: QWEN3_06B_MODEL_FILE,
        local_path: QWEN3_06B_MODEL_PATH,
        expected_bytes: 396_705_472,
        role: ModelFileRole::Model,
    }];
    const WEBGEN_FILES: &[ModelDownloadFile] = &[ModelDownloadFile {
        filename: WEBGEN_MODEL_FILE,
        local_path: WEBGEN_MODEL_PATH,
        expected_bytes: 2_497_286_912,
        role: ModelFileRole::Model,
    }];
    const QWENDEAN_FILES: &[ModelDownloadFile] = &[ModelDownloadFile {
        filename: QWENDEAN_MODEL_FILE,
        local_path: QWENDEAN_MODEL_PATH,
        expected_bytes: 2_497_280_928,
        role: ModelFileRole::Model,
    }];
    const QWEN35_9B_FILES: &[ModelDownloadFile] = &[ModelDownloadFile {
        filename: QWEN35_9B_MODEL_FILE,
        local_path: QWEN35_9B_MODEL_PATH,
        expected_bytes: 5_680_522_464,
        role: ModelFileRole::Model,
    }];
    const QWEN35_4B_REVISED_FILES: &[ModelDownloadFile] = &[ModelDownloadFile {
        filename: QWEN35_4B_REVISED_MODEL_FILE,
        local_path: QWEN35_4B_REVISED_MODEL_PATH,
        expected_bytes: 2_708_808_096,
        role: ModelFileRole::Model,
    }];
    const XLAM2_3B_TOOL_FILES: &[ModelDownloadFile] = &[ModelDownloadFile {
        filename: XLAM2_3B_TOOL_MODEL_FILE,
        local_path: XLAM2_3B_TOOL_MODEL_PATH,
        expected_bytes: 1_929_902_656,
        role: ModelFileRole::Model,
    }];
    const MINISTRAL3_3B_FILES: &[ModelDownloadFile] = &[ModelDownloadFile {
        filename: MINISTRAL3_3B_MODEL_FILE,
        local_path: MINISTRAL3_3B_MODEL_PATH,
        expected_bytes: 2_146_497_824,
        role: ModelFileRole::Model,
    }];
    const GRANITE4_H_MICRO_FILES: &[ModelDownloadFile] = &[ModelDownloadFile {
        filename: GRANITE4_H_MICRO_MODEL_FILE,
        local_path: GRANITE4_H_MICRO_MODEL_PATH,
        expected_bytes: 1_942_564_512,
        role: ModelFileRole::Model,
    }];
    const PHI4_MINI_FILES: &[ModelDownloadFile] = &[ModelDownloadFile {
        filename: PHI4_MINI_MODEL_FILE,
        local_path: PHI4_MINI_MODEL_PATH,
        expected_bytes: 2_493_840_192,
        role: ModelFileRole::Model,
    }];
    const SMOLLM3_3B_FILES: &[ModelDownloadFile] = &[ModelDownloadFile {
        filename: SMOLLM3_3B_MODEL_FILE,
        local_path: SMOLLM3_3B_MODEL_PATH,
        expected_bytes: 1_915_305_312,
        role: ModelFileRole::Model,
    }];
    const GEMMA4_FRONTEND_FILES: &[ModelDownloadFile] = &[
        ModelDownloadFile {
            filename: GEMMA4_FRONTEND_MODEL_FILE,
            local_path: GEMMA4_FRONTEND_MODEL_PATH,
            expected_bytes: 5_335_285_376,
            role: ModelFileRole::Model,
        },
        ModelDownloadFile {
            filename: GEMMA4_FRONTEND_MMPROJ_FILE,
            local_path: GEMMA4_FRONTEND_MMPROJ_PATH,
            expected_bytes: 991_551_904,
            role: ModelFileRole::Mmproj,
        },
    ];
    const UIGEN_FX_FILES: &[ModelDownloadFile] = &[ModelDownloadFile {
        filename: UIGEN_FX_MODEL_FILE,
        local_path: UIGEN_FX_MODEL_PATH,
        expected_bytes: 2_716_064_480,
        role: ModelFileRole::Model,
    }];

    match model_key {
        QWEN3_06B_MODEL_KEY => Some(ModelDownloadSpec {
            model_key: QWEN3_06B_MODEL_KEY,
            display_name: "Qwen3 0.6B Q4_K_M",
            repo_id: QWEN3_06B_MODEL_REPO,
            files: QWEN3_06B_FILES,
        }),
        WEBGEN_MODEL_KEY => Some(ModelDownloadSpec {
            model_key: WEBGEN_MODEL_KEY,
            display_name: "WEBGEN 4B Preview i1 Q4_K_M",
            repo_id: WEBGEN_MODEL_REPO,
            files: WEBGEN_FILES,
        }),
        QWENDEAN_MODEL_KEY => Some(ModelDownloadSpec {
            model_key: QWENDEAN_MODEL_KEY,
            display_name: "Qwendean 4B Q4_K_M",
            repo_id: QWENDEAN_MODEL_REPO,
            files: QWENDEAN_FILES,
        }),
        QWEN35_9B_MODEL_KEY => Some(ModelDownloadSpec {
            model_key: QWEN35_9B_MODEL_KEY,
            display_name: "Qwen3.5 9B Q4_K_M",
            repo_id: QWEN35_9B_MODEL_REPO,
            files: QWEN35_9B_FILES,
        }),
        QWEN35_4B_REVISED_MODEL_KEY => Some(ModelDownloadSpec {
            model_key: QWEN35_4B_REVISED_MODEL_KEY,
            display_name: "Qwen3.5 4B Revised Q4_K_M",
            repo_id: QWEN35_4B_REVISED_MODEL_REPO,
            files: QWEN35_4B_REVISED_FILES,
        }),
        XLAM2_3B_TOOL_MODEL_KEY => Some(ModelDownloadSpec {
            model_key: XLAM2_3B_TOOL_MODEL_KEY,
            display_name: "xLAM-2 3B Function Calling Q4_K_M",
            repo_id: XLAM2_3B_TOOL_MODEL_REPO,
            files: XLAM2_3B_TOOL_FILES,
        }),
        MINISTRAL3_3B_MODEL_KEY => Some(ModelDownloadSpec {
            model_key: MINISTRAL3_3B_MODEL_KEY,
            display_name: "Ministral 3 3B Instruct Q4_K_M",
            repo_id: MINISTRAL3_3B_MODEL_REPO,
            files: MINISTRAL3_3B_FILES,
        }),
        GRANITE4_H_MICRO_MODEL_KEY => Some(ModelDownloadSpec {
            model_key: GRANITE4_H_MICRO_MODEL_KEY,
            display_name: "Granite 4.0 H Micro Q4_K_M",
            repo_id: GRANITE4_H_MICRO_MODEL_REPO,
            files: GRANITE4_H_MICRO_FILES,
        }),
        PHI4_MINI_MODEL_KEY => Some(ModelDownloadSpec {
            model_key: PHI4_MINI_MODEL_KEY,
            display_name: "Phi-4 Mini Instruct Q4_K_M",
            repo_id: PHI4_MINI_MODEL_REPO,
            files: PHI4_MINI_FILES,
        }),
        SMOLLM3_3B_MODEL_KEY => Some(ModelDownloadSpec {
            model_key: SMOLLM3_3B_MODEL_KEY,
            display_name: "SmolLM3 3B Q4_K_M",
            repo_id: SMOLLM3_3B_MODEL_REPO,
            files: SMOLLM3_3B_FILES,
        }),
        GEMMA4_FRONTEND_MODEL_KEY => Some(ModelDownloadSpec {
            model_key: GEMMA4_FRONTEND_MODEL_KEY,
            display_name: "Gemma 4 E4B Frontend Q4_K_M + BF16 mmproj",
            repo_id: GEMMA4_FRONTEND_MODEL_REPO,
            files: GEMMA4_FRONTEND_FILES,
        }),
        UIGEN_FX_MODEL_KEY => Some(ModelDownloadSpec {
            model_key: UIGEN_FX_MODEL_KEY,
            display_name: "UIGEN-FX 4B Preview Q4_K_M",
            repo_id: UIGEN_FX_MODEL_REPO,
            files: UIGEN_FX_FILES,
        }),
        _ => None,
    }
}

fn model_download_spec_ready(spec: &ModelDownloadSpec) -> bool {
    spec.files.iter().all(|file| {
        let path = Path::new(file.local_path);
        path.exists()
            && fs::metadata(path)
                .map(|metadata| metadata.len() >= file.expected_bytes)
                .unwrap_or(false)
    })
}

fn model_artifact_ready(manifest: &crate::runtime::ModelManifest) -> bool {
    let Some(local_path) = manifest.local_path.as_deref() else {
        return false;
    };
    let path = Path::new(local_path);
    if !path.exists() {
        return false;
    }
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if let Some(spec) = download_spec_for_model(&manifest.key) {
        return model_download_spec_ready(&spec);
    }
    metadata.len() > 0
}

fn print_plan(broker: &RuntimeBroker, plan: &ExecutionPlan) {
    println!("Flow Runtime Plan");
    println!("=================");
    println!("Modality: {:?}", plan.modality);
    println!("Device tier: {:?}", plan.device_tier);
    println!(
        "Selected model: {}",
        plan.selected_model.as_deref().unwrap_or("none")
    );
    println!(
        "Runtime: {}",
        plan.selected_runtime
            .map(|runtime| format!("{runtime:?}"))
            .unwrap_or_else(|| "none".to_string())
    );
    println!("Launch: {:?}", plan.launch);

    if let Some(bytes) = plan.estimated_memory_bytes {
        println!("Estimated memory: {}", format_bytes(bytes));
    }

    println!();
    println!("Reasons:");
    for reason in &plan.reasons {
        println!("  - {}", reason);
    }

    if let Some(artifact) = &plan.artifact {
        println!();
        println!("Artifact:");
        println!(
            "  {} files in {}, runtime {:?}, redistributable={}",
            artifact.files.len(),
            artifact.root_dir,
            artifact.runtime,
            artifact.redistributable
        );
    }

    if let Some(job) = &plan.conversion_job {
        println!();
        println!("Conversion:");
        println!(
            "  lane={:?}, target={:?}, command={}",
            job.lane,
            job.target_format,
            job.command_preview.join(" ")
        );
    }

    if let Some(publish) = &plan.publish_record {
        println!();
        println!(
            "Publish: {:?} -> {}",
            publish.status, publish.destination_repo
        );
        if let Some(reason) = &publish.reason {
            println!("  reason={}", reason);
        }
    }

    if let Some(reason) = &plan.unsupported_reason {
        println!();
        println!("Unsupported: {}", reason);
    }

    if matches!(plan.modality, Modality::Chat) {
        let recommended = broker.models_for(Modality::Chat);
        if !recommended.is_empty() {
            println!();
            println!("Local chat candidates:");
            for candidate in recommended {
                let local = candidate
                    .local_path
                    .as_deref()
                    .map(Path::new)
                    .map(Path::exists)
                    .unwrap_or(false);
                println!(
                    "  - {} [{}]",
                    candidate.key,
                    if local { "local" } else { "missing" }
                );
            }
        }
    }
}

fn run_grammar(text: &str, fix: bool) -> Result<()> {
    let checker = HarperGrammarChecker::new();
    let issues = checker.analyze(text)?;

    println!("Grammar");
    println!("=======\n");
    println!("Input: {}", text);

    if issues.is_empty() {
        println!("Issues: none");
    } else {
        println!("Issues:");
        for issue in &issues {
            println!(
                "  - {}..{}: {}{}",
                issue.start,
                issue.end,
                issue.message,
                issue
                    .replacement
                    .as_ref()
                    .map(|replacement| format!(" -> {}", replacement))
                    .unwrap_or_default()
            );
        }
    }

    if fix {
        let corrected = checker.correct(text)?;
        println!();
        println!("Corrected:");
        println!("{}", corrected);
    }

    Ok(())
}

fn print_wake_words(broker: &RuntimeBroker) {
    println!("Wake Words");
    println!("==========");
    println!(
        "Frontend resources: {}",
        if WakeWordDetector::is_available() {
            "present"
        } else {
            "missing"
        }
    );

    let installed = broker.activation().wake_words.iter().collect::<Vec<_>>();
    for definition in wake_command_definitions() {
        let item = installed
            .iter()
            .copied()
            .find(|item| item.command_key == definition.command_key);
        let model_path = Path::new("models/wake_words").join(definition.model_filename);
        println!("{} ({})", definition.command_key, definition.phrase);
        println!(
            "  status={}",
            if item.is_some() {
                "installed"
            } else {
                "missing"
            }
        );
        println!("  model={}", model_path.to_string_lossy());
        println!("  threshold={}%", definition.threshold);
        println!(
            "  aliases={}",
            if definition.aliases.is_empty() {
                "-".to_string()
            } else {
                definition.aliases.join(", ")
            }
        );
    }
}

fn print_blueprint(host: HostSurface) {
    let registry = FlowEmbeddingRegistry::detect();
    let blueprint = registry.blueprint(host);

    println!("Flow Embedding Blueprint");
    println!("========================");
    println!("Host: {:?}", blueprint.host);
    println!("Mode: {:?}", blueprint.integration_mode);
    println!(
        "Device: {} / {:?}",
        blueprint.device_profile.os, blueprint.device_profile.tier
    );
    println!();

    println!("Core subsystems:");
    for subsystem in &blueprint.core_subsystems {
        println!("  - {:?}", subsystem);
    }

    if !blueprint.optional_subsystems.is_empty() {
        println!();
        println!("Optional subsystems:");
        for subsystem in &blueprint.optional_subsystems {
            println!("  - {:?}", subsystem);
        }
    }

    println!();
    println!("Adjacent projects:");
    for project in &blueprint.adjacent_projects {
        println!(
            "  - {} [{}] {}",
            project.key,
            if project.detected {
                "detected"
            } else {
                "missing"
            },
            project.purpose
        );
    }

    println!();
    println!("Workspace projects:");
    for project in &blueprint.workspace_projects {
        println!(
            "  - {}: {} ({}%)",
            project.key, project.role, project.completeness_score
        );
    }

    println!();
    println!(
        "Providers: folder_present={}, auto_switch_local_and_remote={}",
        blueprint.provider_strategy.folder_present,
        blueprint.provider_strategy.auto_switch_local_and_remote
    );
    println!(
        "Provider catalog sources: {}",
        blueprint
            .provider_catalog_plan
            .sources
            .iter()
            .map(|source| format!("{source:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "Serializer: folder_present={}, format={}, rkyv={}, memmap={}",
        blueprint.serializer_strategy.folder_present,
        blueprint.serializer_strategy.format_name,
        blueprint.serializer_strategy.uses_rkyv,
        blueprint.serializer_strategy.uses_memmap
    );
    println!(
        "Metasearch: folder_present={}",
        blueprint.search_strategy.folder_present
    );
    println!(
        "RLM: folder_present={}",
        blueprint.long_context_strategy.folder_present
    );
    println!(
        "Forge: folder_present={}, multi_remote={}",
        blueprint.forge_strategy.folder_present, blueprint.forge_strategy.supports_multi_remote
    );

    println!();
    println!("Notes:");
    for note in &blueprint.notes {
        println!("  - {}", note);
    }
}

fn print_browser_profile(flavor: BrowserHostFlavor) {
    let engine = FlowBrowserEngine::detect();
    let profile = engine.detect_browser_capabilities(flavor, None, None, None, None, None);

    println!("Flow Browser Capability Profile");
    println!("===============================");
    println!("Flavor: {:?}", profile.flavor);
    println!("webgpu={}", profile.webgpu);
    println!("wasm_threads={}", profile.wasm_threads);
    println!("cross_origin_isolated={}", profile.cross_origin_isolated);
    println!("opfs={}", profile.opfs);
    println!("indexeddb={}", profile.indexeddb);
    println!("side_panel={}", profile.side_panel);
    println!("sidebar_action={}", profile.sidebar_action);
    println!("offscreen_document={}", profile.offscreen_document);
    println!(
        "background_service_worker={}",
        profile.background_service_worker
    );
    println!();
    println!("Notes:");
    for note in profile.notes {
        println!("  - {}", note);
    }
}

fn print_browser_plan(
    flavor: BrowserHostFlavor,
    task: BrowserTask,
    modality: Modality,
    model: Option<String>,
    remote_fallback: bool,
) {
    let engine = FlowBrowserEngine::detect();
    let capabilities = engine.detect_browser_capabilities(flavor, None, None, None, None, None);
    let plan = engine.plan_browser_execution(crate::browser::BrowserExecutionRequest {
        task,
        modality,
        local_only: !remote_fallback,
        preferred_model: model,
        allow_remote_fallback: remote_fallback,
        capabilities,
    });

    println!("Flow Browser Execution Plan");
    println!("===========================");
    println!("Task: {:?}", plan.task);
    println!("Modality: {:?}", plan.modality);
    println!(
        "Selected model: {}",
        plan.selected_model.unwrap_or_else(|| "-".to_string())
    );
    println!(
        "Pack key: {}",
        plan.pack_key.unwrap_or_else(|| "-".to_string())
    );
    println!("Backend: {:?}", plan.backend);
    println!("Storage: {:?}", plan.storage_backend);
    println!(
        "Worker: {}",
        plan.worker_kind
            .map(|kind| format!("{kind:?}"))
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "Device target: {}",
        plan.device_target
            .map(|target| format!("{target:?}"))
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "UI surfaces: {}",
        plan.ui_surfaces
            .iter()
            .map(|surface| format!("{surface:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("Local only: {}", plan.local_only);
    println!("Remote allowed: {}", plan.remote_allowed);
    println!();
    println!("Enabled features:");
    for feature in &plan.enabled_features {
        println!("  - {}", feature);
    }
    if !plan.disabled_features.is_empty() {
        println!();
        println!("Disabled features:");
        for feature in &plan.disabled_features {
            println!("  - {}", feature);
        }
    }
    println!();
    println!("Reasons:");
    for reason in &plan.reasons {
        println!("  - {}", reason);
    }
    if let Some(reason) = &plan.unsupported_reason {
        println!();
        println!("Unsupported: {}", reason);
    }
}

fn print_browser_packs() {
    let catalog = default_browser_pack_catalog();

    println!("Flow Browser Pack Catalog");
    println!("=========================");
    for pack in catalog {
        println!("{}", pack.display_name);
        println!("  model_key={}", pack.model_key);
        println!("  pack_key={}", pack.pack_key);
        println!("  modality={:?}", pack.modality);
        println!("  backend={:?}", pack.backend);
        println!(
            "  support=chromium:{} firefox:{} safari:{} web:{} webgpu:{}",
            pack.browser_support.chromium,
            pack.browser_support.firefox,
            pack.browser_support.safari,
            pack.browser_support.standalone_web,
            pack.browser_support.requires_webgpu
        );
        println!(
            "  files={}",
            pack.files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

fn print_production_config(target: FlowIntegrationTarget) -> Result<()> {
    let runtime = crate::dx::DxFlowRuntime::detect();
    println!("{}", runtime.production_config_json(target)?);
    Ok(())
}

fn export_production_bundle_cli(output_dir: &str) -> Result<()> {
    let runtime = crate::dx::DxFlowRuntime::detect();
    let manifest = runtime.export_production_bundle(resolve_repo_relative_path(output_dir))?;
    println!("Exported Flow production bundle");
    println!("===============================");
    println!("Directory: {}", output_dir);
    println!("Device tier: {}", manifest.device_tier);
    println!(
        "Models: text={} stt={} tts={}",
        manifest.selected_text_model.as_deref().unwrap_or("none"),
        manifest.selected_stt_model.as_deref().unwrap_or("none"),
        manifest.selected_tts_model.as_deref().unwrap_or("none"),
    );
    println!("All models ready: {}", manifest.all_models_ready);
    println!("Files:");
    for entry in &manifest.entries {
        println!("  - {}", entry.filename);
    }
    println!("  - manifest.json");
    println!("  - README.txt");
    Ok(())
}

fn print_release_summary() -> Result<()> {
    let runtime = crate::dx::DxFlowRuntime::detect();
    println!("{}", runtime.release_summary()?.to_pretty_json()?);
    Ok(())
}

fn export_release_summary_cli(output_dir: &str) -> Result<()> {
    let runtime = crate::dx::DxFlowRuntime::detect();
    let summary = runtime.export_release_summary(resolve_repo_relative_path(output_dir))?;
    println!("Exported Flow release summary");
    println!("=============================");
    println!("Directory: {}", output_dir);
    println!(
        "Production bundle ready: {}",
        summary.production_bundle_ready
    );
    println!(
        "Artifacts ready: {} / {}",
        summary
            .browser_release_artifacts
            .iter()
            .filter(|artifact| artifact.exists)
            .count(),
        summary.browser_release_artifacts.len()
    );
    println!("Files:");
    println!("  - flow-release-summary.json");
    println!("  - FLOW_RELEASE_HANDOFF.md");
    Ok(())
}

fn resolve_repo_relative_path(path: &str) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
    }
}

async fn load_chat_llm(broker: &RuntimeBroker) -> Result<LocalLlm> {
    let mut request = BrokerRequest::new(Modality::Chat);
    request.allow_conversion = false;
    request.allow_publish = false;
    let plan = broker.build_plan(request);

    let selected = plan
        .selected_model
        .clone()
        .context("No chat model selected by the runtime broker")?;
    let manifest = broker
        .catalog()
        .iter()
        .find(|candidate| candidate.key == selected)
        .context("Selected chat model is missing from the broker catalog")?;
    let model_path = manifest
        .local_path
        .clone()
        .context("Selected chat model has no local path")?;

    if !Path::new(&model_path).exists() {
        return Err(anyhow::anyhow!(
            "Selected chat model '{}' is not present at {}",
            manifest.key,
            model_path
        ));
    }

    let llm = LocalLlm::with_model_path(model_path);
    llm.initialize().await?;
    Ok(llm)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveSessionMode {
    VoiceRoundTrip,
    DictateToFocusedInput,
}

impl LiveSessionMode {
    fn title(self) -> &'static str {
        match self {
            Self::VoiceRoundTrip => "Flow Live",
            Self::DictateToFocusedInput => "Flow Dictation",
        }
    }

    fn terminal_title(self) -> &'static str {
        match self {
            Self::VoiceRoundTrip => "Flow Live - listening",
            Self::DictateToFocusedInput => "Flow Dictation - listening",
        }
    }

    fn uses_focused_input(self) -> bool {
        matches!(self, Self::DictateToFocusedInput)
    }

    fn allows_bare_hotkeys(self) -> bool {
        matches!(self, Self::VoiceRoundTrip)
    }
}

async fn run_live_mode() -> Result<()> {
    run_live_session(LiveSessionMode::VoiceRoundTrip).await
}

async fn run_dictation_mode() -> Result<()> {
    run_live_session(LiveSessionMode::DictateToFocusedInput).await
}

async fn run_live_session(mode: LiveSessionMode) -> Result<()> {
    let broker = RuntimeBroker::detect();

    set_terminal_title(mode.terminal_title());
    println!("{}", mode.title());
    println!("{}", "=".repeat(mode.title().len()));
    println!("Device tier: {:?}", broker.device_profile().tier);
    println!(
        "Push-to-talk: {}",
        shortcut_label(
            &broker.activation().push_to_talk.modifiers,
            &broker.activation().push_to_talk.key
        )
    );
    println!(
        "Hands-free toggle: {}",
        shortcut_label(
            &broker.activation().hands_free_toggle.modifiers,
            &broker.activation().hands_free_toggle.key
        )
    );
    if !broker.activation().wake_words.is_empty() {
        println!(
            "Wake words: {}",
            broker
                .activation()
                .wake_words
                .iter()
                .map(|item| item.command_key.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!();

    if mode.uses_focused_input() {
        println!(
            "[indicator] compact status lives in this terminal title; keep your target input focused"
        );
        println!("[init] arming microphone and wake detector; STT loads only after speech");
    } else {
        println!("[init] arming microphone and wake detector; STT/TTS/LLM load on demand");
    }
    let mut stt: Option<LocalSttEngine> = if mode.uses_focused_input() {
        Some(load_live_stt_engine(&broker, mode)?)
    } else {
        None
    };
    let mut tts: Option<KokoroTTS> = None;
    let mut llm: Option<LocalLlm> = None;
    let mut input_bridge = mode.uses_focused_input().then(|| {
        NativeSelectionBridge::live(OperatingSystemFamily::from_host_label(std::env::consts::OS))
    });

    let wakeword_detector = WakeWordDetector::from_config(&broker.activation().wake_words)?
        .map(|detector| Arc::new(Mutex::new(detector)));
    let wakeword_available = wakeword_detector.is_some();

    let sample_rate = 16_000_u32;
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No input device found"))?;
    let config = device.default_input_config()?;
    let channels = config.channels() as usize;
    let input_sample_rate = config.sample_rate();

    println!(
        "[audio] input={} Hz, channels={}, processing={} Hz",
        input_sample_rate, channels, sample_rate
    );
    if mode.allows_bare_hotkeys() {
        println!("[ready] press Enter to record, Space to stop and process");
    }
    println!("[ready] press Ctrl+Shift+Space to toggle recording");
    if wakeword_available {
        println!("[ready] local wake words are armed");
    } else if mode.uses_focused_input() {
        println!(
            "[ready] wake models are missing; speech activity will start recording automatically"
        );
        println!("[ready] Ctrl+Shift+Space still works as a manual override");
    } else {
        println!(
            "[ready] wake models are missing; Ctrl+Shift+Space is the local fallback until ONNX wake files exist"
        );
    }
    println!("[ready] press Ctrl+C to exit\n");

    let is_recording = Arc::new(AtomicBool::new(false));
    let should_process = Arc::new(AtomicBool::new(false));
    let speech_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let last_voice_at = Arc::new(Mutex::new(Instant::now()));

    let is_recording_kb = Arc::clone(&is_recording);
    let should_process_kb = Arc::clone(&should_process);
    let speech_buffer_kb = Arc::clone(&speech_buffer);
    let last_voice_at_kb = Arc::clone(&last_voice_at);
    let allow_bare_hotkeys = mode.allows_bare_hotkeys();

    std::thread::spawn(move || {
        use rdev::{Event, EventType, Key, listen};

        let ctrl_down = Arc::new(AtomicBool::new(false));
        let shift_down = Arc::new(AtomicBool::new(false));
        let ctrl_down_cb = Arc::clone(&ctrl_down);
        let shift_down_cb = Arc::clone(&shift_down);

        let callback = move |event: Event| match event.event_type {
            EventType::KeyPress(Key::ControlLeft | Key::ControlRight) => {
                ctrl_down_cb.store(true, Ordering::Relaxed);
            }
            EventType::KeyRelease(Key::ControlLeft | Key::ControlRight) => {
                ctrl_down_cb.store(false, Ordering::Relaxed);
            }
            EventType::KeyPress(Key::ShiftLeft | Key::ShiftRight) => {
                shift_down_cb.store(true, Ordering::Relaxed);
            }
            EventType::KeyRelease(Key::ShiftLeft | Key::ShiftRight) => {
                shift_down_cb.store(false, Ordering::Relaxed);
            }
            EventType::KeyPress(Key::Return) if allow_bare_hotkeys => {
                if !is_recording_kb.load(Ordering::Relaxed) {
                    if let Ok(mut buffer) = speech_buffer_kb.lock() {
                        buffer.clear();
                    }
                    if let Ok(mut last_voice) = last_voice_at_kb.lock() {
                        *last_voice = Instant::now();
                    }
                    is_recording_kb.store(true, Ordering::Relaxed);
                    println!("[record] started");
                    set_terminal_title("Flow - recording");
                }
            }
            EventType::KeyPress(Key::Space)
                if ctrl_down_cb.load(Ordering::Relaxed)
                    && shift_down_cb.load(Ordering::Relaxed) =>
            {
                let new_state = !is_recording_kb.load(Ordering::Relaxed);
                if new_state {
                    if let Ok(mut buffer) = speech_buffer_kb.lock() {
                        buffer.clear();
                    }
                    if let Ok(mut last_voice) = last_voice_at_kb.lock() {
                        *last_voice = Instant::now();
                    }
                    is_recording_kb.store(true, Ordering::Relaxed);
                    println!("[record] toggled on");
                    set_terminal_title("Flow - recording");
                } else {
                    is_recording_kb.store(false, Ordering::Relaxed);
                    should_process_kb.store(true, Ordering::Relaxed);
                    println!("[record] toggled off, processing");
                    set_terminal_title("Flow - processing");
                }
            }
            EventType::KeyPress(Key::Space) if allow_bare_hotkeys => {
                if is_recording_kb.load(Ordering::Relaxed) {
                    is_recording_kb.store(false, Ordering::Relaxed);
                    should_process_kb.store(true, Ordering::Relaxed);
                    println!("[record] stopped, processing");
                    set_terminal_title("Flow - processing");
                }
            }
            _ => {}
        };

        if let Err(error) = listen(callback) {
            eprintln!("[error] keyboard listener: {:?}", error);
        }
    });

    let is_recording_audio = Arc::clone(&is_recording);
    let should_process_audio = Arc::clone(&should_process);
    let speech_buffer_audio = Arc::clone(&speech_buffer);
    let pre_roll_buffer_audio = Arc::new(Mutex::new(Vec::<f32>::new()));
    let wakeword_detector_audio = wakeword_detector.clone();
    let last_voice_at_audio = Arc::clone(&last_voice_at);
    let mut vad_gate = NoiseGateVAD::new(sample_rate).ok();
    let silence_timeout = Duration::from_millis(1500);
    let min_auto_stop_samples = sample_rate as usize / 2;
    let auto_record_on_voice = mode.uses_focused_input() && !wakeword_available;
    let auto_start_min_rms = 0.00002_f32;
    let pre_roll_limit_samples = (sample_rate as usize * 3) / 4;
    let mut last_idle_meter_at = Instant::now();

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &_| {
            let mono: Vec<f32> = if channels == 2 {
                data.chunks(2)
                    .map(|chunk| (chunk[0] + chunk.get(1).copied().unwrap_or(0.0)) / 2.0)
                    .collect()
            } else {
                data.to_vec()
            };

            let processed = if input_sample_rate != sample_rate && input_sample_rate > sample_rate {
                let ratio = (input_sample_rate / sample_rate).max(1) as usize;
                mono.iter().step_by(ratio).copied().collect::<Vec<_>>()
            } else {
                mono
            };

            let input_rms = rms_energy(&processed);
            let (speech_samples, is_speech) = if let Some(vad) = vad_gate.as_mut() {
                let (gated, is_speech, _) = vad.process(&processed);
                (gated, is_speech)
            } else {
                (processed.clone(), input_rms > auto_start_min_rms)
            };
            let voice_active = if auto_record_on_voice {
                input_rms >= auto_start_min_rms || is_speech
            } else {
                is_speech
            };
            let record_samples = if auto_record_on_voice {
                &processed
            } else {
                &speech_samples
            };

            if !is_recording_audio.load(Ordering::Relaxed) {
                if auto_record_on_voice {
                    if let Ok(mut pre_roll) = pre_roll_buffer_audio.try_lock() {
                        pre_roll.extend_from_slice(record_samples);
                        if pre_roll.len() > pre_roll_limit_samples {
                            let excess = pre_roll.len() - pre_roll_limit_samples;
                            pre_roll.drain(..excess);
                        }
                    }
                }

                if auto_record_on_voice && last_idle_meter_at.elapsed() >= Duration::from_secs(2) {
                    println!("[meter] idle rms={:.7}", input_rms);
                    last_idle_meter_at = Instant::now();
                }

                if let Some(detector) = &wakeword_detector_audio {
                    if let Ok(mut detector) = detector.try_lock() {
                        match detector.feed_f32(&processed) {
                            Ok(Some(detection)) => {
                                if let Ok(mut buffer) = speech_buffer_audio.try_lock() {
                                    buffer.clear();
                                }
                                if let Ok(mut last_voice) = last_voice_at_audio.try_lock() {
                                    *last_voice = Instant::now();
                                }
                                is_recording_audio.store(true, Ordering::Relaxed);
                                println!(
                                    "[wake] '{}' ({}) detected at {:.0}% confidence",
                                    detection.command_key,
                                    detection.phrase,
                                    detection.confidence * 100.0
                                );
                            }
                            Ok(None) => {}
                            Err(error) => {
                                eprintln!("[warn] wake-word detection error: {}", error);
                            }
                        }
                    }
                }

                if auto_record_on_voice && voice_active {
                    if let Ok(mut buffer) = speech_buffer_audio.try_lock() {
                        buffer.clear();
                        if let Ok(pre_roll) = pre_roll_buffer_audio.try_lock() {
                            buffer.extend_from_slice(&pre_roll);
                        }
                        buffer.extend_from_slice(record_samples);
                    }
                    if let Ok(mut pre_roll) = pre_roll_buffer_audio.try_lock() {
                        pre_roll.clear();
                    }
                    if let Ok(mut last_voice) = last_voice_at_audio.try_lock() {
                        *last_voice = Instant::now();
                    }
                    is_recording_audio.store(true, Ordering::Relaxed);
                    println!("[voice] speech detected, recording (rms={:.5})", input_rms);
                    set_terminal_title("Flow - recording");
                }
                return;
            }

            if let Ok(mut buffer) = speech_buffer_audio.try_lock() {
                buffer.extend_from_slice(record_samples);
                if voice_active {
                    if let Ok(mut last_voice) = last_voice_at_audio.try_lock() {
                        *last_voice = Instant::now();
                    }
                } else if buffer.len() >= min_auto_stop_samples {
                    if let Ok(last_voice) = last_voice_at_audio.try_lock() {
                        if last_voice.elapsed() >= silence_timeout {
                            is_recording_audio.store(false, Ordering::Relaxed);
                            should_process_audio.store(true, Ordering::Relaxed);
                            println!("[record] silence detected, processing");
                            set_terminal_title("Flow - processing");
                        }
                    }
                }
            }
        },
        |error| eprintln!("[error] audio stream: {}", error),
        None,
    )?;

    stream.play()?;

    let mut recording_counter = 1_u32;

    loop {
        if should_process.load(Ordering::Relaxed) {
            should_process.store(false, Ordering::Relaxed);

            let recorded_samples = {
                let mut buffer = speech_buffer.lock().unwrap();
                let samples = buffer.clone();
                buffer.clear();
                samples
            };

            if recorded_samples.is_empty() {
                println!("[warn] no audio recorded\n");
                continue;
            }

            let energy = recorded_samples
                .iter()
                .map(|sample| sample * sample)
                .sum::<f32>()
                / recorded_samples.len() as f32;
            let rms = energy.sqrt();

            println!(
                "[process] {} samples ({:.2}s), rms={:.6}",
                recorded_samples.len(),
                recorded_samples.len() as f32 / sample_rate as f32,
                rms
            );

            let prepared = prepare_recording_for_stt(&recorded_samples, sample_rate, mode);
            if prepared.samples.len() < minimum_stt_samples(sample_rate, mode) {
                println!(
                    "[warn] captured clip is too short after cleanup ({:.2}s); keep speaking a little longer\n",
                    prepared.samples.len() as f32 / sample_rate as f32
                );
                set_terminal_title(mode.terminal_title());
                continue;
            }

            println!(
                "[process] prepared {:.2}s, noise_floor={:.7}, gain={:.1}x, final_rms={:.6}",
                prepared.samples.len() as f32 / sample_rate as f32,
                prepared.noise_floor,
                prepared.gain,
                prepared.final_rms
            );

            let numbered_file = format!("recording_{recording_counter:04}.wav");
            recording_counter += 1;
            write_wav(&numbered_file, sample_rate, &prepared.samples)?;
            write_wav("temp_live_recording.wav", sample_rate, &prepared.samples)?;
            println!("[file] saved {}", numbered_file);

            set_terminal_title("Flow - transcribing");
            print!("[stt] transcribing... ");
            std::io::stdout().flush()?;
            if stt.is_none() {
                stt = Some(load_live_stt_engine(&broker, mode)?);
            }
            let raw_text = stt
                .as_mut()
                .context("Local STT engine was not initialized")?
                .transcribe("temp_live_recording.wav")?;
            println!("\"{}\"", raw_text);

            if raw_text.trim().len() < 3 {
                println!("[warn] transcription too short\n");
                set_terminal_title(mode.terminal_title());
                continue;
            }

            match mode {
                LiveSessionMode::DictateToFocusedInput => {
                    let dictated_text = prepare_dictation_text(&raw_text);
                    print!("[input] inserting into focused input... ");
                    std::io::stdout().flush()?;
                    let inserted = input_bridge
                        .as_mut()
                        .map(|bridge| bridge.replace_selection(&dictated_text))
                        .unwrap_or(false);
                    if inserted {
                        println!("done");
                    } else {
                        println!("failed");
                        println!(
                            "[warn] desktop paste bridge is unavailable; transcript: {}",
                            dictated_text
                        );
                    }
                    set_terminal_title(mode.terminal_title());
                    println!();
                }
                LiveSessionMode::VoiceRoundTrip => {
                    print!("[ai] cleaning... ");
                    std::io::stdout().flush()?;
                    if llm.is_none() {
                        llm = Some(load_chat_llm(&broker).await?);
                    }
                    let cleaned_text = llm
                        .as_ref()
                        .context("Local chat model was not initialized")?
                        .clean_speech(&raw_text)
                        .await?;
                    println!("\"{}\"", cleaned_text);

                    print!("[tts] speaking... ");
                    std::io::stdout().flush()?;
                    if tts.is_none() {
                        tts = Some(KokoroTTS::new_async().await?);
                    }
                    tts.as_mut()
                        .context("Kokoro TTS was not initialized")?
                        .speak(&cleaned_text)?;
                    println!("done\n");
                    set_terminal_title(mode.terminal_title());
                }
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

fn load_live_stt_engine(broker: &RuntimeBroker, mode: LiveSessionMode) -> Result<LocalSttEngine> {
    if mode.uses_focused_input()
        && LocalSttEngine::model_files_ready(PARAKEET_STT_KEY, Some(PARAKEET_STT_PATH))
    {
        #[cfg(feature = "sherpa-stt")]
        {
            println!("[stt] preloading Parakeet TDT 0.6B v3 INT8...");
            let started = Instant::now();
            let engine = LocalSttEngine::from_selection(PARAKEET_STT_KEY, Some(PARAKEET_STT_PATH))?;
            println!(
                "[stt] Parakeet ready in {:.1}s",
                started.elapsed().as_secs_f32()
            );
            return Ok(engine);
        }

        #[cfg(not(feature = "sherpa-stt"))]
        {
            println!(
                "[stt] Parakeet files are present, but this binary was built without --features sherpa-stt"
            );
        }
    }

    println!("[stt] preloading local broker-selected STT...");
    let started = Instant::now();
    let engine = LocalSttEngine::from_broker(broker)?;
    println!(
        "[stt] broker-selected STT ready in {:.1}s",
        started.elapsed().as_secs_f32()
    );
    Ok(engine)
}

struct PreparedRecording {
    samples: Vec<f32>,
    noise_floor: f32,
    gain: f32,
    final_rms: f32,
}

fn prepare_recording_for_stt(
    samples: &[f32],
    sample_rate: u32,
    mode: LiveSessionMode,
) -> PreparedRecording {
    let mut cleaned = remove_dc_offset(samples);
    let noise_floor = estimate_noise_floor(&cleaned, sample_rate);

    if mode.uses_focused_input() {
        let gate_threshold = (noise_floor * 2.0).clamp(0.000005, 0.003);
        for sample in &mut cleaned {
            if sample.abs() < gate_threshold {
                *sample *= 0.35;
            }
        }
        cleaned = trim_low_energy_edges(&cleaned, sample_rate, gate_threshold * 1.25);
    }

    let target_rms = if mode.uses_focused_input() {
        0.08_f32
    } else {
        0.1_f32
    };
    let max_gain = if mode.uses_focused_input() {
        50.0_f32
    } else {
        10.0_f32
    };
    let current_rms = rms_energy(&cleaned);
    let gain = if current_rms > 0.000001 {
        (target_rms / current_rms).clamp(1.0, max_gain)
    } else {
        1.0
    };
    if gain > 1.5 {
        println!("[process] boosting input by {:.1}x", gain);
    }

    let samples = cleaned
        .iter()
        .map(|sample| (*sample * gain).clamp(-1.0, 1.0))
        .collect::<Vec<_>>();
    let final_rms = rms_energy(&samples);

    PreparedRecording {
        samples,
        noise_floor,
        gain,
        final_rms,
    }
}

fn remove_dc_offset(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
    samples
        .iter()
        .map(|sample| (*sample - mean).clamp(-1.0, 1.0))
        .collect()
}

fn estimate_noise_floor(samples: &[f32], sample_rate: u32) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let window = ((sample_rate as usize) / 5).clamp(1, samples.len());
    let head = rms_energy(&samples[..window]);
    let tail = rms_energy(&samples[samples.len().saturating_sub(window)..]);
    head.min(tail)
}

fn trim_low_energy_edges(samples: &[f32], sample_rate: u32, threshold: f32) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let first = samples.iter().position(|sample| sample.abs() >= threshold);
    let last = samples.iter().rposition(|sample| sample.abs() >= threshold);
    let (Some(first), Some(last)) = (first, last) else {
        return samples.to_vec();
    };

    let keep = (sample_rate as usize / 4).max(1);
    let start = first.saturating_sub(keep);
    let end = (last + keep).min(samples.len().saturating_sub(1));
    if end <= start || end - start < sample_rate as usize / 4 {
        return samples.to_vec();
    }

    samples[start..=end].to_vec()
}

fn minimum_stt_samples(sample_rate: u32, mode: LiveSessionMode) -> usize {
    if mode.uses_focused_input() {
        (sample_rate as f32 * 0.75) as usize
    } else {
        (sample_rate as f32 * 0.35) as usize
    }
}

fn prepare_dictation_text(raw_text: &str) -> String {
    raw_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn rms_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let energy = samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32;
    energy.sqrt()
}

fn set_terminal_title(title: &str) {
    let sanitized = title.replace(['\x07', '\x1b'], "");
    print!("\x1b]0;{}\x07", sanitized);
    let _ = std::io::stdout().flush();
}

fn write_wav(path: &str, sample_rate: u32, samples: &[f32]) -> Result<()> {
    use hound::{SampleFormat, WavSpec, WavWriter};

    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)?;
    for sample in samples {
        let scaled = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(scaled)?;
    }
    writer.finalize()?;
    Ok(())
}

fn parse_modality(value: &str) -> Result<Modality> {
    match value.to_ascii_lowercase().as_str() {
        "chat" | "llm" => Ok(Modality::Chat),
        "text" => Ok(Modality::Text),
        "ui" | "uigen" | "ui-generation" | "ui_generation" | "frontend" => {
            Ok(Modality::UiGeneration)
        }
        "vlm" | "vision-language" | "vision_language" => Ok(Modality::VisionLanguage),
        "stt" | "speech" | "speech-to-text" | "speech_to_text" => Ok(Modality::SpeechToText),
        "tts" | "text-to-speech" | "text_to_speech" => Ok(Modality::TextToSpeech),
        "ocr" => Ok(Modality::Ocr),
        "image" | "image-generation" | "image_generation" => Ok(Modality::ImageGeneration),
        "video" | "video-generation" | "video_generation" => Ok(Modality::VideoGeneration),
        "wake" | "wakeword" | "wake-word" | "wake_words" => Ok(Modality::WakeWord),
        "grammar" => Ok(Modality::Grammar),
        other => Err(anyhow::anyhow!("Unsupported modality '{}'", other)),
    }
}

fn parse_host_surface(value: &str) -> Result<HostSurface> {
    match value.to_ascii_lowercase().as_str() {
        "dx" => Ok(HostSurface::Dx),
        "flow" | "flow-app" => Ok(HostSurface::FlowApp),
        "zeroclaw" => Ok(HostSurface::ZeroclawFork),
        "codex" => Ok(HostSurface::CodexFork),
        "zed" => Ok(HostSurface::ZedFork),
        "desktop" => Ok(HostSurface::Desktop),
        "android" => Ok(HostSurface::AndroidNative),
        "ios" => Ok(HostSurface::IosNative),
        "tauri" => Ok(HostSurface::Tauri),
        "flutter" => Ok(HostSurface::Flutter),
        "browser" | "wasm" | "browser-wasm" => Ok(HostSurface::BrowserWasm),
        "vps" => Ok(HostSurface::Vps),
        "raspberry-pi" | "raspberrypi" | "pi" => Ok(HostSurface::RaspberryPi),
        "watch" => Ok(HostSurface::Watch),
        "tv" => Ok(HostSurface::Tv),
        "tablet" => Ok(HostSurface::Tablet),
        "custom" => Ok(HostSurface::CustomRustHost),
        other => Err(anyhow::anyhow!("Unsupported host surface '{}'", other)),
    }
}

fn parse_browser_flavor(value: &str) -> Result<BrowserHostFlavor> {
    match value.to_ascii_lowercase().as_str() {
        "chromium" | "chrome" | "edge" => Ok(BrowserHostFlavor::ChromiumExtension),
        "firefox" | "gecko" => Ok(BrowserHostFlavor::FirefoxExtension),
        "safari" => Ok(BrowserHostFlavor::SafariWebExtension),
        "web" | "standalone" | "webapp" => Ok(BrowserHostFlavor::StandaloneWebApp),
        other => Err(anyhow::anyhow!("Unsupported browser flavor '{}'", other)),
    }
}

fn parse_browser_task(value: &str) -> Result<BrowserTask> {
    match value.to_ascii_lowercase().as_str() {
        "rewrite" | "rewrite-selection" | "rewrite_selection" => Ok(BrowserTask::RewriteSelection),
        "summarize-selection" | "summarize_selection" => Ok(BrowserTask::SummarizeSelection),
        "summarize-page" | "summarize_page" | "summarize" => Ok(BrowserTask::SummarizePage),
        "compose" | "compose-draft" | "compose_draft" => Ok(BrowserTask::ComposeDraft),
        "explain" | "explain-page" | "explain_page" => Ok(BrowserTask::ExplainPage),
        "ocr" | "ocr-image" | "ocr_image" => Ok(BrowserTask::OcrImage),
        "vlm" | "multimodal" | "multimodal-ask" | "multimodal_ask" => {
            Ok(BrowserTask::MultimodalAsk)
        }
        other => Err(anyhow::anyhow!("Unsupported browser task '{}'", other)),
    }
}

fn parse_integration_target(value: &str) -> Result<FlowIntegrationTarget> {
    match value.to_ascii_lowercase().as_str() {
        "dx" | "dx-desktop" | "desktop" => Ok(FlowIntegrationTarget::DxDesktop),
        "browser" | "browser-extension" | "webext" => Ok(FlowIntegrationTarget::BrowserExtension),
        "zed" | "zed-fork" => Ok(FlowIntegrationTarget::ZedFork),
        "codex" | "codex-fork" => Ok(FlowIntegrationTarget::CodexFork),
        "zeroclaw" | "zeroclaw-fork" | "openclaw" => Ok(FlowIntegrationTarget::ZeroClawFork),
        other => Err(anyhow::anyhow!(
            "Unsupported integration target '{}'",
            other
        )),
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let bytes = bytes as f64;
    if bytes >= GB {
        format!("{:.1} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{:.0} B", bytes)
    }
}

fn shortcut_label(modifiers: &[String], key: &str) -> String {
    if modifiers.is_empty() {
        key.to_string()
    } else {
        format!("{}+{}", modifiers.join("+"), key)
    }
}
