import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const source = readFileSync("crates/agent_ui/src/model_selector.rs", "utf8");

test("model selector declares named source caps for grouped model materialization", () => {
  assert.match(source, /const MAX_MODEL_SELECTOR_MODELS: usize = 4096;/);
  assert.match(source, /const MAX_MODEL_SELECTOR_GROUPS: usize = 128;/);
  assert.match(source, /const MAX_MODEL_SELECTOR_MODELS_PER_GROUP: usize = 512;/);
  assert.match(source, /const MAX_MODEL_SELECTOR_PICKER_ENTRIES: usize = 10_000;/);
  assert.match(source, /const MAX_MODEL_SELECTOR_FUZZY_CANDIDATES: usize = 4096;/);
  assert.match(source, /const MAX_MODEL_SELECTOR_FUZZY_MATCHES: usize = 100;/);
});

test("favorites and picker rows are bounded before render materialization", () => {
  const helperStart = source.indexOf("fn capped_model_refs");
  const entriesStart = source.indexOf("fn info_list_to_picker_entries");
  const fuzzyStart = source.indexOf("\nasync fn fuzzy_search");

  assert.ok(helperStart >= 0, "expected capped model reference helper");
  assert.ok(entriesStart > helperStart, "expected entries helper after capped refs");
  assert.ok(fuzzyStart > entriesStart, "expected fuzzy search after entries helper");

  const helper = source.slice(helperStart, entriesStart);
  const entries = source.slice(entriesStart, fuzzyStart);

  assert.match(helper, /list\.iter\(\)\.take\(MAX_MODEL_SELECTOR_MODELS\)\.collect\(\)/);
  assert.match(helper, /index_map\s*\.values\(\)\s*\.take\(MAX_MODEL_SELECTOR_GROUPS\)/);
  assert.match(
    helper,
    /models\s*\.iter\(\)\s*\.take\(MAX_MODEL_SELECTOR_MODELS_PER_GROUP\)/,
  );
  assert.match(helper, /\.take\(MAX_MODEL_SELECTOR_MODELS\)\s*\.collect\(\)/);
  assert.match(entries, /let all_models = capped_model_refs\(&model_list\);/);
  assert.match(entries, /fn push_picker_entry/);
  assert.match(entries, /entries\.len\(\) >= MAX_MODEL_SELECTOR_PICKER_ENTRIES/);
  assert.match(entries, /ModelPickerEntry::ProviderHeader\(ProviderHeaderInfo \{/);
  assert.match(entries, /model_count: models\.len\(\)/);
  assert.match(entries, /collapsed_model_groups\.contains\(&group_name\)/);
  assert.match(entries, /if collapsed \{\s*continue;\s*\}/);
  assert.match(entries, /list\.into_iter\(\)\.take\(MAX_MODEL_SELECTOR_MODELS\)/);
  assert.match(
    entries,
    /index_map\s*\.into_iter\(\)\s*\.take\(MAX_MODEL_SELECTOR_GROUPS\)/,
  );
  assert.match(
    entries,
    /models\s*\.into_iter\(\)\s*\.take\(MAX_MODEL_SELECTOR_MODELS_PER_GROUP\)/,
  );
  assert.doesNotMatch(entries, /values\(\)\.flatten\(\)\.collect/);
});

test("acp model selector uses collapsible provider headers in the real picker", () => {
  assert.match(source, /AgentModelGroupName, AgentModelIcon/);
  assert.match(source, /collapsed_model_groups:\s*HashSet<AgentModelGroupName>/);
  assert.match(source, /ProviderHeader\(ProviderHeaderInfo\)/);
  assert.match(source, /struct ProviderHeaderInfo \{[\s\S]*group_name: AgentModelGroupName,[\s\S]*model_count: usize,[\s\S]*collapsed: bool,[\s\S]*\}/);
  assert.match(source, /fn toggle_model_group\(/);
  assert.match(source, /let force_model_groups_expanded = !query\.is_empty\(\);/);
  assert.match(source, /\.count\(header\.model_count\)[\s\S]*\.expanded\(!collapsed\)[\s\S]*\.on_toggle\(on_toggle\)/);
  assert.match(source, /ModelPickerEntry::ProviderHeader\(_\)[\s\S]*false/);
});

test("favorite cycling checks the next favorite before selecting it", () => {
  const cycleStart = source.indexOf("pub fn cycle_favorite_models");
  const helperStart = source.indexOf("\nfn capped_model_refs", cycleStart);

  assert.ok(cycleStart >= 0, "expected favorite cycling method");
  assert.ok(helperStart > cycleStart, "expected capped refs helper after favorite cycling");

  const cycle = source.slice(cycleStart, helperStart);

  assert.match(
    cycle,
    /let\s+Some\(next_model\)\s*=\s*favorite_models\s*\.get\(next_index\)\s*\.cloned\(\)\s*else\s*\{\s*return;\s*\};/,
  );
  assert.doesNotMatch(cycle, /favorite_models\s*\[\s*next_index\s*\]/);
});

test("fuzzy search caps candidates and grouped fanout before collection", () => {
  const fuzzyStart = source.indexOf("async fn fuzzy_search");
  const testsStart = source.indexOf("\n#[cfg(test)]", fuzzyStart);

  assert.ok(fuzzyStart >= 0, "expected fuzzy search helper");
  assert.ok(testsStart > fuzzyStart, "expected tests after fuzzy search");

  const fuzzy = source.slice(fuzzyStart, testsStart);
  const listStart = fuzzy.indexOf("async fn fuzzy_search_list");
  const candidates = fuzzy.indexOf("let candidates = model_list");
  const grouped = fuzzy.indexOf("AgentModelList::Grouped");
  const joinAll = fuzzy.indexOf("futures::future::join_all", grouped);

  assert.ok(listStart >= 0, "expected per-list fuzzy helper");
  assert.ok(candidates > listStart, "expected fuzzy candidates after helper start");
  assert.ok(grouped > candidates, "expected grouped branch after fuzzy list helper");
  assert.ok(joinAll > grouped, "expected join_all in grouped branch");
  assert.match(
    fuzzy,
    /let model_list = model_list\s*\.into_iter\(\)\s*\.take\(MAX_MODEL_SELECTOR_FUZZY_CANDIDATES\)\s*\.collect::<Vec<_>>\(\);/,
  );
  assert.ok(
    fuzzy.indexOf("take(MAX_MODEL_SELECTOR_FUZZY_CANDIDATES)") < candidates,
    "fuzzy candidate list must be capped before StringMatchCandidate collection",
  );
  assert.match(fuzzy, /MAX_MODEL_SELECTOR_FUZZY_MATCHES/);
  assert.match(
    fuzzy,
    /join_all\(\s*index_map\s*\.into_iter\(\)\s*\.take\(MAX_MODEL_SELECTOR_GROUPS\)/,
  );
  const fanout = fuzzy.slice(joinAll);
  assert.ok(
    fanout.indexOf("take(MAX_MODEL_SELECTOR_GROUPS)") <
      fanout.indexOf("fuzzy_search_list(models, &query, executor.clone())"),
    "group fanout must be capped before provider search futures are created",
  );
});
