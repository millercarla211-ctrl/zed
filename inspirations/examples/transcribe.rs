use anyhow::Result;
use flow::{LocalSttEngine, RuntimeBroker};
use std::path::Path;

fn main() -> Result<()> {
    println!("Flow - Audio Transcription Example");

    let audio_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/fixtures/audio.wav".to_string());

    if !Path::new(&audio_path).exists() {
        println!("Audio file not found: {}", audio_path);
        println!("Pass a WAV file path as the first argument.");
        return Ok(());
    }

    let broker = RuntimeBroker::detect();
    let mut stt = match LocalSttEngine::from_broker(&broker) {
        Ok(engine) => engine,
        Err(error) => {
            println!("Local STT models are not installed or ready: {error}");
            println!("Provide a WAV file after installing models to run this example.");
            return Ok(());
        }
    };
    let text = stt.transcribe(&audio_path)?;

    println!("Transcription: {}", text);

    Ok(())
}
