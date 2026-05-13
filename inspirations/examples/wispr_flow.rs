use anyhow::Result;
use flow::{LocalLlm, LocalSttEngine, RuntimeBroker};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Flow - Wispr-style Pipeline Example");

    let audio_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/fixtures/audio.wav".to_string());

    let broker = RuntimeBroker::detect();
    let raw_text = if Path::new(&audio_path).exists() {
        match LocalSttEngine::from_broker(&broker) {
            Ok(mut stt) => stt.transcribe(&audio_path)?,
            Err(error) => {
                println!("Using a mock transcript because local STT is not ready: {error}");
                "um i think we should maybe ship the browser extension first".to_string()
            }
        }
    } else {
        println!("Using a mock transcript because the WAV input is not available.");
        "um i think we should maybe ship the browser extension first".to_string()
    };
    println!("Raw: {}", raw_text);

    let llm = LocalLlm::new();
    llm.initialize().await?;

    let prompt = format!(
        "Clean up this transcription. Remove filler words and add punctuation:\n\n{}",
        raw_text
    );
    let (enhanced, metrics) = llm.generate_with_metrics(&prompt).await?;

    println!("Enhanced: {}", enhanced);
    println!(
        "Metrics: {} prompt tokens, {} generated tokens, {:.2} tok/s",
        metrics.prompt_tokens, metrics.generated_tokens, metrics.tokens_per_second
    );

    Ok(())
}
