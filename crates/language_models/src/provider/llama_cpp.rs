use anyhow::{Context as _, Result, anyhow};
use futures::{
    AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, FutureExt, StreamExt, future::BoxFuture,
    io::BufReader, stream::BoxStream,
};
use gpui::{AnyView, App, AsyncApp, Context, Entity, Subscription, Task, Window};
use http_client::{AsyncBody, HttpClient, Method, Request as HttpRequest, StatusCode};
use language_model::{
    AuthenticateError, IconOrSvg, LanguageModel, LanguageModelCompletionError,
    LanguageModelCompletionEvent, LanguageModelId, LanguageModelName, LanguageModelProvider,
    LanguageModelProviderId, LanguageModelProviderName, LanguageModelProviderState,
    LanguageModelRequest, LanguageModelToolChoice, LanguageModelToolSchemaFormat, MessageContent,
    RateLimiter, Role, StopReason, TokenUsage,
};
use serde::{Deserialize, Serialize};
use settings::{Settings, SettingsStore};
use std::{
    collections::BTreeMap,
    env,
    fs::File,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};
use ui::{ButtonLike, ConfiguredApiCard, List, ListBulletItem, prelude::*};
use util::ResultExt;

use crate::AllLanguageModelSettings;

pub use settings::LlamaCppAvailableModel as AvailableModel;

const PROVIDER_ID: LanguageModelProviderId = LanguageModelProviderId::new("llama_cpp");
const PROVIDER_NAME: LanguageModelProviderName = LanguageModelProviderName::new("Llama.cpp");

const LOW_SPEC_MEMORY_THRESHOLD_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const LOCAL_LLAMA_SERVER_PARALLEL_SLOTS: u64 = 1;
const LOCAL_LLAMA_PROMPT_TOKEN_MARGIN: u64 = 256;

#[derive(Clone, Debug, PartialEq)]
pub struct LlamaCppSettings {
    pub api_url: String,
    pub host: String,
    pub port: u16,
    pub server_path: Option<String>,
    pub models_dir: Option<String>,
    pub cache_dir: Option<String>,
    pub tools_dir: Option<String>,
    pub auto_start: bool,
    pub auto_download_models: bool,
    pub auto_install_server: bool,
    pub auto_discover_local_models: bool,
    pub context_window: u64,
    pub threads: Option<u64>,
    pub gpu_layers: Option<i64>,
    pub normal_model: AvailableModel,
    pub low_spec_model: AvailableModel,
    pub available_models: Vec<AvailableModel>,
}

pub struct LlamaCppLanguageModelProvider {
    http_client: Arc<dyn HttpClient>,
    state: Entity<State>,
}

pub struct State {
    settings: LlamaCppSettings,
    use_low_spec_model: bool,
    discovered_models: Vec<AvailableModel>,
    runtime: Arc<LlamaCppRuntime>,
    _subscription: Subscription,
}

impl State {
    fn selected_model(&self) -> AvailableModel {
        if self.use_low_spec_model {
            self.settings.low_spec_model.clone()
        } else {
            self.settings.normal_model.clone()
        }
    }

    fn all_models(&self) -> Vec<AvailableModel> {
        let mut models = BTreeMap::default();
        models.insert(self.selected_model().name.clone(), self.selected_model());
        models.insert(
            self.settings.normal_model.name.clone(),
            self.settings.normal_model.clone(),
        );
        models.insert(
            self.settings.low_spec_model.name.clone(),
            self.settings.low_spec_model.clone(),
        );
        for model in &self.settings.available_models {
            models.insert(model.name.clone(), model.clone());
        }
        for model in &self.discovered_models {
            models.entry(model.name.clone()).or_insert(model.clone());
        }

        models.into_values().collect()
    }

    fn discover_models_from_settings(settings: &LlamaCppSettings) -> Vec<AvailableModel> {
        if !settings.auto_discover_local_models {
            return Vec::new();
        }

        let models_dir = settings
            .models_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(default_models_dir);
        let known_local_paths = std::iter::once(&settings.normal_model)
            .chain(std::iter::once(&settings.low_spec_model))
            .chain(settings.available_models.iter())
            .filter_map(|model| model.local_path.as_ref().map(PathBuf::from))
            .collect::<Vec<_>>();

        discover_local_gguf_models(&models_dir, &known_local_paths, settings.context_window)
    }

    fn launch_config(&self, model: &AvailableModel) -> LlamaCppLaunchConfig {
        LlamaCppLaunchConfig {
            api_url: self.settings.api_url.clone(),
            host: self.settings.host.clone(),
            port: self.settings.port,
            server_path: self.settings.server_path.as_ref().map(PathBuf::from),
            models_dir: self
                .settings
                .models_dir
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(default_models_dir),
            cache_dir: self
                .settings
                .cache_dir
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(default_cache_dir),
            tools_dir: self
                .settings
                .tools_dir
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(default_tools_dir),
            auto_start: self.settings.auto_start,
            auto_download_models: self.settings.auto_download_models,
            auto_install_server: self.settings.auto_install_server,
            context_window: model.max_tokens,
            threads: self.settings.threads,
            gpu_layers: self.settings.gpu_layers,
            model: model.clone(),
        }
    }
}

impl LlamaCppLanguageModelProvider {
    pub fn new(http_client: Arc<dyn HttpClient>, cx: &mut App) -> Self {
        let runtime = Arc::new(LlamaCppRuntime::default());
        let state = cx.new(|cx| {
            let subscription = cx.observe_global::<SettingsStore>({
                let mut settings = AllLanguageModelSettings::get_global(cx).llama_cpp.clone();
                move |this: &mut State, cx| {
                    let new_settings = AllLanguageModelSettings::get_global(cx).llama_cpp.clone();
                    if settings != new_settings {
                        settings = new_settings.clone();
                        this.discovered_models =
                            State::discover_models_from_settings(&new_settings);
                        this.settings = new_settings;
                        cx.notify();
                    }
                }
            });
            let settings = AllLanguageModelSettings::get_global(cx).llama_cpp.clone();
            let discovered_models = State::discover_models_from_settings(&settings);

            State {
                settings,
                use_low_spec_model: is_low_spec_system(),
                discovered_models,
                runtime,
                _subscription: subscription,
            }
        });

        Self { http_client, state }
    }

    fn create_language_model(&self, model: AvailableModel) -> Arc<dyn LanguageModel> {
        Arc::new(LlamaCppLanguageModel {
            id: LanguageModelId::from(model.name.clone()),
            model,
            state: self.state.clone(),
            http_client: self.http_client.clone(),
            request_limiter: RateLimiter::new(2),
        })
    }
}

impl LanguageModelProviderState for LlamaCppLanguageModelProvider {
    type ObservableEntity = State;

    fn observable_entity(&self) -> Option<Entity<Self::ObservableEntity>> {
        Some(self.state.clone())
    }
}

impl LanguageModelProvider for LlamaCppLanguageModelProvider {
    fn id(&self) -> LanguageModelProviderId {
        PROVIDER_ID
    }

    fn name(&self) -> LanguageModelProviderName {
        PROVIDER_NAME
    }

    fn icon(&self) -> IconOrSvg {
        IconOrSvg::Icon(IconName::AiOllama)
    }

    fn default_model(&self, cx: &App) -> Option<Arc<dyn LanguageModel>> {
        Some(self.create_language_model(self.state.read(cx).selected_model()))
    }

    fn default_fast_model(&self, cx: &App) -> Option<Arc<dyn LanguageModel>> {
        Some(self.create_language_model(self.state.read(cx).settings.low_spec_model.clone()))
    }

    fn provided_models(&self, cx: &App) -> Vec<Arc<dyn LanguageModel>> {
        self.state
            .read(cx)
            .all_models()
            .into_iter()
            .map(|model| self.create_language_model(model))
            .collect()
    }

    fn recommended_models(&self, cx: &App) -> Vec<Arc<dyn LanguageModel>> {
        vec![self.create_language_model(self.state.read(cx).selected_model())]
    }

    fn is_authenticated(&self, _cx: &App) -> bool {
        true
    }

