/// Command-line arguments
#[derive(Debug)]
pub struct Args {
    pub command: Command,
}

#[derive(Debug)]
pub enum Command {
    /// Transcribe audio file
    Transcribe { file: String },
    /// Full Wispr Flow pipeline (STT + LLM enhancement)
    Wispr { file: String },
    /// Speak text using TTS
    Speak { text: String },
    /// Live recording mode (microphone -> STT -> enhance -> TTS)
    Live,
    /// Live dictation mode (wake/hotkey -> STT -> focused input)
    Dictate,
    /// Interactive mode
    Interactive,
    /// Chat with AI (interactive CLI chat)
    Chat { model: Option<String> },
    /// Run one bounded local tool-agent prompt
    ToolAgent {
        tools: Option<String>,
        request: String,
    },
    /// OCR - Extract text from image
    Ocr {
        image: String,
        prompt: Option<String>,
    },
    /// Show the detected device profile and activation config
    Profile,
    /// Show the DX project stack and current completeness scores
    Projects,
    /// Show the Flow competitive scorecard
    Scorecard,
    /// List the broker catalog, optionally filtered by modality
    Models { modality: Option<String> },
    /// Download a known local model artifact
    InstallModel { model: String },
    /// Show UI model candidates before downloading more models
    UiModelCandidates,
    /// Show ranked local tool-calling model candidates
    ToolModelCandidates,
    /// Show the Flow local model role policy
    ModelRoles,
    /// Generate a single-file UI artifact with the default local UI model
    Uigen {
        model: Option<String>,
        output: String,
        prompt: String,
    },
    /// Generate the standard Google homepage clone evaluation artifact
    UigenGoogle { model: Option<String> },
    /// Generate a UI from a screenshot with the local vision UI model
    UigenVision {
        screenshot: String,
        output: String,
        prompt: String,
    },
    /// Generate the standard Google homepage clone evaluation artifact from a screenshot
    UigenVisionGoogle,
    /// Build and print a broker execution plan
    Plan {
        modality: String,
        model: Option<String>,
    },
    /// Build and print a host embedding blueprint
    Blueprint { host: String },
    /// Show detected browser capability defaults for a browser flavor
    BrowserProfile { flavor: String },
    /// Build and print a browser execution plan
    BrowserPlan {
        flavor: String,
        task: String,
        modality: String,
        model: Option<String>,
        remote_fallback: bool,
    },
    /// Show registered browser-ready packs
    BrowserPacks,
    /// Inspect or correct text with the local grammar engine
    Grammar { text: String, fix: bool },
    /// Show local wake-word configuration
    WakeWords,
    /// Print the recommended production config for a host target
    ProductionConfig { target: String },
    /// Export all production configs and a delivery manifest into a directory
    ExportProductionBundle { output_dir: String },
    /// Print a release summary for the current repository scope
    ReleaseSummary,
    /// Export release-summary handoff files into a directory
    ExportReleaseSummary { output_dir: String },
}

impl Args {
    /// Parse command-line arguments
    pub fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();

        if args.len() < 2 {
            return Self {
                command: Command::Interactive,
            };
        }

