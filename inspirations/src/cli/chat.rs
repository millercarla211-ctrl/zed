use std::io::{self, BufRead, Write};
use std::path::Path;

use anyhow::{Context, Result};

use crate::models::llm::LocalLlm;
use crate::runtime::{BrokerRequest, Modality, RuntimeBroker};

pub async fn run_chat(model_key: Option<String>) -> Result<()> {
    let broker = RuntimeBroker::detect();
    let chat_models = broker.models_for(Modality::Chat);

    println!("Flow AI Chat");
    println!("============\n");
    println!(
        "Detected device tier: {:?} ({:.1} GB available)\n",
        broker.device_profile().tier,
        broker.device_profile().available_memory_bytes as f64 / 1024.0 / 1024.0 / 1024.0
    );

    println!("Broker catalog for chat:");
    for (index, manifest) in chat_models.iter().enumerate() {
        let local_state = manifest
            .local_path
            .as_deref()
            .map(Path::new)
            .map(Path::exists)
            .unwrap_or(false);
        let filename = manifest
            .local_path
            .as_deref()
            .and_then(|path| Path::new(path).file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("-");
        println!(
            "  {}. {} [{}] - {} ({})",
            index + 1,
            manifest.key,
            if local_state { "local" } else { "missing" },
            manifest.display_name,
            filename
        );
    }
    println!();

    let mut request = BrokerRequest::new(Modality::Chat).with_model(model_key.clone());
    request.allow_conversion = false;
    request.allow_publish = false;

    let plan = broker.build_plan(request);
    let selected_key = plan
        .selected_model
        .clone()
        .ok_or_else(|| anyhow::anyhow!("No chat model is available for this device"))?;
    let manifest = broker
        .catalog()
        .iter()
        .find(|item| item.key == selected_key)
        .context("Selected model is not in the broker catalog")?;
    let model_path = manifest
        .local_path
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Selected model has no local path"))?;

    if !Path::new(&model_path).exists() {
        return Err(anyhow::anyhow!(
            "Selected model '{}' is not present locally at {}",
            manifest.key,
            model_path
        ));
    }

    println!("Selected model: {}", manifest.display_name);
    println!(
        "Runtime plan: {:?} via {:?}\n",
        plan.launch, plan.selected_runtime
    );

    let llm = LocalLlm::with_model_path(model_path.clone());

    match llm.initialize().await {
        Ok(_) => {
            println!("Model loaded successfully.\n");
        }
        Err(error) if manifest.key.contains("gemma") => {
            println!("Primary model failed to load: {}", error);
            let fallback = broker
                .catalog()
                .iter()
                .find(|candidate| {
                    candidate.modality == Modality::Chat
                        && candidate.key != manifest.key
                        && candidate
                            .local_path
                            .as_deref()
                            .map(Path::new)
                            .map(Path::exists)
                            .unwrap_or(false)
                })
                .context("No fallback chat model is available locally")?;

            let fallback_path = fallback
                .local_path
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Fallback model has no local path"))?;
            let fallback_llm = LocalLlm::with_model_path(fallback_path);
            fallback_llm.initialize().await?;
            println!("Fell back to {}.\n", fallback.display_name);
            return run_chat_session(fallback_llm).await;
        }
        Err(error) => return Err(error),
    }

    run_chat_session(llm).await
}

async fn run_chat_session(llm: LocalLlm) -> Result<()> {
    println!("Type your message and press Enter. Type 'exit' or 'quit' to end the chat.\n");
    println!("Commands:");
    println!("  /clear  - Clear conversation history");
    println!("  /help   - Show this help message");
    println!("  /exit   - Exit chat\n");

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = io::stdout();

    loop {
        print!("You: ");
        stdout.flush()?;

        let mut input = String::new();
        reader.read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        match input {
            "exit" | "quit" | "/exit" | "/quit" => {
                println!("\nGoodbye.");
                break;
            }
            "/clear" => {
                llm.clear_history()?;
                println!("\nConversation history cleared.\n");
                continue;
            }
            "/help" => {
                println!("\nCommands:");
                println!("  /clear  - Clear conversation history");
                println!("  /help   - Show this help message");
                println!("  /exit   - Exit chat\n");
                continue;
            }
            _ => {}
        }

        print!("AI: ");
        stdout.flush()?;

        match llm.generate_with_metrics(input).await {
            Ok((response, metrics)) => {
                println!("{}", response);
                println!(
                    "\n[{} tokens in {:.2}s @ {:.1} tok/s]\n",
                    metrics.generated_tokens,
                    metrics.total_time_ms as f64 / 1000.0,
                    metrics.tokens_per_second
                );
            }
            Err(error) => {
                println!("\nError: {}\n", error);
            }
        }
    }

    Ok(())
}