    fn authenticate(&self, _cx: &mut App) -> Task<Result<(), AuthenticateError>> {
        // Local models do not have credentials. Avoid loading a model during
        // background provider authentication because the selected agent model
        // may be different from the provider's fallback/default model.
        Task::ready(Ok(()))
    }

    fn configuration_view(
        &self,
        _target_agent: language_model::ConfigurationViewTargetAgent,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyView {
        cx.new(|cx| ConfigurationView::new(self.state.clone(), self.http_client.clone(), cx))
            .into()
    }

    fn reset_credentials(&self, _cx: &mut App) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }
}

pub struct LlamaCppLanguageModel {
    id: LanguageModelId,
    model: AvailableModel,
    state: Entity<State>,
    http_client: Arc<dyn HttpClient>,
    request_limiter: RateLimiter,
}

impl LlamaCppLanguageModel {
    fn stream_completion(
        &self,
        request: LanguageModelRequest,
        cx: &AsyncApp,
    ) -> BoxFuture<
        'static,
        Result<
            BoxStream<'static, Result<LanguageModelCompletionEvent, LanguageModelCompletionError>>,
            LanguageModelCompletionError,
        >,
    > {
        let (runtime, config) = self.state.read_with(cx, |state, _| {
            (state.runtime.clone(), state.launch_config(&self.model))
        });
        let http_client = self.http_client.clone();
        let future = self.request_limiter.stream(async move {
            log::info!(
                "starting llama.cpp completion with model {}",
                config.model.name
            );
            runtime
                .ensure_server(config.clone(), http_client.clone())
                .await
                .map_err(LanguageModelCompletionError::Other)?;
            let config = effective_launch_config(http_client.as_ref(), config).await;
            stream_llama_completion(http_client.as_ref(), &config, request).await
        });

        async move { Ok(future.await?.boxed()) }.boxed()
    }
}

impl LanguageModel for LlamaCppLanguageModel {
    fn id(&self) -> LanguageModelId {
        self.id.clone()
    }

    fn name(&self) -> LanguageModelName {
        LanguageModelName::from(
            self.model
                .display_name
                .clone()
                .unwrap_or_else(|| self.model.name.clone()),
        )
    }

    fn provider_id(&self) -> LanguageModelProviderId {
        PROVIDER_ID
    }

    fn provider_name(&self) -> LanguageModelProviderName {
        PROVIDER_NAME
    }

    fn supports_tools(&self) -> bool {
        false
    }

    fn tool_input_format(&self) -> LanguageModelToolSchemaFormat {
        LanguageModelToolSchemaFormat::JsonSchemaSubset
    }

    fn supports_tool_choice(&self, choice: LanguageModelToolChoice) -> bool {
        match choice {
            LanguageModelToolChoice::Auto | LanguageModelToolChoice::Any => false,
            LanguageModelToolChoice::None => true,
        }
    }

    fn supports_streaming_tools(&self) -> bool {
        false
    }

    fn supports_images(&self) -> bool {
        self.model.supports_images
    }

    fn supports_split_token_display(&self) -> bool {
        true
    }

    fn telemetry_id(&self) -> String {
        format!("llama_cpp/{}", self.model.name)
    }

    fn max_token_count(&self) -> u64 {
        self.model.max_tokens
    }

    fn max_output_tokens(&self) -> Option<u64> {
        self.model.max_output_tokens
    }

    fn stream_completion(
        &self,
        request: LanguageModelRequest,
        cx: &AsyncApp,
    ) -> BoxFuture<
        'static,
        Result<
            futures::stream::BoxStream<
                'static,
                Result<LanguageModelCompletionEvent, LanguageModelCompletionError>,
            >,
            LanguageModelCompletionError,
        >,
    > {
        self.stream_completion(request, cx)
    }
}

#[derive(Clone)]
struct LlamaCppLaunchConfig {
    api_url: String,
    host: String,
    port: u16,
    server_path: Option<PathBuf>,
    models_dir: PathBuf,
    cache_dir: PathBuf,
    tools_dir: PathBuf,
    auto_start: bool,
    auto_download_models: bool,
    auto_install_server: bool,
    context_window: u64,
    threads: Option<u64>,
    gpu_layers: Option<i64>,
    model: AvailableModel,
}

impl LlamaCppLaunchConfig {
    fn signature(&self, server_path: &Path) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            server_path.display(),
            self.api_url,
            self.model.name,
            self.model.local_path.as_deref().unwrap_or_default(),
            self.model.hf_repo.as_deref().unwrap_or_default(),
            self.model.hf_quant.as_deref().unwrap_or_default(),
            self.context_window,
            self.threads
                .map(|threads| threads.to_string())
                .unwrap_or_default(),
            self.gpu_layers
                .map(|gpu_layers| gpu_layers.to_string())
                .unwrap_or_default(),
            LOCAL_LLAMA_SERVER_PARALLEL_SLOTS,
        )
    }
}

#[derive(Default)]
struct LlamaCppRuntime {
    startup_lock: async_lock::Mutex<()>,
    running: Mutex<Option<RunningServer>>,
}

struct RunningServer {
    signature: String,
    child: Child,
}

impl Drop for LlamaCppRuntime {
    fn drop(&mut self) {
        if let Some(mut running) = self.running.lock().ok().and_then(|mut child| child.take()) {
            _ = running.child.kill();
        }
    }
}

impl LlamaCppRuntime {
    async fn ensure_server(
        &self,
        config: LlamaCppLaunchConfig,
        http_client: Arc<dyn HttpClient>,
    ) -> Result<()> {
        if server_is_ready_for_model(http_client.as_ref(), &config).await {
            return Ok(());
        }

        let _startup_guard = self.startup_lock.lock().await;
        if server_is_ready_for_model(http_client.as_ref(), &config).await {
            return Ok(());
        }

        if !config.auto_start {
            return Ok(());
        }

        let server_path = self
            .resolve_server_path(&config, http_client.clone())
            .await
            .context("finding llama-server")?;
        let signature = config.signature(&server_path);

        {
            let mut running = self.running.lock().unwrap();
            if let Some(running_server) = running.as_mut() {
                if running_server.child.try_wait()?.is_none()
                    && running_server.signature == signature
                {
                    return Ok(());
                }
                log::info!(
                    "restarting llama.cpp server for selected model {}",
                    config.model.name
                );
                _ = running_server.child.kill();
                _ = running_server.child.wait();
                *running = None;
            }
        }

        if server_is_ready(http_client.as_ref(), &config.api_url).await {
            let loaded_models = server_model_names(http_client.as_ref(), &config.api_url)
                .await
                .unwrap_or_default();
            let loaded_model_names = loaded_models.join(", ");
            if server_models_include(&loaded_models, &config.model) {
                let running_props = server_props(http_client.as_ref(), &config.api_url)
                    .await
                    .ok();
                let running_settings = running_props
                    .as_ref()
                    .map(|props| props.describe())
                    .unwrap_or_else(|| "unknown server settings".to_string());
                log::warn!(
                    "reusing already-running llama.cpp server at {} with model(s) [{}] and settings ({}); requested {} ctx and {} parallel slot",
                    config.api_url,
                    loaded_model_names,
                    running_settings,
                    config.context_window,
                    LOCAL_LLAMA_SERVER_PARALLEL_SLOTS
                );
                return Ok(());
            } else {
                if stop_existing_local_llama_server_for_switch(&config, &server_path)? {
                    log::info!(
                        "stopped existing local llama-server at {} with model(s) [{}] so {} can start",
                        config.api_url,
                        loaded_model_names,
                        config.model.name
                    );
                    async_io::Timer::after(Duration::from_millis(500)).await;
                } else {
                    anyhow::bail!(
                        "llama-server is already running at {} with model(s) [{}], but {} was selected. Stop the existing llama-server or use a different port.",
                        config.api_url,
                        loaded_model_names,
                        config.model.name
                    );
                }
            }
        }

        std::fs::create_dir_all(&config.models_dir)
            .with_context(|| format!("creating {}", config.models_dir.display()))?;
        std::fs::create_dir_all(&config.cache_dir)
            .with_context(|| format!("creating {}", config.cache_dir.display()))?;
        std::fs::create_dir_all(&config.tools_dir)
            .with_context(|| format!("creating {}", config.tools_dir.display()))?;

        let model_path = if let Some(model_path) = resolve_local_model_path(&config) {
            Some(model_path)
        } else if config.auto_download_models {
            download_configured_model_file(&config, http_client.as_ref()).await?
        } else {
            None
        };

        let log_path = config.tools_dir.join("llama-server.log");
        let stdout = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("opening {}", log_path.display()))?;
        let stderr = stdout
            .try_clone()
            .with_context(|| format!("cloning {}", log_path.display()))?;