        let command = match args[1].as_str() {
            "--transcribe" | "-t" => {
                let file = args
                    .get(2)
                    .cloned()
                    .unwrap_or_else(|| "tests/fixtures/audio.mp3".to_string());
                Command::Transcribe { file }
            }
            "--wispr" | "-w" => {
                let file = args
                    .get(2)
                    .cloned()
                    .unwrap_or_else(|| "tests/fixtures/audio.mp3".to_string());
                Command::Wispr { file }
            }
            "--speak" | "-s" => {
                let text = args[2..].join(" ");
                Command::Speak { text }
            }
            "--live" | "-l" => Command::Live,
            "--dictate" | "--live-type" | "--type" => Command::Dictate,
            "--chat" | "-c" => {
                let model = args.get(2).cloned();
                Command::Chat { model }
            }
            "--tool-agent" => {
                if args.len() <= 2 {
                    eprintln!("Error: prompt required");
                    eprintln!("Usage: flow --tool-agent <prompt>");
                    std::process::exit(1);
                }
                Command::ToolAgent {
                    tools: None,
                    request: args[2..].join(" "),
                }
            }
            "--tool-agent-tools" => {
                let tools = args.get(2).cloned().unwrap_or_else(|| {
                    eprintln!("Error: tools JSON path required");
                    eprintln!("Usage: flow --tool-agent-tools <tools.json> <request>");
                    std::process::exit(1);
                });
                if args.len() <= 3 {
                    eprintln!("Error: request required");
                    eprintln!("Usage: flow --tool-agent-tools <tools.json> <request>");
                    std::process::exit(1);
                }
                Command::ToolAgent {
                    tools: Some(tools),
                    request: args[3..].join(" "),
                }
            }
            "--ocr" | "-o" => {
                let image = args.get(2).cloned().unwrap_or_else(|| {
                    eprintln!("Error: Image path required for OCR");
                    eprintln!("Usage: flow --ocr <image_path> [prompt]");
                    std::process::exit(1);
                });
                let prompt = if args.len() > 3 {
                    Some(args[3..].join(" "))
                } else {
                    None
                };
                Command::Ocr { image, prompt }
            }
            "--profile" => Command::Profile,
            "--projects" => Command::Projects,
            "--scorecard" => Command::Scorecard,
            "--models" => {
                let modality = args.get(2).cloned();
                Command::Models { modality }
            }
            "--install-model" => {
                let model = args.get(2).cloned().unwrap_or_else(|| {
                    eprintln!("Error: model key required");
                    eprintln!("Usage: flow --install-model webgen-4b-preview-i1-q4km");
                    std::process::exit(1);
                });
                Command::InstallModel { model }
            }
            "--ui-model-candidates" => Command::UiModelCandidates,
            "--tool-model-candidates" | "--agent-model-candidates" => Command::ToolModelCandidates,
            "--model-roles" | "--local-model-roles" => Command::ModelRoles,
            "--uigen" => {
                let output = args.get(2).cloned().unwrap_or_else(|| {
                    eprintln!("Error: output path required");
                    eprintln!("Usage: flow --uigen <output.html> <prompt>");
                    std::process::exit(1);
                });
                if args.len() <= 3 {
                    eprintln!("Error: prompt required");
                    eprintln!("Usage: flow --uigen <output.html> <prompt>");
                    std::process::exit(1);
                }
                let prompt = args[3..].join(" ");
                Command::Uigen {
                    model: None,
                    output,
                    prompt,
                }
            }
            "--uigen-model" => {
                let model = args.get(2).cloned().unwrap_or_else(|| {
                    eprintln!("Error: model key required");
                    eprintln!("Usage: flow --uigen-model <model> <output.html> <prompt>");
                    std::process::exit(1);
                });
                let output = args.get(3).cloned().unwrap_or_else(|| {
                    eprintln!("Error: output path required");
                    eprintln!("Usage: flow --uigen-model <model> <output.html> <prompt>");
                    std::process::exit(1);
                });
                if args.len() <= 4 {
                    eprintln!("Error: prompt required");
                    eprintln!("Usage: flow --uigen-model <model> <output.html> <prompt>");
                    std::process::exit(1);
                }
                let prompt = args[4..].join(" ");
                Command::Uigen {
                    model: Some(model),
                    output,
                    prompt,
                }
            }
            "--uigen-google" => {
                let model = args.get(2).cloned();
                Command::UigenGoogle { model }
            }
            "--uigen-vision" => {
                let screenshot = args.get(2).cloned().unwrap_or_else(|| {
                    eprintln!("Error: screenshot path required");
                    eprintln!("Usage: flow --uigen-vision <screenshot.png> <output.html> <prompt>");
                    std::process::exit(1);
                });
                let output = args.get(3).cloned().unwrap_or_else(|| {
                    eprintln!("Error: output path required");
                    eprintln!("Usage: flow --uigen-vision <screenshot.png> <output.html> <prompt>");
                    std::process::exit(1);
                });
                if args.len() <= 4 {
                    eprintln!("Error: prompt required");
                    eprintln!("Usage: flow --uigen-vision <screenshot.png> <output.html> <prompt>");
                    std::process::exit(1);
                }
                let prompt = args[4..].join(" ");
                Command::UigenVision {
                    screenshot,
                    output,
                    prompt,
                }
            }
            "--uigen-vision-google" => Command::UigenVisionGoogle,
            "--plan" => {
                let modality = args.get(2).cloned().unwrap_or_else(|| "chat".to_string());
                let model = args.get(3).cloned();
                Command::Plan { modality, model }
            }
            "--blueprint" => {
                let host = args.get(2).cloned().unwrap_or_else(|| "dx".to_string());
                Command::Blueprint { host }
            }
            "--browser-profile" => {
                let flavor = args
                    .get(2)
                    .cloned()
                    .unwrap_or_else(|| "chromium".to_string());
                Command::BrowserProfile { flavor }
            }
            "--browser-plan" => {
                let flavor = args
                    .get(2)
                    .cloned()
                    .unwrap_or_else(|| "chromium".to_string());
                let task = args
                    .get(3)
                    .cloned()
                    .unwrap_or_else(|| "rewrite-selection".to_string());
                let modality = args.get(4).cloned().unwrap_or_else(|| "chat".to_string());
                let mut remote_fallback = false;
                let mut model = None;
                for arg in args.iter().skip(5) {
                    if matches!(arg.as_str(), "--remote" | "--allow-remote") {
                        remote_fallback = true;
                    } else if model.is_none() {
                        model = Some(arg.clone());
                    }
                }
                Command::BrowserPlan {
                    flavor,
                    task,
                    modality,
                    model,
                    remote_fallback,
                }
            }
            "--browser-packs" => Command::BrowserPacks,
            "--grammar" => {
                let mut index = 2;
                let mut fix = false;
                if matches!(args.get(index).map(String::as_str), Some("--fix" | "-f")) {
                    fix = true;
                    index += 1;
                }

                if args.len() <= index {
                    eprintln!("Error: Text is required for grammar analysis");
                    eprintln!("Usage: flow --grammar [--fix] <text>");
                    std::process::exit(1);
                }

                let text = args[index..].join(" ");
                Command::Grammar { text, fix }
            }
            "--wakewords" | "--wake-words" => Command::WakeWords,
            "--production-config" => {
                let target = args
                    .get(2)
                    .cloned()
                    .unwrap_or_else(|| "dx-desktop".to_string());
                Command::ProductionConfig { target }
            }
            "--export-production-bundle" => {
                let output_dir = args
                    .get(2)
                    .cloned()
                    .unwrap_or_else(|| "configs/production".to_string());
                Command::ExportProductionBundle { output_dir }
            }
            "--release-summary" => Command::ReleaseSummary,
            "--export-release-summary" => {
                let output_dir = args
                    .get(2)
                    .cloned()
                    .unwrap_or_else(|| "release".to_string());
                Command::ExportReleaseSummary { output_dir }
            }
            _ => Command::Interactive,
        };

        Self { command }
    }
}
