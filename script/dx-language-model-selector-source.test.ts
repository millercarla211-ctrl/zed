import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/agent_ui/src/language_model_selector.rs";

const productionSource = (source: string) =>
  source.split(/\r?\n#\[cfg\(test\)\]\r?\nmod tests\s*\{/)[0] ?? source;

const source = productionSource(readFileSync(sourcePath, "utf8"));

function sliceBetween(haystack: string, start: string, end: string): string {
  const startIndex = haystack.indexOf(start);
  assert.notEqual(startIndex, -1, `expected ${start}`);
  const endIndex = haystack.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `expected ${end} after ${start}`);
  return haystack.slice(startIndex, endIndex);
}

function assertBefore(
  haystack: string,
  before: string,
  after: string,
  message: string,
) {
  const beforeIndex = haystack.indexOf(before);
  const afterIndex = haystack.indexOf(after);
  assert.notEqual(beforeIndex, -1, `expected ${before}`);
  assert.notEqual(afterIndex, -1, `expected ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
}

test("language model selector declares named candidate and render caps", () => {
  assert.match(source, /const MAX_SELECTOR_VISIBLE_PROVIDERS: usize = \d+;/);
  assert.match(source, /const MAX_SELECTOR_FAVORITE_SETTINGS: usize = \d+;/);
  assert.match(source, /const MAX_SELECTOR_FAVORITE_MODELS: usize = \d+;/);
  assert.match(
    source,
    /const MAX_SELECTOR_RECOMMENDED_MODELS_PER_PROVIDER: usize = \d+;/,
  );
  assert.match(source, /const MAX_SELECTOR_MODELS_PER_PROVIDER: usize = \d+;/);
  assert.match(source, /const MAX_SELECTOR_RECOMMENDED_MODELS: usize = \d+;/);
  assert.match(source, /const MAX_SELECTOR_MODELS: usize = \d+;/);
  assert.match(source, /const MAX_SELECTOR_MATCH_CANDIDATES: usize = MAX_SELECTOR_MODELS;/);
  assert.match(
    source,
    /const MAX_SELECTOR_EXACT_MATCHES: usize = MAX_SELECTOR_RECOMMENDED_MODELS;/,
  );
  assert.match(source, /const MAX_SELECTOR_FUZZY_MATCHES: usize = 100;/);
  assert.match(source, /const MAX_SELECTOR_RENDER_ENTRIES: usize =/);
});

test("all model discovery caps providers, favorites, and provider models before collection", () => {
  const allModels = sliceBetween(
    source,
    "fn all_models(cx: &App) -> GroupedModels {",
    "type FavoritesIndex",
  );
  const providers = sliceBetween(
    allModels,
    "let providers = lm_registry",
    "let mut favorites_index",
  );
  const favorites = sliceBetween(
    allModels,
    "for sel in AgentSettings::get_global(cx)",
    "let recommended = providers",
  );
  const recommended = sliceBetween(
    allModels,
    "let recommended = providers",
    "let all = providers",
  );
  const all = sliceBetween(
    allModels,
    "let all = providers",
    "GroupedModels::new",
  );

  assertBefore(
    providers,
    ".take(MAX_SELECTOR_VISIBLE_PROVIDERS)",
    ".collect::<Vec<_>>()",
    "visible providers must be capped before selector provider-vector materialization",
  );
  assertBefore(
    favorites,
    ".take(MAX_SELECTOR_FAVORITE_SETTINGS)",
    ".entry(sel.provider",
    "favorite settings must be capped before insertion into the selector favorite index",
  );
  assertBefore(
    recommended,
    ".take(MAX_SELECTOR_RECOMMENDED_MODELS_PER_PROVIDER)",
    ".map(|model| ModelInfo::new",
    "provider recommended models must be capped before becoming selector rows",
  );
  assertBefore(
    recommended,
    ".take(MAX_SELECTOR_RECOMMENDED_MODELS)",
    ".collect()",
    "recommended models must be capped before collection",
  );
  assertBefore(
    all,
    ".take(MAX_SELECTOR_MODELS_PER_PROVIDER)",
    ".map(|model| ModelInfo::new",
    "provider model lists must be capped before becoming selector rows",
  );
  assertBefore(
    all,
    ".take(MAX_SELECTOR_MODELS)",
    ".collect()",
    "all model rows must be capped before collection",
  );
});

test("grouped model buckets and render entries are capped before materialization", () => {
  const groupedNew = sliceBetween(
    source,
    "pub fn new(all: Vec<ModelInfo>, recommended: Vec<ModelInfo>) -> Self {",
    "fn entries(&self) -> Vec<LanguageModelPickerEntry> {",
  );
  const boundedAll = sliceBetween(
    groupedNew,
    "let all = all",
    "let recommended = recommended",
  );
  const boundedRecommended = sliceBetween(
    groupedNew,
    "let recommended = recommended",
    "let favorites = all",
  );
  const entries = sliceBetween(
    source,
    "fn entries(&self) -> Vec<LanguageModelPickerEntry> {",
    "enum LanguageModelPickerEntry",
  );
  const pushHelper = sliceBetween(
    source,
    "fn push_picker_entry(",
    "struct ModelMatcher",
  );

  assertBefore(
    boundedAll,
    ".take(MAX_SELECTOR_MODELS)",
    ".collect::<Vec<_>>()",
    "grouped all-model input must be capped before regrouping",
  );
  assertBefore(
    boundedRecommended,
    ".take(MAX_SELECTOR_RECOMMENDED_MODELS)",
    ".collect::<Vec<_>>()",
    "grouped recommended input must be capped before storage",
  );
  assertBefore(
    groupedNew,
    ".take(MAX_SELECTOR_FAVORITE_MODELS)",
    ".collect()",
    "favorite rows must be capped before collection",
  );
  assert.match(groupedNew, /models\.len\(\) < MAX_SELECTOR_MODELS_PER_PROVIDER/);
  assert.match(groupedNew, /all_by_provider\.len\(\) < MAX_SELECTOR_VISIBLE_PROVIDERS/);
  assert.match(entries, /self\.favorites\.iter\(\)\.take\(MAX_SELECTOR_FAVORITE_MODELS\)/);
  assert.match(
    entries,
    /self\s*\.recommended\s*\.iter\(\)\s*\.take\(MAX_SELECTOR_RECOMMENDED_MODELS\)/,
  );
  assert.match(entries, /self\.all\.values\(\)\.take\(MAX_SELECTOR_VISIBLE_PROVIDERS\)/);
  assert.match(entries, /models\.iter\(\)\.take\(MAX_SELECTOR_MODELS_PER_PROVIDER\)/);
  assertBefore(
    pushHelper,
    "entries.len() >= MAX_SELECTOR_RENDER_ENTRIES",
    "entries.push(entry)",
    "render entries must check the cap before pushing list rows",
  );
});

test("favorite cycling uses checked favorite lookup", () => {
  const cycleFavoriteModels = sliceBetween(
    source,
    "pub fn cycle_favorite_models(",
    "struct GroupedModels",
  );

  assert.doesNotMatch(
    cycleFavoriteModels,
    /\.favorites\s*\[\s*next_index\s*\]/,
    "favorite cycling must not direct-index favorites by next_index",
  );
  assert.match(
    cycleFavoriteModels,
    /\.favorites\s*\.get\(\s*next_index\s*\)/,
    "favorite cycling must use a checked favorites lookup before cloning the next model",
  );
});

test("model matcher caps searchable candidates and result vectors", () => {
  const matcher = sliceBetween(source, "impl ModelMatcher {", "impl PickerDelegate");
  const matcherNew = sliceBetween(matcher, "fn new(", "pub fn fuzzy_search");
  const fuzzySearch = sliceBetween(
    matcher,
    "pub fn fuzzy_search(&self, query: &str) -> Vec<ModelInfo> {",
    "pub fn exact_search",
  );
  const exactSearch = sliceBetween(
    matcher,
    "pub fn exact_search(&self, query: &str) -> Vec<ModelInfo> {",
    "fn make_match_candidates",
  );
  const matchCandidates = sliceBetween(
    source,
    "fn make_match_candidates(model_infos: &[ModelInfo]) -> Vec<StringMatchCandidate> {",
    "impl PickerDelegate",
  );

  assertBefore(
    matcherNew,
    ".take(MAX_SELECTOR_MATCH_CANDIDATES)",
    "Self::make_match_candidates(&models)",
    "matcher inputs must be capped before fuzzy candidate construction",
  );
  assertBefore(
    fuzzySearch,
    "MAX_SELECTOR_FUZZY_MATCHES",
    "self.bg_executor.clone()",
    "fuzzy search must pass a named result cap to match_strings",
  );
  assertBefore(
    fuzzySearch,
    ".take(MAX_SELECTOR_FUZZY_MATCHES)",
    ".collect()",
    "fuzzy result vectors must be capped before collection",
  );
  assertBefore(
    exactSearch,
    "let query = query.to_lowercase();",
    ".filter(|m|",
    "exact search should materialize the lowercase query once before filtering",
  );
  assertBefore(
    exactSearch,
    ".take(MAX_SELECTOR_EXACT_MATCHES)",
    ".collect::<Vec<_>>()",
    "exact match rows must be capped before collection",
  );
  assertBefore(
    matchCandidates,
    ".take(MAX_SELECTOR_MATCH_CANDIDATES)",
    "StringMatchCandidate::new",
    "match candidates must be capped before candidate allocation",
  );
  assertBefore(
    matchCandidates,
    ".take(MAX_SELECTOR_MATCH_CANDIDATES)",
    ".collect::<Vec<_>>()",
    "match candidates must be capped before collection",
  );
});

test("match updates cap configured providers and filtered model vectors before matching", () => {
  const updateMatches = sliceBetween(
    source,
    "fn update_matches(",
    "fn confirm(",
  );
  const configuredProviders = sliceBetween(
    updateMatches,
    "let configured_provider_ids = language_model_registry",
    "let recommended_models = all_models",
  );
  const recommended = sliceBetween(
    updateMatches,
    "let recommended_models = all_models",
    "let available_models = all_models",
  );
  const available = sliceBetween(
    updateMatches,
    "let available_models = all_models",
    "let matcher_rec =",
  );

  assertBefore(
    configuredProviders,
    ".take(MAX_SELECTOR_VISIBLE_PROVIDERS)",
    ".collect::<HashSet<_>>()",
    "configured providers must be capped before provider-id set materialization",
  );
  assertBefore(
    recommended,
    ".take(MAX_SELECTOR_RECOMMENDED_MODELS)",
    ".collect::<Vec<_>>()",
    "filtered recommended models must be capped before matcher input collection",
  );
  assertBefore(
    available,
    ".take(MAX_SELECTOR_MODELS)",
    ".collect::<Vec<_>>()",
    "filtered available models must be capped before matcher input collection",
  );
  assertBefore(
    updateMatches,
    "ModelMatcher::new(recommended_models",
    "matcher_rec.exact_search(&query)",
    "recommended matcher should consume the capped model vector",
  );
  assertBefore(
    updateMatches,
    "ModelMatcher::new(available_models",
    "matcher_all.fuzzy_search(&query)",
    "fuzzy matcher should consume the capped model vector",
  );
});

test("language model selector source guard is focused on production source", () => {
  assert.equal(sourcePath, "crates/agent_ui/src/language_model_selector.rs");
  assert.doesNotMatch(sourcePath, /test/i);
  assert.doesNotMatch(
    source,
    /#\[cfg\(test\)\]/,
    "source guard should only inspect production selector code",
  );
});