        let mut command = Command::new(&server_path);
        command
            .arg("--host")
            .arg(&config.host)
            .arg("--port")
            .arg(config.port.to_string())
            .arg("-c")
            .arg(config.context_window.to_string())
            .arg("--parallel")
            .arg(LOCAL_LLAMA_SERVER_PARALLEL_SLOTS.to_string())
            .arg("--alias")
            .arg(&config.model.name)
            .arg("--jinja")
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .env("HF_HOME", &config.cache_dir)
            .env("HF_HUB_CACHE", config.cache_dir.join("hub"))
            .env("LLAMA_CACHE", config.cache_dir.join("llama.cpp"))
            .env("GIT_TERMINAL_PROMPT", "0");

        if let Some(parent) = server_path.parent() {
            command.current_dir(parent);
        }

        if let Some(threads) = config.threads {
            let threads = threads.to_string();
            command.arg("-t").arg(&threads).arg("-tb").arg(threads);
        }

        if let Some(gpu_layers) = config.gpu_layers {
            command.arg("-ngl").arg(gpu_layers.to_string());
        }

        if let Some(model_path) = model_path {
            command.arg("-m").arg(model_path);
        } else if config.auto_download_models {
            add_hugging_face_model_args(&mut command, &config.model)?;
        } else {
            anyhow::bail!(
                "model {} was not found in {} and auto_download_models is disabled",
                config.model.name,
                config.models_dir.display()
            );
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }

        log::info!(
            "launching llama.cpp server for model {} at {}",
            config.model.name,
            config.api_url
        );

        let child = command
            .spawn()
            .with_context(|| format!("starting {}", server_path.display()))?;

        {
            let mut running = self.running.lock().unwrap();
            *running = Some(RunningServer { signature, child });
        }

        for _ in 0..900 {
            if server_is_ready_for_model(http_client.as_ref(), &config).await {
                warm_up_llama_server(http_client.as_ref(), &config).await;
                return Ok(());
            }
            {
                let mut running = self.running.lock().unwrap();
                if let Some(running_server) = running.as_mut()
                    && let Some(status) = running_server.child.try_wait()?
                {
                    *running = None;
                    anyhow::bail!("llama-server exited before becoming ready: {status}");
                }
            }
            async_io::Timer::after(Duration::from_secs(1)).await;
        }

        anyhow::bail!(
            "timed out waiting for llama-server at {}. The first model download can be large; try again after it finishes.",
            config.api_url
        );
    }

    async fn resolve_server_path(
        &self,
        config: &LlamaCppLaunchConfig,
        http_client: Arc<dyn HttpClient>,
    ) -> Result<PathBuf> {
        if let Some(path) = config.server_path.as_ref()
            && (path.exists() || path.components().count() == 1)
        {
            return Ok(path.clone());
        }

        if let Some(path) = find_server_binary(&config.tools_dir) {
            return Ok(path);
        }

        if let Some(path) = find_server_on_path() {
            return Ok(path);
        }

        if !config.auto_install_server {
            anyhow::bail!(
                "llama-server was not found. Install llama.cpp or set language_models.llama_cpp.server_path"
            );
        }

        download_llama_server(http_client, &config.tools_dir).await
    }
}

async fn server_is_ready(http_client: &dyn HttpClient, api_url: &str) -> bool {
    let url = format!("{}/health", api_url.trim_end_matches('/'));
    let Ok(response) = http_client.get(&url, AsyncBody::default(), true).await else {
        return false;
    };
    response.status().is_success()
}

async fn server_is_ready_for_model(
    http_client: &dyn HttpClient,
    config: &LlamaCppLaunchConfig,
) -> bool {
    if !server_is_ready(http_client, &config.api_url).await {
        return false;
    }

    if !server_model_names(http_client, &config.api_url)
        .await
        .is_ok_and(|models| server_models_include(&models, &config.model))
    {
        return false;
    }

    server_props(http_client, &config.api_url)
        .await
        .map(|props| props.matches_config(config))
        .unwrap_or(true)
}

async fn server_model_names(http_client: &dyn HttpClient, api_url: &str) -> Result<Vec<String>> {
    let url = format!("{}/models", api_url.trim_end_matches('/'));
    let mut response = http_client
        .get(&url, AsyncBody::default(), true)
        .await
        .with_context(|| format!("checking loaded llama.cpp models at {url}"))?;
    anyhow::ensure!(
        response.status().is_success(),
        "checking loaded llama.cpp models returned {}",
        response.status()
    );

    let mut body = String::new();
    response
        .body_mut()
        .read_to_string(&mut body)
        .await
        .with_context(|| format!("reading {url}"))?;

    let response: LlamaModelsResponse =
        serde_json::from_str(&body).with_context(|| format!("parsing {url} response"))?;
    let mut names = Vec::new();
    for model in response.data.into_iter().chain(response.models) {
        if let Some(id) = model.id {
            names.push(id);
        }
        if let Some(name) = model.name {
            names.push(name);
        }
        if let Some(model_name) = model.model {
            names.push(model_name);
        }
        names.extend(model.aliases);
    }

    names.sort();
    names.dedup();
    Ok(names)
}

fn server_models_include(models: &[String], requested: &AvailableModel) -> bool {
    let requested_names = requested_model_names(requested);
    models
        .iter()
        .map(|model| model.to_lowercase())
        .any(|model| requested_names.iter().any(|requested| requested == &model))
}

fn requested_model_names(model: &AvailableModel) -> Vec<String> {
    let mut names = vec![model.name.to_lowercase()];
    if let Some(local_path) = model.local_path.as_ref().map(PathBuf::from)
        && let Some(stem) = local_path.file_stem().and_then(|stem| stem.to_str())
    {
        names.push(stem.to_lowercase());
    }
    if let Some(file) = model.hf_file.as_deref()
        && let Some(stem) = Path::new(file).file_stem().and_then(|stem| stem.to_str())
    {
        names.push(stem.to_lowercase());
    }
    names.sort();
    names.dedup();
    names
}

async fn server_props(http_client: &dyn HttpClient, api_url: &str) -> Result<LlamaServerProps> {
    let url = format!("{}/props", native_api_url(api_url));
    let mut response = http_client
        .get(&url, AsyncBody::default(), true)
        .await
        .with_context(|| format!("checking llama.cpp server props at {url}"))?;
    anyhow::ensure!(
        response.status().is_success(),
        "checking llama.cpp server props returned {}",
        response.status()
    );

    let mut body = String::new();
    response
        .body_mut()
        .read_to_string(&mut body)
        .await
        .with_context(|| format!("reading {url}"))?;

    serde_json::from_str(&body).with_context(|| format!("parsing {url} response"))
}

async fn effective_launch_config(
    http_client: &dyn HttpClient,
    mut config: LlamaCppLaunchConfig,
) -> LlamaCppLaunchConfig {
    let Ok(props) = server_props(http_client, &config.api_url).await else {
        return config;
    };
    if let Some(context_window) = props.context_window()
        && context_window != config.context_window
    {
        log::warn!(
            "using running llama.cpp context window {} for model {} instead of configured {}",
            context_window,
            config.model.name,
            config.context_window
        );
        config.context_window = context_window;
    }
    config
}

#[derive(Deserialize)]
struct LlamaServerProps {
    default_generation_settings: Option<LlamaDefaultGenerationSettings>,
    total_slots: Option<u64>,
}

impl LlamaServerProps {
    fn context_window(&self) -> Option<u64> {
        self.default_generation_settings
            .as_ref()
            .and_then(|settings| settings.n_ctx)
    }

