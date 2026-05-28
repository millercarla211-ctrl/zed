import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/project/src/context_server_store.rs";
const source = readFileSync(sourcePath, "utf8");

function sliceBetween(startNeedle: string, endNeedle: string): string {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);

  const end = source.indexOf(endNeedle, start);
  assert.ok(end > start, `expected ${endNeedle} after ${startNeedle}`);

  return source.slice(start, end);
}

function assertBefore(
  haystack: string,
  beforeNeedle: string,
  afterNeedle: string,
  message: string,
) {
  const before = haystack.indexOf(beforeNeedle);
  const after = haystack.indexOf(afterNeedle);
  assert.notEqual(before, -1, `expected ${beforeNeedle}`);
  assert.notEqual(after, -1, `expected ${afterNeedle}`);
  assert.ok(before < after, message);
}

test("context server store caps configured server collections before fanout", () => {
  assert.match(source, /const MAX_CONTEXT_SERVER_CONFIGURED_ENTRIES: usize = 512;/);
  assert.match(source, /const MAX_CONTEXT_SERVER_ENABLED_FANOUT: usize = 512;/);
  assert.match(source, /const MAX_CONTEXT_SERVER_ID_COLLECTION_ENTRIES: usize = 1024;/);
  assert.match(source, /fn bounded_context_server_settings\(/);
  assert.match(source, /configuration_errors: HashMap<ContextServerId, Arc<str>>,/);

  const configuredIds = sliceBetween(
    "pub fn configured_server_ids(&self) -> Vec<ContextServerId> {",
    "\n    #[cfg(feature = \"test-support\")]",
  );
  assertBefore(
    configuredIds,
    ".filter(|(_, settings)| settings.enabled())",
    ".take(MAX_CONTEXT_SERVER_CONFIGURED_ENTRIES)",
    "configured server IDs should filter disabled entries before applying the enabled-ID cap",
  );
  assertBefore(
    configuredIds,
    ".take(MAX_CONTEXT_SERVER_CONFIGURED_ENTRIES)",
    ".collect()",
    "configured server IDs must be capped before collection",
  );

  const populateIds = sliceBetween(
    "fn populate_server_ids(&mut self, cx: &App) {",
    "\n    pub fn running_servers(&self)",
  );
  assertBefore(
    populateIds,
    ".take(MAX_CONTEXT_SERVER_ID_COLLECTION_ENTRIES)",
    ".sorted_unstable_by(",
    "server ID candidates must be capped before sorting/materializing the UI list",
  );

  const maintain = sliceBetween(
    "async fn maintain_servers(this: WeakEntity<Self>, cx: &mut AsyncApp) -> Result<()> {",
    "/// Determines the appropriate server state after a start attempt fails.",
  );
  assertBefore(
    maintain,
    ".take(MAX_CONTEXT_SERVER_CONFIGURED_ENTRIES)",
    ".partition(|(_, settings)| settings.enabled())",
    "configured context servers must be capped before partitioning into enabled fanout",
  );
  assertBefore(
    maintain,
    ".take(MAX_CONTEXT_SERVER_ENABLED_FANOUT)",
    "join_all(enabled_servers.map",
    "enabled context servers must be capped before join_all fanout",
  );
  assertBefore(
    maintain,
    "let mut configuration_errors: HashMap<ContextServerId, Arc<str>> = HashMap::default();",
    "let configured_servers = resolved_servers",
    "configuration resolution errors must be collected before filtering valid configurations",
  );
  assertBefore(
    maintain,
    "configuration_errors.insert(id, err.to_string().into());",
    "this.configuration_errors.insert(id.clone(), error.clone());",
    "configuration errors must remain visible in store state instead of being filtered away",
  );
  assertBefore(
    maintain,
    "this.configuration_errors.insert(id.clone(), error.clone());",
    "status: ContextServerStatus::Error(error)",
    "configuration errors must emit visible error status updates",
  );
});

test("context server store caps remote command responses before env and command materialization", () => {
  assert.match(source, /const MAX_CONTEXT_SERVER_REMOTE_COMMAND_ARGS: usize = 256;/);
  assert.match(source, /const MAX_CONTEXT_SERVER_REMOTE_COMMAND_ENV_VARS: usize = 256;/);
  assert.match(source, /const MAX_CONTEXT_SERVER_COMMAND_FIELD_BYTES: usize = 16 \* 1024;/);
  assert.match(source, /fn bounded_remote_context_server_command\(/);
  assert.match(source, /fn bounded_context_server_env_entries<I>\(/);

  const remoteCommand = sliceBetween(
    "let response = upstream_client",
    "let cached_token_provider: Option<Arc<dyn oauth::OAuthTokenProvider>> =",
  );
  assertBefore(
    remoteCommand,
    "let response = Self::bounded_remote_context_server_command(response)?;",
    "client.build_command(",
    "remote context server responses must be bounded before command wrapping",
  );
  assertBefore(
    remoteCommand,
    "let response_env: HashMap<_, _> = bounded_context_server_env_entries(",
    "client.build_command(",
    "remote response env maps must be bounded before build_command materialization",
  );
  assertBefore(
    remoteCommand,
    "let remote_command_env: HashMap<_, _> = bounded_context_server_env_entries(",
    "env: Some(remote_command_env)",
    "remote wrapper env maps must be bounded before ContextServerCommand materialization",
  );

  const handleRpc = sliceBetween(
    "async fn handle_get_context_server_command(",
    "\n    fn resolve_project_settings<'a>(",
  );
  assertBefore(
    handleRpc,
    "bounded_context_server_env_entries(env, \"context server command environment\")",
    "Ok(proto::ContextServerCommand {",
    "context-server RPC env maps must be bounded before proto response materialization",
  );
});

test("context server store caps URL and OAuth/session payloads before keychain materialization", () => {
  assert.match(source, /const MAX_CONTEXT_SERVER_URL_BYTES: usize = 16 \* 1024;/);
  assert.match(source, /const MAX_CONTEXT_SERVER_URL_PATH_SEGMENTS: usize = 128;/);
  assert.match(source, /const MAX_CONTEXT_SERVER_OAUTH_SESSION_BYTES: usize = 64 \* 1024;/);
  assert.match(source, /const MAX_CONTEXT_SERVER_CLIENT_SECRET_BYTES: usize = 16 \* 1024;/);

  const urlGuard = sliceBetween(
    "fn ensure_context_server_url_within_limit(",
    "\nfn ensure_oauth_session_bytes_within_limit(",
  );
  assert.match(
    urlGuard,
    /server_url\s*\.path_segments\(\)\s*\.map\(\|segments\|\s*\{\s*segments\s*\.take\(MAX_CONTEXT_SERVER_URL_PATH_SEGMENTS \+ 1\)\s*\.count\(\)\s*\}\)/s,
  );
  assert.doesNotMatch(urlGuard, /collect::<Vec/);

  const fromSettings = sliceBetween(
    "pub async fn from_settings(",
    "pub type ContextServerFactory",
  );
  assert.match(fromSettings, /\) -> Result<Option<Self>> \{/);
  assert.doesNotMatch(
    fromSettings,
    /\.log_err\(\)\?/,
    "configuration-size failures should propagate to visible error state instead of disappearing",
  );
  assertBefore(
    fromSettings,
    "ensure_context_server_url_within_limit(&url)?;",
    "Ok(Some(ContextServerConfiguration::Http {",
    "HTTP context-server URLs must be bounded before configuration materialization",
  );

  const oauthFlow = sliceBetween(
    "async fn run_oauth_flow(",
    "\n    /// Store the full OAuth session",
  );
  assertBefore(
    oauthFlow,
    "ensure_context_server_url_within_limit(&discovery.resource_metadata.resource)?;",
    "oauth::canonical_server_uri",
    "discovered OAuth resource URLs must be bounded before canonical URI materialization",
  );

  const storeSession = sliceBetween(
    "async fn store_session(",
    "\n    /// Load the full OAuth session",
  );
  assertBefore(
    storeSession,
    "let json = serialize_oauth_session_within_limit(session)?;",
    ".write_credentials(",
    "OAuth sessions must be serialized through the bounded writer before keychain writes",
  );
  assert.doesNotMatch(storeSession, /serde_json::to_string\(session\)/);

  const loadSession = sliceBetween(
    "async fn load_session(",
    "\n    /// Clear the stored OAuth session",
  );
  assertBefore(
    loadSession,
    "ensure_oauth_session_bytes_within_limit(password_bytes.len())?;",
    "serde_json::from_slice(&password_bytes)",
    "OAuth session bytes must be capped before JSON parsing",
  );

  const loadClientSecret = sliceBetween(
    "async fn load_client_secret(",
    "\n    pub async fn store_client_secret(",
  );
  assertBefore(
    loadClientSecret,
    "ensure_context_server_client_secret_bytes_within_limit(secret_bytes.len())?;",
    "String::from_utf8(secret_bytes)",
    "client secret bytes must be capped before string materialization",
  );
});
