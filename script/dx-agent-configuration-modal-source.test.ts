import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const providerModal = readFileSync(
  "crates/agent_ui/src/agent_configuration/add_llm_provider_modal.rs",
  "utf8",
);
const contextServerModal = readFileSync(
  "crates/agent_ui/src/agent_configuration/configure_context_server_modal.rs",
  "utf8",
);
const profilesModal = readFileSync(
  "crates/agent_ui/src/agent_configuration/manage_profiles_modal.rs",
  "utf8",
);

function sliceBetween(source, startNeedle, endNeedle) {
  const start = source.indexOf(startNeedle);
  assert.notEqual(start, -1, `expected ${startNeedle}`);

  const end = source.indexOf(endNeedle, start);
  assert.ok(end > start, `expected ${endNeedle} after ${startNeedle}`);

  return source.slice(start, end);
}

function regexIndex(source, regex, message) {
  const index = source.search(regex);
  assert.notEqual(index, -1, message);
  return index;
}

test("provider modal bounds text before settings and keychain mutations", () => {
  assert.match(providerModal, /const MAX_PROVIDER_NAME_INPUT_BYTES: usize = 1024;/);
  assert.match(providerModal, /const MAX_PROVIDER_API_URL_INPUT_BYTES: usize = 4096;/);
  assert.match(providerModal, /const MAX_PROVIDER_API_KEY_INPUT_BYTES: usize = 16 \* 1024;/);
  assert.match(providerModal, /fn input_text_within_limit/);
  assert.match(providerModal, /fn ensure_modal_text_within_limit/);

  const saveProvider = sliceBetween(
    providerModal,
    "fn save_provider_to_settings(",
    "\npub struct AddLlmProviderModal",
  );
  const providerNameGuard = regexIndex(
    saveProvider,
    /let provider_name =\s*input_text_within_limit\(&input\.provider_name, PROVIDER_NAME_INPUT_LIMIT, cx\)\?;/,
    "expected provider name guard",
  );
  const apiUrlGuard = regexIndex(
    saveProvider,
    /let api_url =\s*input_text_within_limit\(&input\.api_url, PROVIDER_API_URL_INPUT_LIMIT, cx\)\?;/,
    "expected API URL guard",
  );
  const apiKeyGuard = regexIndex(
    saveProvider,
    /let api_key =\s*input_text_within_limit\(&input\.api_key, PROVIDER_API_KEY_INPUT_LIMIT, cx\)\?;/,
    "expected API key guard",
  );
  assert.ok(
    providerNameGuard < saveProvider.indexOf("LanguageModelRegistry::read_global"),
    "provider name size guard must run before provider registry lookup",
  );
  assert.ok(
    apiUrlGuard < saveProvider.indexOf("cx.write_credentials"),
    "API URL size guard must run before keychain/settings mutation",
  );
  assert.ok(
    apiKeyGuard < saveProvider.indexOf("cx.write_credentials"),
    "API key size guard must run before keychain mutation",
  );
});