    fn matches_config(&self, config: &LlamaCppLaunchConfig) -> bool {
        self.context_window()
            .map_or(true, |n_ctx| n_ctx == config.context_window)
            && self
                .total_slots
                .map_or(true, |slots| slots == LOCAL_LLAMA_SERVER_PARALLEL_SLOTS)
    }

    fn describe(&self) -> String {
        let context = self
            .context_window()
            .map(|context| context.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let slots = self
            .total_slots
            .map(|slots| slots.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        format!("{context} ctx, {slots} parallel slots")
    }
}

#[derive(Deserialize)]
struct LlamaDefaultGenerationSettings {
    n_ctx: Option<u64>,
}

#[derive(Deserialize)]
struct LlamaModelsResponse {
    #[serde(default)]
    data: Vec<LlamaModelListEntry>,
    #[serde(default)]
    models: Vec<LlamaModelListEntry>,
}

#[derive(Deserialize)]
struct LlamaModelListEntry {
    id: Option<String>,
    name: Option<String>,
    model: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
}

async fn stream_llama_completion(
    http_client: &dyn HttpClient,
    config: &LlamaCppLaunchConfig,
    request: LanguageModelRequest,
) -> Result<
    BoxStream<'static, Result<LanguageModelCompletionEvent, LanguageModelCompletionError>>,
    LanguageModelCompletionError,
> {
    let prompt = apply_llama_chat_template(http_client, config, &request).await?;
    let native_url = native_api_url(&config.api_url);
    let url = format!("{native_url}/completion");
    let request = LlamaCompletionRequest {
        prompt,
        stream: true,
        n_predict: config
            .model
            .max_output_tokens
            .and_then(|tokens| i64::try_from(tokens).ok())
            .unwrap_or(1024),
        stop: request.stop,
        temperature: request.temperature,
        cache_prompt: true,
        timings_per_token: false,
        return_progress: false,
    };

    let mut response = send_json(http_client, &url, &request).await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response_body_string(&mut response).await?;
        return Err(map_llama_http_error(status, body));
    }

    let reader = BufReader::new(response.into_body());
    let stream = reader
        .lines()
        .filter_map(|line| async move {
            let line = match line {
                Ok(line) => line,
                Err(error) => {
                    return Some(vec![Err(
                        LanguageModelCompletionError::ApiReadResponseError {
                            provider: PROVIDER_NAME,
                            error,
                        },
                    )]);
                }
            };
            let Some(line) = line
                .strip_prefix("data: ")
                .or_else(|| line.strip_prefix("data:"))
            else {
                return None;
            };
            let line = line.trim();
            if line.is_empty() || line == "[DONE]" {
                return None;
            }

            let chunk = match serde_json::from_str::<LlamaCompletionChunk>(line) {
                Ok(chunk) => chunk,
                Err(error) => {
                    return Some(vec![Err(
                        LanguageModelCompletionError::DeserializeResponse {
                            provider: PROVIDER_NAME,
                            error,
                        },
                    )]);
                }
            };

            let mut events = Vec::new();
            if let Some(content) = chunk.content
                && !content.is_empty()
            {
                events.push(Ok(LanguageModelCompletionEvent::Text(content)));
            }

            if chunk.stop.unwrap_or(false) {
                events.push(Ok(LanguageModelCompletionEvent::UsageUpdate(TokenUsage {
                    input_tokens: chunk.tokens_evaluated.unwrap_or(0),
                    output_tokens: chunk.tokens_predicted.unwrap_or(0),
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: chunk.tokens_cached.unwrap_or(0),
                })));

                let stop_reason = match chunk.stop_type.as_deref() {
                    Some("limit") => StopReason::MaxTokens,
                    _ if chunk.truncated.unwrap_or(false) => StopReason::MaxTokens,
                    _ => StopReason::EndTurn,
                };
                events.push(Ok(LanguageModelCompletionEvent::Stop(stop_reason)));
            }

            Some(events)
        })
        .flat_map(futures::stream::iter)
        .boxed();

    Ok(stream)
}

async fn apply_llama_chat_template(
    http_client: &dyn HttpClient,
    config: &LlamaCppLaunchConfig,
    request: &LanguageModelRequest,
) -> Result<String, LanguageModelCompletionError> {
    let native_url = native_api_url(&config.api_url);
    let messages = request
        .messages
        .iter()
        .filter_map(llama_chat_message_from_request)
        .collect::<Vec<_>>();
    let prompt =
        apply_llama_template_to_messages(http_client, &native_url, messages.clone()).await?;
    let prompt_budget = llama_prompt_token_budget(config);

    if llama_prompt_fits(http_client, &native_url, &prompt, prompt_budget).await {
        return Ok(prompt);
    }

    fit_llama_prompt_to_context(http_client, &native_url, messages, prompt_budget)
        .await
        .unwrap_or(Ok(prompt))
}

async fn apply_llama_template_to_messages(
    http_client: &dyn HttpClient,
    native_url: &str,
    messages: Vec<LlamaChatMessage>,
) -> Result<String, LanguageModelCompletionError> {
    let url = format!("{native_url}/apply-template");
    let request = LlamaApplyTemplateRequest {
        messages,
        add_generation_prompt: true,
        chat_template_kwargs: LlamaChatTemplateKwargs {
            enable_thinking: false,
        },
    };
    let mut response = send_json(http_client, &url, &request).await?;
    let status = response.status();
    let body = response_body_string(&mut response).await?;

    if !status.is_success() {
        return Err(map_llama_http_error(status, body));
    }

    let response: LlamaApplyTemplateResponse = serde_json::from_str(&body).map_err(|error| {
        LanguageModelCompletionError::DeserializeResponse {
            provider: PROVIDER_NAME,
            error,
        }
    })?;
    Ok(response.prompt)
}

async fn fit_llama_prompt_to_context(
    http_client: &dyn HttpClient,
    native_url: &str,
    messages: Vec<LlamaChatMessage>,
    prompt_budget: u64,
) -> Option<Result<String, LanguageModelCompletionError>> {
    if messages.len() <= 1 {
        return trim_last_llama_message_to_context(
            http_client,
            native_url,
            messages,
            prompt_budget,
        )
        .await;
    }

    let system_message = messages
        .first()
        .filter(|message| message.role == "system")
        .cloned();
    let first_conversation_index = usize::from(system_message.is_some());
    for start in (first_conversation_index + 1)..messages.len() {
        let mut candidate = Vec::new();
        if let Some(system_message) = system_message.clone() {
            candidate.push(system_message);
        }
        candidate.extend(messages[start..].iter().cloned());
        let prompt = match apply_llama_template_to_messages(
            http_client,
            native_url,
            candidate.clone(),
        )
        .await
        {
            Ok(prompt) => prompt,
            Err(error) => return Some(Err(error)),
        };
        if llama_prompt_fits(http_client, native_url, &prompt, prompt_budget).await {
            log::warn!(
                "trimmed older llama.cpp chat history to fit local context window ({} messages kept)",
                candidate.len()
            );
            return Some(Ok(prompt));
        }
    }

    trim_last_llama_message_to_context(http_client, native_url, messages, prompt_budget).await
}

async fn trim_last_llama_message_to_context(
    http_client: &dyn HttpClient,
    native_url: &str,
    messages: Vec<LlamaChatMessage>,
    prompt_budget: u64,
) -> Option<Result<String, LanguageModelCompletionError>> {
    let last_message = messages.last()?;
    let system_message = messages
        .first()
        .filter(|message| message.role == "system")
        .cloned();
    let mut keep_chars = last_message.content.chars().count().saturating_mul(3) / 4;
    keep_chars = keep_chars.max(512);

    while keep_chars >= 512 {
        let mut candidate = Vec::new();
        if let Some(system_message) = system_message.clone() {
            candidate.push(system_message);
        }
        candidate.push(LlamaChatMessage {
            role: last_message.role.clone(),
            content: tail_content_for_local_context(&last_message.content, keep_chars),
        });
        let prompt =
            match apply_llama_template_to_messages(http_client, native_url, candidate).await {
                Ok(prompt) => prompt,
                Err(error) => return Some(Err(error)),
            };
        if llama_prompt_fits(http_client, native_url, &prompt, prompt_budget).await {
            log::warn!("trimmed oversized llama.cpp prompt tail to fit local context window");
            return Some(Ok(prompt));
        }
        keep_chars /= 2;
    }

    None
}

fn tail_content_for_local_context(content: &str, keep_chars: usize) -> String {
    let total_chars = content.chars().count();
    if total_chars <= keep_chars {
        return content.to_string();
    }

    let tail = content
        .chars()
        .skip(total_chars.saturating_sub(keep_chars))
        .collect::<String>();
    format!("[Earlier local context was omitted to fit this llama.cpp model.]\n{tail}")
}

async fn llama_prompt_fits(
    http_client: &dyn HttpClient,
    native_url: &str,
    prompt: &str,
    prompt_budget: u64,
) -> bool {
    llama_prompt_token_count(http_client, native_url, prompt)
        .await
        .map(|tokens| tokens <= prompt_budget)
        .unwrap_or(true)
}

async fn llama_prompt_token_count(
    http_client: &dyn HttpClient,
    native_url: &str,
    prompt: &str,
) -> Result<u64, LanguageModelCompletionError> {
    let url = format!("{native_url}/tokenize");
    let request = LlamaTokenizeRequest {
        content: prompt.to_string(),
    };
    let mut response = send_json(http_client, &url, &request).await?;
    let status = response.status();
    let body = response_body_string(&mut response).await?;

    if !status.is_success() {
        return Err(map_llama_http_error(status, body));
    }

    let response: LlamaTokenizeResponse = serde_json::from_str(&body).map_err(|error| {
        LanguageModelCompletionError::DeserializeResponse {
            provider: PROVIDER_NAME,
            error,
        }
    })?;
    Ok(response.tokens.len() as u64)
}

fn llama_prompt_token_budget(config: &LlamaCppLaunchConfig) -> u64 {
    let output_tokens = config.model.max_output_tokens.unwrap_or(1024);
    config
        .context_window
        .saturating_sub(output_tokens)
        .saturating_sub(LOCAL_LLAMA_PROMPT_TOKEN_MARGIN)
        .max(config.context_window / 2)
}

async fn warm_up_llama_server(http_client: &dyn HttpClient, config: &LlamaCppLaunchConfig) {
    let native_url = native_api_url(&config.api_url);
    let request = LlamaCompletionRequest {
        prompt: "User: warm up.\nAssistant:".to_string(),
        stream: false,
        n_predict: 1,
        stop: Vec::new(),
        temperature: Some(0.0),
        cache_prompt: false,
        timings_per_token: false,
        return_progress: false,
    };

    match send_json(http_client, &format!("{native_url}/completion"), &request).await {
        Ok(mut response) => {
            let status = response.status();
            if status.is_success() {
                let _ = response_body_string(&mut response).await;
            } else {
                let body = response_body_string(&mut response)
                    .await
                    .unwrap_or_else(|_| String::new());
                log::warn!(
                    "llama.cpp warmup for {} returned {}: {}",
                    config.model.name,
                    status,
                    body
                );
            }
        }
        Err(error) => {
            log::warn!(
                "llama.cpp warmup for {} failed: {}",
                config.model.name,
                error
            );
        }
    }
}

fn llama_chat_message_from_request(
    message: &language_model::LanguageModelRequestMessage,
) -> Option<LlamaChatMessage> {
    let mut content = String::new();
    for part in &message.content {
        match part {
            MessageContent::Text(text) => content.push_str(text),
            MessageContent::Thinking { text, .. } => content.push_str(text),
            MessageContent::ToolResult(tool_result) => {
                content.push_str(&tool_result.text_contents());
            }
            MessageContent::ToolUse(tool_use) => {
                content.push_str(&format!(
                    "\n[tool call: {} {}]\n",
                    tool_use.name, tool_use.raw_input
                ));
            }
            MessageContent::RedactedThinking(_) | MessageContent::Image(_) => {}
        }
    }

    if content.trim().is_empty() {
        return None;
    }

    let role = match message.role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
    };

