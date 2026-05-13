//! Metasearch CLI — entry point for the application.

// mimalloc: 2-6x faster than system allocator, critical on musl targets.
// Works on all platforms (Windows, Linux, macOS, Alpine/musl).
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::{path::Path, sync::Arc};

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use metasearch_core::config::Settings;
use metasearch_engine::EngineRegistry;
use metasearch_server::{
    app,
    cache::SearchCache,
    health::EngineHealthTracker,
    orchestrator::SearchOrchestrator,
    state::AppState,
    templates::Templates,
};

#[derive(Parser)]
#[command(name = "metasearch")]
#[command(about = "A blazing-fast, privacy-respecting metasearch engine")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Host to bind to
    #[arg(long)]
    host: Option<String>,

    /// Port to listen on
    #[arg(short, long)]
    port: Option<u16>,

    /// Path to templates directory
    #[arg(long)]
    templates: Option<String>,

    /// Path to static assets directory
    #[arg(long)]
    static_dir: Option<String>,

    /// Optional path to a TOML configuration file
    #[arg(long)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the web server (default)
    Serve,
    /// List all registered engines
    Engines,
    /// Print the effective configuration
    Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("metasearch=info".parse()?))
        .init();

    let cli = Cli::parse();

    // Build settings, preferring an explicit config path or the root config file.
    let mut settings = match cli.config.as_deref() {
        Some(path) => load_settings_from_path(path)?,
        None if Path::new("config.toml").exists() => load_settings_from_path("config.toml")?,
        None => Settings::default(),
    };
    let original_host = settings.server.host.clone();
    let original_port = settings.server.port;
    let original_base_url = settings.server.base_url.clone();

    if let Some(host) = cli.host {
        settings.server.host = host;
    }
    if let Some(port) = cli.port {
        settings.server.port = port;
    }
    if let Some(templates_dir) = cli.templates.as_ref() {
        settings.server.templates_dir = templates_dir.clone();
    }
    if let Some(static_dir) = cli.static_dir.as_ref() {
        settings.server.static_dir = static_dir.clone();
    }

    let original_base_host = if original_host == "0.0.0.0" {
        "localhost"
    } else {
        original_host.as_str()
    };
    let original_derived_base_url = format!("http://{}:{}", original_base_host, original_port);
    let current_base_host = if settings.server.host == "0.0.0.0" {
        "localhost"
    } else {
        settings.server.host.as_str()
    };

    if original_base_url.trim().is_empty()
        || original_base_url == "http://localhost:8888"
        || original_base_url == original_derived_base_url
    {
        settings.server.base_url = format!("http://{}:{}", current_base_host, settings.server.port);
    }

    // Build optimized HTTP client with connection pooling
    let http_client = reqwest::Client::builder()
        .user_agent("Metasearch/0.1 (https://github.com/najmus-sakib-hossain/metasearch)")
        .timeout(std::time::Duration::from_secs(10))
        .connect_timeout(std::time::Duration::from_secs(3))
        .pool_max_idle_per_host(50)  // More connections per host
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .tcp_nodelay(true)  // Disable Nagle's algorithm for lower latency
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .http2_adaptive_window(true)  // Better HTTP/2 performance
        .http2_keep_alive_interval(std::time::Duration::from_secs(30))
        .http2_keep_alive_timeout(std::time::Duration::from_secs(10))
        .http2_keep_alive_while_idle(true)
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()?;

    // Register all built-in engines using with_defaults()
    // Clone the client so we keep one for the registry and one for general use (autocomplete, etc.)
    let shared_client = http_client.clone();
    let registry = EngineRegistry::with_defaults(http_client);
    let engine_count = registry.count();
    let registry = Arc::new(registry);

    validate_asset_dir(&settings.server.templates_dir, "template")?;
    validate_asset_dir(&settings.server.static_dir, "static")?;

    // Load templates
    let templates = Templates::new(&settings.server.templates_dir)?;

    // Build the performance stack
    let cache = SearchCache::new(settings.cache.max_entries, settings.cache.ttl_secs);
    let health = Arc::new(EngineHealthTracker::new());
    let max_engines = settings.search.max_concurrent_engines;
    let orchestrator = Arc::new(SearchOrchestrator::new(
        Arc::clone(&registry),
        cache.clone(),
        Arc::clone(&health),
        max_engines,
    ));

    let state = Arc::new(AppState {
        cache,
        engine_registry: registry,
        template_dir: settings.server.templates_dir.clone(),
        static_dir: settings.server.static_dir.clone(),
        templates: Arc::new(templates),
        orchestrator,
        health,
        settings,
        http_client: shared_client,
    });

    match cli.command.unwrap_or(Commands::Serve) {
        Commands::Serve => {
            tracing::info!("Registered {} search engines", engine_count);
            app::run(state).await?;
        }
        Commands::Engines => {
            println!("Registered engines ({}):", engine_count);
            let mut names = state.engine_registry.engine_names();
            names.sort();
            for name in names {
                println!("  - {}", name);
            }
        }
        Commands::Config => {
            println!("{}", serde_json::to_string_pretty(&state.settings)?);
        }
    }

    Ok(())
}

fn load_settings_from_path(path: &str) -> anyhow::Result<Settings> {
    let contents = std::fs::read_to_string(path)?;
    let settings = toml::from_str::<Settings>(&contents)?;
    Ok(settings)
}

fn validate_asset_dir(path: &str, label: &str) -> anyhow::Result<()> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| anyhow::anyhow!("{} directory `{}` is not accessible: {}", label, path, error))?;
    if !metadata.is_dir() {
        anyhow::bail!("{} directory `{}` is not a directory", label, path);
    }
    Ok(())
}
