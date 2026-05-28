import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const sourcePath = "crates/recent_projects/src/sidebar_recent_projects.rs";
const source = readFileSync(sourcePath, "utf8").replace(/\r\n/g, "\n");

const functionBody = (name: string) => {
  const start = source.search(new RegExp(`fn\\s+${name}\\s*\\(`));
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = source.indexOf("{", start);
  assert.ok(bodyStart > start, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(start, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
};

const assertBefore = ({
  body,
  before,
  after,
  message,
}: {
  body: string;
  before: string | RegExp;
  after: string | RegExp;
  message: string;
}) => {
  const beforeIndex =
    typeof before === "string" ? body.indexOf(before) : body.match(before)?.index ?? -1;
  const afterIndex =
    typeof after === "string" ? body.indexOf(after) : body.match(after)?.index ?? -1;
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("sidebar recent projects caps fuzzy candidate and result materialization", () => {
  const updateMatches = functionBody("update_matches");

  assert.match(source, /const MAX_SIDEBAR_RECENT_PROJECT_CANDIDATES: usize = 2_000;/);
  assert.match(source, /const MAX_SIDEBAR_RECENT_PROJECT_MATCHES: usize = 100;/);
  assert.match(source, /const MAX_SIDEBAR_RECENT_PROJECT_CANDIDATE_PATHS: usize = 64;/);
  assert.match(
    source,
    /fn sidebar_recent_project_candidate_string\(workspace: &RecentWorkspace\) -> String/,
  );

  assertBefore({
    body: updateMatches,
    before: ".take(MAX_SIDEBAR_RECENT_PROJECT_CANDIDATES)",
    after: "StringMatchCandidate::new(id, &combined_string)",
    message: "workspace filtering must be capped before fuzzy candidate strings are built",
  });
  assertBefore({
    body: updateMatches,
    before: ".take(MAX_SIDEBAR_RECENT_PROJECT_MATCHES)",
    after: "StringMatch {\n                    candidate_id",
    message: "empty recent-project queries must cap result rows before collection",
  });
  assert.match(
    updateMatches,
    /match_strings\(\s*&candidates,\s*query,\s*case,\s*fuzzy_nucleo::LengthPenalty::On,\s*MAX_SIDEBAR_RECENT_PROJECT_MATCHES,\s*\)/,
    "fuzzy recent-project queries must use the named result cap",
  );

  const candidateString = functionBody("sidebar_recent_project_candidate_string");
  assertBefore({
    body: candidateString,
    before: ".take(MAX_SIDEBAR_RECENT_PROJECT_CANDIDATE_PATHS)",
    after: ".collect::<Vec<_>>()",
    message: "candidate path labels must be capped before candidate-string joins",
  });
});

test("sidebar recent projects clamps stale selection after match updates", () => {
  const updateMatches = functionBody("update_matches");
  const setSelectedIndex = functionBody("set_selected_index");
  const clampSelectedIndex = functionBody("clamp_selected_index");

  assert.match(clampSelectedIndex, /self\.filtered_workspaces\.len\(\)\.checked_sub\(1\)/);
  assert.match(clampSelectedIndex, /self\.selected_index = self\.selected_index\.min\(max_index\);/);
  assert.match(clampSelectedIndex, /None => self\.selected_index = 0,/);

  assertBefore({
    body: updateMatches,
    before: "self.filtered_workspaces = if is_empty_query",
    after: "self.clamp_selected_index();",
    message: "match replacement must happen before stale selection is clamped",
  });
  assertBefore({
    body: setSelectedIndex,
    before: "self.selected_index = ix;",
    after: "self.clamp_selected_index();",
    message: "picker selection writes must clamp before later workspace materialization",
  });
});

test("sidebar recent projects caps rendered path labels before tooltip and highlight joins", () => {
  const renderMatch = functionBody("render_match");
  const cappedPaths = functionBody("capped_sidebar_recent_project_paths");

  assert.match(source, /const MAX_SIDEBAR_RECENT_PROJECT_RENDERED_PATHS: usize = 16;/);
  assert.match(source, /fn sidebar_recent_project_tooltip_path\(/);

  assertBefore({
    body: cappedPaths,
    before: ".take(MAX_SIDEBAR_RECENT_PROJECT_RENDERED_PATHS.saturating_add(1))",
    after: "paths.truncate(MAX_SIDEBAR_RECENT_PROJECT_RENDERED_PATHS);",
    message: "rendered identity paths must be capped before labels are returned",
  });
  assertBefore({
    body: renderMatch,
    before: "let (rendered_paths, paths_truncated) = capped_sidebar_recent_project_paths(workspace);",
    after: "let tooltip_path = sidebar_recent_project_tooltip_path(",
    message: "tooltip text must be built from capped rendered paths",
  });
  assertBefore({
    body: renderMatch,
    before: /rendered_paths\s*\.iter\(\)/,
    after: "HighlightedMatch::join(match_labels.into_iter().flatten(), \", \")",
    message: "highlight labels must be joined from capped rendered paths",
  });
});

test("recent projects source guard stays in worker-owned files", () => {
  assert.equal(sourcePath, "crates/recent_projects/src/sidebar_recent_projects.rs");
});