    Some(LlamaChatMessage {
        role: role.to_string(),
        content,
    })
}

async fn send_json<T: Serialize>(
    http_client: &dyn HttpClient,
    url: &str,
    body: &T,
) -> Result<http_client::Response<AsyncBody>, LanguageModelCompletionError> {
    let request = HttpRequest::builder()
        .method(Method::POST)
        .uri(url)
        .header("Content-Type", "application/json")
        .body(AsyncBody::from(serde_json::to_string(body).map_err(
            |error| LanguageModelCompletionError::SerializeRequest {
                provider: PROVIDER_NAME,
                error,
            },
        )?))
        .map_err(|error| LanguageModelCompletionError::BuildRequestBody {
            provider: PROVIDER_NAME,
            error,
        })?;

    http_client
        .send(request)
        .await
        .map_err(|error| LanguageModelCompletionError::HttpSend {
            provider: PROVIDER_NAME,
            error,
        })
}

async fn response_body_string(
    response: &mut http_client::Response<AsyncBody>,
) -> Result<String, LanguageModelCompletionError> {
    let mut body = String::new();
    response
        .body_mut()
        .read_to_string(&mut body)
        .await
        .map_err(|error| LanguageModelCompletionError::ApiReadResponseError {
            provider: PROVIDER_NAME,
            error,
        })?;
    Ok(body)
}

fn map_llama_http_error(status: StatusCode, body: String) -> LanguageModelCompletionError {
    if body.contains("exceed_context_size_error")
        || body.contains("exceeds the available context size")
    {
        return LanguageModelCompletionError::PromptTooLarge { tokens: None };
    }

    LanguageModelCompletionError::from_http_status(PROVIDER_NAME, status, body, None)
}

fn native_api_url(api_url: &str) -> String {
    api_url
        .trim_end_matches('/')
        .strip_suffix("/v1")
        .unwrap_or_else(|| api_url.trim_end_matches('/'))
        .to_string()
}

#[derive(Serialize)]
struct LlamaApplyTemplateRequest {
    messages: Vec<LlamaChatMessage>,
    add_generation_prompt: bool,
    chat_template_kwargs: LlamaChatTemplateKwargs,
}

#[derive(Serialize)]
struct LlamaChatTemplateKwargs {
    enable_thinking: bool,
}

#[derive(Clone, Serialize)]
struct LlamaChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct LlamaApplyTemplateResponse {
    prompt: String,
}

#[derive(Serialize)]
struct LlamaTokenizeRequest {
    content: String,
}

#[derive(Deserialize)]
struct LlamaTokenizeResponse {
    tokens: Vec<i64>,
}

#[derive(Serialize)]
struct LlamaCompletionRequest {
    prompt: String,
    stream: bool,
    n_predict: i64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    cache_prompt: bool,
    timings_per_token: bool,
    return_progress: bool,
}

#[derive(Deserialize)]
struct LlamaCompletionChunk {
    content: Option<String>,
    stop: Option<bool>,
    stop_type: Option<String>,
    truncated: Option<bool>,
    tokens_cached: Option<u64>,
    tokens_evaluated: Option<u64>,
    tokens_predicted: Option<u64>,
}

fn add_hugging_face_model_args(command: &mut Command, model: &AvailableModel) -> Result<()> {
    let repo = model
        .hf_repo
        .as_deref()
        .ok_or_else(|| anyhow!("{} has no hf_repo configured", model.name))?;

    if let Some(quant) = model.hf_quant.as_deref() {
        command.arg("-hf").arg(format!("{repo}:{quant}"));
    } else if let Some(file) = model.hf_file.as_deref() {
        command
            .arg("--hf-repo")
            .arg(repo)
            .arg("--hf-file")
            .arg(file);
    } else {
        command.arg("-hf").arg(repo);
    }

    if !model.supports_images {
        command.arg("--no-mmproj");
    }

    Ok(())
}

