use anyhow::Result;
use flow::RuntimeBroker;
use flow::models::{KokoroTTS, LocalLlm, LocalSttEngine};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Flow - Complete Wispr-style Pipeline");
    println!("STT (Moonshine) -> LLM (LocalLlm) -> TTS (Kokoro)");

    let audio_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/fixtures/audio.wav".to_string());

    println!("\nStep 1: Speech-to-text");
    let broker = RuntimeBroker::detect();
    let raw_transcript = if Path::new(&audio_path).exists() {
        match LocalSttEngine::from_broker(&broker) {
            Ok(mut stt) => stt.transcribe(&audio_path)?,
            Err(error) => {
                println!("Local STT is not ready ({error}), using a mock transcript.");
                "hello mike testing one two three hello".to_string()
            }
        }
    } else {
        println!("WAV input is not available, using a mock transcript.");
        "hello mike testing one two three hello".to_string()
    };
    println!("Raw transcript: {}", raw_transcript);

    println!("\nStep 2: Text enhancement");
    let llm = LocalLlm::new();
    let enhanced_text = if Path::new(llm.model_path()).exists() {
        llm.initialize().await?;
        let prompt = format!(
            concat!(
                "You are a speech cleanup engine.\n",
                "1. Remove filler words.\n",
                "2. Add punctuation and capitalization.\n",
                "3. Keep the meaning identical.\n",
                "4. Return only the cleaned text.\n\n",
                "Raw transcript:\n{}"
            ),
            raw_transcript
        );
        let response: String = llm.generate(&prompt).await?;
        first_non_empty_line(&response)
            .unwrap_or(response.trim())
            .to_string()
    } else {
        println!("Local LLM model is not installed, using a rule-based fallback.");
        enhance_text_basic(&raw_transcript)
    };
    println!("Enhanced text: {}", enhanced_text);

    println!("\nStep 3: Text-to-speech");
    if KokoroTTS::is_available() {
        let mut tts: KokoroTTS = KokoroTTS::new_async().await?;
        let audio = tts.synthesize(&enhanced_text)?;
        let output_path = "output_wispr_flow.wav";
        tts.save_wav(&audio, output_path)?;
        println!("Audio saved to: {}", output_path);
    } else {
        println!("Kokoro TTS models are not installed, skipping audio generation.");
    }

    println!("\nPipeline complete.");
    println!("Input audio: {}", audio_path);
    println!("Transcript: {}", raw_transcript);
    println!("Enhanced: {}", enhanced_text);

    Ok(())
}

fn first_non_empty_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

fn enhance_text_basic(text: &str) -> String {
    let fillers = ["um", "uh", "like", "you know", "sort of", "kind of"];
    let mut enhanced = text.to_lowercase();

    for filler in &fillers {
        enhanced = enhanced.replace(&format!(" {} ", filler), " ");
    }

    enhanced = enhanced.split_whitespace().collect::<Vec<_>>().join(" ");

    if !enhanced.is_empty() && !enhanced.ends_with('.') {
        enhanced.push('.');
    }

    if let Some(first_char) = enhanced.chars().next() {
        enhanced = first_char.to_uppercase().collect::<String>() + &enhanced[1..];
    }

    enhanced
}