test("provider model fields are size-checked before parsing numeric text", () => {
  assert.match(providerModal, /const MAX_MODEL_NAME_INPUT_BYTES: usize = 1024;/);
  assert.match(providerModal, /const MAX_MODEL_TOKEN_INPUT_BYTES: usize = 64;/);

  const parseModel = sliceBetween(
    providerModal,
    "fn parse(&self, cx: &App) -> Result<AvailableModel, SharedString> {",
    "\nfn save_provider_to_settings",
  );
  const nameGuard =
    "let name = input_text_within_limit(&self.name, MODEL_NAME_INPUT_LIMIT, cx)?;";
  const maxCompletionGuard = regexIndex(
    parseModel,
    /let max_completion_tokens =\s*input_text_within_limit\(/,
    "expected max completion token guard",
  );
  const maxOutputGuard = regexIndex(
    parseModel,
    /let max_output_tokens =\s*input_text_within_limit\(/,
    "expected max output token guard",
  );
  const maxTokensGuard = regexIndex(
    parseModel,
    /let max_tokens =\s*input_text_within_limit\(/,
    "expected max token guard",
  );
  const maxCompletionParse = regexIndex(
    parseModel,
    /max_completion_tokens\s*\.parse::<u64>\(\)/,
    "expected max completion token parse",
  );
  const maxOutputParse = regexIndex(
    parseModel,
    /max_output_tokens\s*\.parse::<u64>\(\)/,
    "expected max output token parse",
  );
  const maxTokensParse = regexIndex(
    parseModel,
    /max_tokens:\s*max_tokens\s*\.parse::<u64>\(\)/,
    "expected max token parse",
  );

  assert.match(parseModel, new RegExp(nameGuard.replace(/[().?&]/g, "\\$&")));
  assert.ok(
    maxCompletionGuard < maxCompletionParse,
    "max completion token text must be size-checked before parsing",
  );
  assert.ok(
    maxOutputGuard < maxOutputParse,
    "max output token text must be size-checked before parsing",
  );
  assert.ok(
    maxTokensGuard < maxTokensParse,
    "max token text must be size-checked before parsing",
  );
});

test("context server editor text is capped before parse and settings mutation", () => {
  assert.match(
    contextServerModal,
    /const MAX_CONTEXT_SERVER_CONFIGURATION_INPUT_BYTES: usize = 256 \* 1024;/,
  );
  assert.match(contextServerModal, /fn configuration_editor_text/);
  assert.match(contextServerModal, /fn ensure_editor_snapshot_within_limit/);

  const output = sliceBetween(
    contextServerModal,
    "fn output(&self, cx: &mut App) -> Result<(ContextServerId, ContextServerSettings)> {",
    "\nfn context_server_input",
  );
  const textGuard =
    /let text = configuration_editor_text\(\s*editor\.read\(cx\),\s*CONTEXT_SERVER_CONFIGURATION_INPUT_LIMIT,\s*cx,\s*\)\?;/;
  const firstTextGuard = output.search(textGuard);
  const secondTextGuardOffset = output.slice(firstTextGuard + 1).search(textGuard);
  const secondTextGuard = secondTextGuardOffset === -1
    ? -1
    : firstTextGuard + 1 + secondTextGuardOffset;

  assert.notEqual(firstTextGuard, -1, "expected guarded context-server editor extraction");
  assert.notEqual(
    secondTextGuard,
    -1,
    "expected guarded extension context-server editor extraction",
  );
  assert.ok(
    firstTextGuard < output.indexOf("parse_http_input(&text)"),
    "HTTP context-server text must be capped before serde parsing",
  );
  assert.ok(
    firstTextGuard < output.indexOf("parse_input(&text)"),
    "stdio context-server text must be capped before serde parsing",
  );
  assert.ok(
    secondTextGuard < output.indexOf("serde_json_lenient::from_str::<serde_json::Value>(&text)"),
    "extension context-server text must be capped before serde parsing",
  );

  const confirm = sliceBetween(
    contextServerModal,
    "fn confirm(&mut self, _: &menu::Confirm, cx: &mut Context<Self>) {",
    "\n    fn cancel(&mut self",
  );
  assert.ok(
    confirm.indexOf("self.source.output(cx)") <
      confirm.indexOf("update_settings_file(fs.clone(), cx, move |current, _|"),
    "context-server settings mutation must only happen after guarded output extraction",
  );
});

test("context server client secret is capped before auth submission", () => {
  assert.match(
    contextServerModal,
    /const MAX_CONTEXT_SERVER_CLIENT_SECRET_INPUT_BYTES: usize = 16 \* 1024;/,
  );
  assert.match(contextServerModal, /fn client_secret_editor_text/);

  const submitSecret = sliceBetween(
    contextServerModal,
    "fn submit_client_secret(&mut self, server_id: ContextServerId, cx: &mut Context<Self>) {",
    "\n    fn await_auth_outcome",
  );
  const secretGuard =
    "let secret = match client_secret_editor_text(self.secret_editor.read(cx), cx) {";

  assert.match(submitSecret, new RegExp(secretGuard.replace(/[().?]/g, "\\$&")));
  assert.ok(
    submitSecret.indexOf(secretGuard) < submitSecret.indexOf("store.submit_client_secret"),
    "client secret guard must run before handing the value to the store",
  );
  assert.match(submitSecret, /State::ClientSecretRequired/);
});

test("profile names are capped before profile creation", () => {
  assert.match(profilesModal, /const MAX_AGENT_PROFILE_NAME_INPUT_BYTES: usize = 1024;/);
  assert.match(profilesModal, /fn profile_name_editor_text/);
  assert.match(profilesModal, /new_profile_error: Option<SharedString>/);

  const confirm = sliceBetween(
    profilesModal,
    "fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {",
    "\n    fn delete_profile",
  );
  const profileNameGuard =
    "let name = match profile_name_editor_text(mode.name_editor.read(cx), cx) {";

  assert.match(confirm, new RegExp(profileNameGuard.replace(/[().?]/g, "\\$&")));
  assert.ok(
    confirm.indexOf(profileNameGuard) < confirm.indexOf("AgentProfile::create"),
    "profile name guard must run before handing text to AgentProfile::create",
  );

  const renderNewProfile = sliceBetween(
    profilesModal,
    "fn render_new_profile(",
    "\n    fn render_view_profile",
  );
  assert.match(renderNewProfile, /when_some\(mode\.new_profile_error/);
  assert.match(renderNewProfile, /IconName::Warning/);
});