async fn download_configured_model_file(
    config: &LlamaCppLaunchConfig,
    http_client: &dyn HttpClient,
) -> Result<Option<PathBuf>> {
    let Some(file_name) = config.model.hf_file.as_deref() else {
        return Ok(None);
    };
    let Some(repo) = config.model.hf_repo.as_deref() else {
        anyhow::bail!("{} has no hf_repo configured", config.model.name);
    };

    let target_path = config
        .model
        .local_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| config.models_dir.join(file_name));
    if target_path.exists() {
        return Ok(Some(target_path));
    }

    let Some(parent) = target_path.parent() else {
        anyhow::bail!("model path has no parent: {}", target_path.display());
    };
    async_fs::create_dir_all(parent)
        .await
        .with_context(|| format!("creating {}", parent.display()))?;

    let temp_path = target_path.with_file_name(format!(
        ".{}.download",
        target_path
            .file_name()
            .map(|file_name| file_name.to_string_lossy())
            .unwrap_or_else(|| config.model.name.clone().into())
    ));
    _ = async_fs::remove_file(&temp_path).await;

    let url = format!("https://huggingface.co/{repo}/resolve/main/{file_name}");
    log::info!(
        "downloading llama.cpp model from {} to {}",
        url,
        target_path.display()
    );
    let mut response = http_client
        .get(&url, AsyncBody::default(), true)
        .await
        .with_context(|| format!("downloading {url}"))?;
    anyhow::ensure!(
        response.status().is_success(),
        "downloading {url} returned {}",
        response.status()
    );

    let mut output = async_fs::File::create(&temp_path)
        .await
        .with_context(|| format!("creating {}", temp_path.display()))?;
    let bytes = futures::io::copy(response.body_mut(), &mut output)
        .await
        .with_context(|| format!("saving {url} to {}", temp_path.display()))?;
    output
        .close()
        .await
        .with_context(|| format!("closing {}", temp_path.display()))?;
    anyhow::ensure!(bytes > 0, "downloaded zero bytes from {url}");

    async_fs::rename(&temp_path, &target_path)
        .await
        .with_context(|| {
            format!(
                "renaming {} to {}",
                temp_path.display(),
                target_path.display()
            )
        })?;

    Ok(Some(target_path))
}

fn resolve_local_model_path(config: &LlamaCppLaunchConfig) -> Option<PathBuf> {
    if let Some(local_path) = config.model.local_path.as_ref().map(PathBuf::from)
        && local_path.exists()
    {
        return Some(local_path);
    }

    let expected_file = config
        .model
        .hf_file
        .as_deref()
        .map(str::to_owned)
        .or_else(|| {
            config
                .model
                .hf_quant
                .as_ref()
                .map(|quant| format!("{}-{quant}.gguf", config.model.name))
        });
    let expected_file = expected_file.as_deref();
    for dir in candidate_model_dirs(&config.models_dir) {
        if let Some(path) = find_gguf_in_dir(&dir, expected_file, &config.model.name, 4) {
            return Some(path);
        }
    }

    None
}

fn discover_local_gguf_models(
    primary: &Path,
    known_local_paths: &[PathBuf],
    context_window: u64,
) -> Vec<AvailableModel> {
    let known_local_paths = known_local_paths
        .iter()
        .map(normalized_path_key)
        .collect::<Vec<_>>();
    let mut models = BTreeMap::new();

    for dir in candidate_model_dirs(primary) {
        let mut files = Vec::new();
        collect_gguf_files(&dir, 4, &mut files);
        for path in files {
            if known_local_paths
                .iter()
                .any(|known_path| known_path == &normalized_path_key(&path))
            {
                continue;
            }

            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let name = stem.to_string();
            let inferred_context_window = infer_gguf_context_window(&path)
                .map(|window| window.min(context_window))
                .unwrap_or(context_window);
            let model = apply_local_model_override(
                AvailableModel {
                    name,
                    display_name: Some(display_name_from_file_stem(stem)),
                    max_tokens: inferred_context_window,
                    max_output_tokens: Some(2048),
                    hf_repo: None,
                    hf_quant: None,
                    hf_file: path
                        .file_name()
                        .and_then(|file_name| file_name.to_str())
                        .map(str::to_owned),
                    local_path: Some(path.to_string_lossy().replace('\\', "/")),
                    supports_tools: false,
                    supports_images: false,
                },
                &path,
            );
            models.entry(model.name.clone()).or_insert(model);
        }
    }

    models.into_values().collect()
}

#[derive(Default, Deserialize)]
struct LocalModelOverride {
    name: Option<String>,
    display_name: Option<String>,
    max_tokens: Option<u64>,
    max_output_tokens: Option<u64>,
    hf_repo: Option<String>,
    hf_quant: Option<String>,
    hf_file: Option<String>,
    local_path: Option<String>,
    supports_tools: Option<bool>,
    supports_images: Option<bool>,
}

fn apply_local_model_override(mut model: AvailableModel, path: &Path) -> AvailableModel {
    let Some(override_settings) = read_local_model_override(path) else {
        return model;
    };

    if let Some(name) = override_settings.name {
        model.name = name;
    }
    if let Some(display_name) = override_settings.display_name {
        model.display_name = Some(display_name);
    }
    if let Some(max_tokens) = override_settings.max_tokens {
        model.max_tokens = max_tokens;
    }
    if let Some(max_output_tokens) = override_settings.max_output_tokens {
        model.max_output_tokens = Some(max_output_tokens);
    }
    if let Some(hf_repo) = override_settings.hf_repo {
        model.hf_repo = Some(hf_repo);
    }
    if let Some(hf_quant) = override_settings.hf_quant {
        model.hf_quant = Some(hf_quant);
    }
    if let Some(hf_file) = override_settings.hf_file {
        model.hf_file = Some(hf_file);
    }
    if let Some(local_path) = override_settings.local_path {
        model.local_path = Some(local_path);
    }
    if let Some(supports_tools) = override_settings.supports_tools {
        model.supports_tools = supports_tools;
    }
    if let Some(supports_images) = override_settings.supports_images {
        model.supports_images = supports_images;
    }

    model
}

fn read_local_model_override(path: &Path) -> Option<LocalModelOverride> {
    for sidecar_path in [
        path.with_extension("gguf.json"),
        path.with_extension("json"),
    ] {
        if !sidecar_path.exists() {
            continue;
        }
        match std::fs::read_to_string(&sidecar_path)
            .with_context(|| format!("reading {}", sidecar_path.display()))
            .and_then(|content| {
                serde_json::from_str(&content)
                    .with_context(|| format!("parsing {}", sidecar_path.display()))
            }) {
            Ok(override_settings) => return Some(override_settings),
            Err(error) => {
                log::warn!("failed to read llama.cpp local model override: {error:#}");
            }
        }
    }

    None
}

const GGUF_METADATA_READ_LIMIT: u64 = 1024 * 1024;
const GGUF_TYPE_UINT8: u32 = 0;
const GGUF_TYPE_INT8: u32 = 1;
const GGUF_TYPE_UINT16: u32 = 2;
const GGUF_TYPE_INT16: u32 = 3;
const GGUF_TYPE_UINT32: u32 = 4;
const GGUF_TYPE_INT32: u32 = 5;
const GGUF_TYPE_FLOAT32: u32 = 6;
const GGUF_TYPE_BOOL: u32 = 7;
const GGUF_TYPE_STRING: u32 = 8;
const GGUF_TYPE_ARRAY: u32 = 9;
const GGUF_TYPE_UINT64: u32 = 10;
const GGUF_TYPE_INT64: u32 = 11;
const GGUF_TYPE_FLOAT64: u32 = 12;

fn infer_gguf_context_window(path: &Path) -> Option<u64> {
    let mut file = File::open(path).ok()?;
    let mut buffer = vec![0; GGUF_METADATA_READ_LIMIT as usize];
    let len = file.read(&mut buffer).ok()?;
    buffer.truncate(len);

    let mut cursor = Cursor::new(buffer.as_slice());
    let mut magic = [0; 4];
    cursor.read_exact(&mut magic).ok()?;
    if &magic != b"GGUF" {
        return None;
    }

    let _version = read_gguf_u32(&mut cursor)?;
    let _tensor_count = read_gguf_u64(&mut cursor)?;
    let metadata_count = read_gguf_u64(&mut cursor)?;
    for _ in 0..metadata_count.min(4096) {
        let key = read_gguf_string(&mut cursor)?;
        let value_type = read_gguf_u32(&mut cursor)?;
        if key.ends_with(".context_length") || key == "context_length" {
            return read_gguf_numeric_value(value_type, &mut cursor);
        }
        skip_gguf_value(value_type, &mut cursor)?;
    }

    None
}

