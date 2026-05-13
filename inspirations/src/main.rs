use anyhow::Result;
use flow::{Args, execute};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    execute(args.command).await
}