fn read_gguf_u32(cursor: &mut Cursor<&[u8]>) -> Option<u32> {
    let mut bytes = [0; 4];
    cursor.read_exact(&mut bytes).ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn read_gguf_u64(cursor: &mut Cursor<&[u8]>) -> Option<u64> {
    let mut bytes = [0; 8];
    cursor.read_exact(&mut bytes).ok()?;
    Some(u64::from_le_bytes(bytes))
}

fn read_gguf_string(cursor: &mut Cursor<&[u8]>) -> Option<String> {
    let len = usize::try_from(read_gguf_u64(cursor)?).ok()?;
    let position = usize::try_from(cursor.position()).ok()?;
    let end = position.checked_add(len)?;
    if end > cursor.get_ref().len() {
        return None;
    }
    let value = String::from_utf8_lossy(&cursor.get_ref()[position..end]).into_owned();
    cursor.set_position(end as u64);
    Some(value)
}

fn read_gguf_numeric_value(value_type: u32, cursor: &mut Cursor<&[u8]>) -> Option<u64> {
    match value_type {
        GGUF_TYPE_UINT8 => {
            let mut bytes = [0; 1];
            cursor.read_exact(&mut bytes).ok()?;
            Some(bytes[0] as u64)
        }
        GGUF_TYPE_INT8 => {
            let mut bytes = [0; 1];
            cursor.read_exact(&mut bytes).ok()?;
            Some(i8::from_le_bytes(bytes).max(0) as u64)
        }
        GGUF_TYPE_UINT16 => {
            let mut bytes = [0; 2];
            cursor.read_exact(&mut bytes).ok()?;
            Some(u16::from_le_bytes(bytes) as u64)
        }
        GGUF_TYPE_INT16 => {
            let mut bytes = [0; 2];
            cursor.read_exact(&mut bytes).ok()?;
            Some(i16::from_le_bytes(bytes).max(0) as u64)
        }
        GGUF_TYPE_UINT32 => {
            let mut bytes = [0; 4];
            cursor.read_exact(&mut bytes).ok()?;
            Some(u32::from_le_bytes(bytes) as u64)
        }
        GGUF_TYPE_INT32 => {
            let mut bytes = [0; 4];
            cursor.read_exact(&mut bytes).ok()?;
            Some(i32::from_le_bytes(bytes).max(0) as u64)
        }
        GGUF_TYPE_UINT64 => read_gguf_u64(cursor),
        GGUF_TYPE_INT64 => {
            let mut bytes = [0; 8];
            cursor.read_exact(&mut bytes).ok()?;
            Some(i64::from_le_bytes(bytes).max(0) as u64)
        }
        _ => None,
    }
}

fn skip_gguf_value(value_type: u32, cursor: &mut Cursor<&[u8]>) -> Option<()> {
    match value_type {
        GGUF_TYPE_UINT8 | GGUF_TYPE_INT8 | GGUF_TYPE_BOOL => skip_gguf_bytes(cursor, 1),
        GGUF_TYPE_UINT16 | GGUF_TYPE_INT16 => skip_gguf_bytes(cursor, 2),
        GGUF_TYPE_UINT32 | GGUF_TYPE_INT32 | GGUF_TYPE_FLOAT32 => skip_gguf_bytes(cursor, 4),
        GGUF_TYPE_UINT64 | GGUF_TYPE_INT64 | GGUF_TYPE_FLOAT64 => skip_gguf_bytes(cursor, 8),
        GGUF_TYPE_STRING => {
            read_gguf_string(cursor)?;
            Some(())
        }
        GGUF_TYPE_ARRAY => {
            let element_type = read_gguf_u32(cursor)?;
            let len = read_gguf_u64(cursor)?;
            if let Some(element_size) = fixed_gguf_value_size(element_type) {
                let byte_len = usize::try_from(len).ok()?.checked_mul(element_size)?;
                return skip_gguf_bytes(cursor, byte_len);
            }
            None
        }
        _ => None,
    }
}

fn fixed_gguf_value_size(value_type: u32) -> Option<usize> {
    match value_type {
        GGUF_TYPE_UINT8 | GGUF_TYPE_INT8 | GGUF_TYPE_BOOL => Some(1),
        GGUF_TYPE_UINT16 | GGUF_TYPE_INT16 => Some(2),
        GGUF_TYPE_UINT32 | GGUF_TYPE_INT32 | GGUF_TYPE_FLOAT32 => Some(4),
        GGUF_TYPE_UINT64 | GGUF_TYPE_INT64 | GGUF_TYPE_FLOAT64 => Some(8),
        _ => None,
    }
}

fn skip_gguf_bytes(cursor: &mut Cursor<&[u8]>, count: usize) -> Option<()> {
    let position = usize::try_from(cursor.position()).ok()?;
    let end = position.checked_add(count)?;
    if end > cursor.get_ref().len() {
        return None;
    }
    cursor.set_position(end as u64);
    Some(())
}

fn collect_gguf_files(dir: &Path, max_depth: usize, files: &mut Vec<PathBuf>) {
    if max_depth == 0 || !dir.is_dir() {
        return;
    }

    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_gguf_files(&path, max_depth - 1, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("gguf") {
            files.push(path);
        }
    }
}

fn display_name_from_file_stem(stem: &str) -> String {
    stem.replace(['_', '-'], " ")
}

fn normalized_path_key(path: &PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

fn candidate_model_dirs(primary: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![primary.to_path_buf()];
    if cfg!(target_os = "windows") && Path::new("G:/").exists() {
        for path in [
            "G:/Zed/models",
            "G:/Zed/models/llama.cpp",
            "G:/Models",
            "G:/models",
            "G:/AI/models",
        ] {
            let path = PathBuf::from(path);
            if path != primary && path.exists() {
                dirs.push(path);
            }
        }
    }
    dirs
}

fn find_gguf_in_dir(
    dir: &Path,
    expected_file: Option<&str>,
    model_name: &str,
    max_depth: usize,
) -> Option<PathBuf> {
    if max_depth == 0 || !dir.is_dir() {
        return None;
    }

    let expected_file = expected_file.map(|file| file.to_lowercase());
    let model_tokens = model_name
        .to_lowercase()
        .split(['-', '_', '.', ' '])
        .filter(|token| !token.is_empty() && *token != "gguf")
        .map(str::to_owned)
        .collect::<Vec<_>>();

    let read_dir = std::fs::read_dir(dir).ok()?;
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(path) =
                find_gguf_in_dir(&path, expected_file.as_deref(), model_name, max_depth - 1)
            {
                return Some(path);
            }
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("gguf") {
            continue;
        }

        let file_name = path.file_name()?.to_string_lossy().to_lowercase();
        if expected_file
            .as_ref()
            .is_some_and(|expected| &file_name == expected)
            || model_tokens.iter().all(|token| file_name.contains(token))
        {
            return Some(path);
        }
    }

    None
}

async fn download_llama_server(
    http_client: Arc<dyn HttpClient>,
    tools_dir: &Path,
) -> Result<PathBuf> {
    #[cfg(target_family = "wasm")]
    {
        let _ = http_client;
        let _ = tools_dir;
        anyhow::bail!("automatic llama.cpp server installation is not supported on wasm");
    }

    #[cfg(not(target_family = "wasm"))]
    {
        use http_client::{github, github_download};

        std::fs::create_dir_all(tools_dir)
            .with_context(|| format!("creating {}", tools_dir.display()))?;
        let release =
            github::latest_github_release("ggml-org/llama.cpp", true, false, http_client.clone())
                .await?;
        let (asset, asset_kind) = select_llama_release_asset(&release.assets)
            .ok_or_else(|| anyhow!("no llama.cpp release asset matched this OS"))?;
        let destination = tools_dir.join(&release.tag_name);
        github_download::download_server_binary(
            http_client.as_ref(),
            &asset.browser_download_url,
            asset.digest.as_deref(),
            &destination,
            asset_kind,
        )
        .await?;
        find_server_binary(&destination)
            .ok_or_else(|| anyhow!("downloaded llama.cpp release did not contain llama-server"))
    }
}

#[cfg(not(target_family = "wasm"))]
fn select_llama_release_asset<'a>(
    assets: &'a [http_client::github::GithubReleaseAsset],
) -> Option<(
    &'a http_client::github::GithubReleaseAsset,
    http_client::github::AssetKind,
)> {
    fn matches(name: &str, required: &[&str], forbidden: &[&str]) -> bool {
        required.iter().all(|part| name.contains(part))
            && forbidden.iter().all(|part| !name.contains(part))
    }

    let forbidden_accelerators = [
        "cuda", "cudart", "vulkan", "sycl", "hip", "rocm", "openvino",
    ];
    let candidates: &[&[&str]] = if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        &[
            &["bin-win-cpu-x64"],
            &["bin-win-avx2-x64"],
            &["bin-win", "x64"],
        ]
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        &[&["bin-win-cpu-arm64"], &["bin-win", "arm64"]]
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        &[&["bin-ubuntu-cpu-x64"], &["bin-ubuntu-x64"]]
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        &[&["bin-ubuntu-cpu-arm64"], &["bin-ubuntu-arm64"]]
    } else {
        &[]
    };

    for required in candidates {
        if let Some(asset) = assets.iter().find(|asset| {
            let name = asset.name.to_lowercase();
            (name.ends_with(".zip") || name.ends_with(".tar.gz"))
                && matches(&name, required, &forbidden_accelerators)
        }) {
            let name = asset.name.to_lowercase();
            let kind = if name.ends_with(".tar.gz") {
                http_client::github::AssetKind::TarGz
            } else {
                http_client::github::AssetKind::Zip
            };
            return Some((asset, kind));
        }
    }

    None
}

fn find_server_binary(root: &Path) -> Option<PathBuf> {
    find_named_binary(root, server_binary_name(), 5)
}

fn find_named_binary(root: &Path, name: &str, max_depth: usize) -> Option<PathBuf> {
    if max_depth == 0 || !root.is_dir() {
        return None;
    }

    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(path) = find_named_binary(&path, name, max_depth - 1) {
                return Some(path);
            }
        } else if path.file_name().and_then(|name| name.to_str()) == Some(name) {
            return Some(path);
        }
    }

    None
}

fn find_server_on_path() -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|path| path.join(server_binary_name()))
            .find(|path| path.exists())
    })
}

fn server_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "llama-server.exe"
    } else {
        "llama-server"
    }
}

fn stop_existing_local_llama_server_for_switch(
    config: &LlamaCppLaunchConfig,
    server_path: &Path,
) -> Result<bool> {
    let process_refresh_kind = sysinfo::ProcessRefreshKind::nothing()
        .with_cmd(sysinfo::UpdateKind::Always)
        .with_exe(sysinfo::UpdateKind::Always);
    let system = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::nothing().with_processes(process_refresh_kind),
    );
    let mut stopped_any = false;

    for process in system.processes().values() {
        if !is_llama_server_process_name(process.name()) {
            continue;
        }

        let Some(exe) = process.exe() else {
            continue;
        };

        if !is_local_managed_llama_server(exe, server_path, &config.tools_dir) {
            continue;
        }

        log::info!(
            "stopping local llama-server process {} at {} before switching to {}",
            process.pid(),
            exe.display(),
            config.model.name
        );
        stopped_any |= process.kill();
    }

    Ok(stopped_any)
}

fn is_llama_server_process_name(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy()
        .eq_ignore_ascii_case(server_binary_name())
}

fn is_local_managed_llama_server(exe: &Path, server_path: &Path, tools_dir: &Path) -> bool {
    let exe = comparable_path(exe);
    let server_path = comparable_path(server_path);
    let tools_dir = comparable_path(tools_dir);

    exe == server_path || exe.starts_with(&format!("{tools_dir}/"))
}

fn comparable_path(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn is_low_spec_system() -> bool {
    let system = sysinfo::System::new_all();
    system.total_memory() < LOW_SPEC_MEMORY_THRESHOLD_BYTES
}

fn default_models_dir() -> PathBuf {
    if cfg!(target_os = "windows") && Path::new("G:/").exists() {
        PathBuf::from("G:/Zed/models/llama.cpp")
    } else {
        default_user_data_dir().join("models").join("llama.cpp")
    }
}

fn default_cache_dir() -> PathBuf {
    if cfg!(target_os = "windows") && Path::new("G:/").exists() {
        PathBuf::from("G:/Zed/models/huggingface")
    } else {
        default_user_data_dir().join("models").join("huggingface")
    }
}

fn default_tools_dir() -> PathBuf {
    if cfg!(target_os = "windows") && Path::new("G:/").exists() {
        PathBuf::from("G:/Zed/tools/llama.cpp")
    } else {
        default_user_data_dir().join("tools").join("llama.cpp")
    }
}

fn default_user_data_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("Zed")
}

struct ConfigurationView {
    state: Entity<State>,
    http_client: Arc<dyn HttpClient>,
    load_task: Option<Task<()>>,
}

impl ConfigurationView {
    fn new(state: Entity<State>, http_client: Arc<dyn HttpClient>, cx: &mut Context<Self>) -> Self {
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self {
            state,
            http_client,
            load_task: None,
        }
    }

    fn connect(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let (runtime, config) = self.state.read_with(cx, |state, _| {
            let model = state.selected_model();
            (state.runtime.clone(), state.launch_config(&model))
        });
        let http_client = self.http_client.clone();

        self.load_task = Some(cx.spawn(async move |this, cx| {
            let result = runtime.ensure_server(config, http_client).await;
            if let Err(error) = result {
                log::error!("failed to start llama.cpp: {error:?}");
            }
            this.update(cx, |this, cx| {
                this.load_task = None;
                cx.notify();
            })
            .log_err();
        }));
    }
}

impl Render for ConfigurationView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let selected_model = state.selected_model();
        let config = state.launch_config(&selected_model);
        let is_loading = self.load_task.is_some();

        v_flex()
            .gap_2()
            .child(Label::new("Run the agent with a local llama.cpp server."))
            .child(
                ConfiguredApiCard::new(format!(
                    "{} at {}",
                    selected_model
                        .display_name
                        .clone()
                        .unwrap_or_else(|| selected_model.name.clone()),
                    config.api_url
                ))
                .disabled(true),
            )
            .child(
                List::new()
                    .child(ListBulletItem::new(format!(
                        "Models and Hugging Face cache use {}",
                        config.cache_dir.display()
                    )))
                    .child(ListBulletItem::new(format!(
                        "llama.cpp tools use {}",
                        config.tools_dir.display()
                    )))
                    .child(ListBulletItem::new(
                        "Qwen 3.5 0.8B is the fast local default; Gemma 4 and any discovered GGUF models stay available in the model picker.",
                    ))
                    .child(ListBulletItem::new(format!(
                        "Drop GGUF files into {} and optionally add a matching .gguf.json sidecar for custom display names or limits.",
                        config.models_dir.display()
                    ))),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new(
                            "connect-llama-cpp",
                            if is_loading {
                                "Starting..."
                            } else {
                                "Start Local Model"
                            },
                        )
                        .disabled(is_loading)
                        .start_icon(Icon::new(IconName::PlayFilled).size(IconSize::XSmall))
                        .on_click(cx.listener(|this, _, window, cx| this.connect(window, cx))),
                    )
                    .child(
                        ButtonLike::new("llama-cpp-status")
                            .disabled(true)
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(Icon::new(IconName::Check).color(Color::Success))
                                    .child(Label::new("Local Provider"))
                                    .into_any_element(),
                            ),
                    ),
            )
    }
}
