import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const agentDiff = readFileSync("crates/agent_ui/src/agent_diff.rs", "utf8");
const productionAgentDiff = agentDiff.slice(0, agentDiff.indexOf("\n#[cfg(test)]"));

const functionBody = (name: string) => {
  const signature = new RegExp(`\\n(?:    )?(?:pub\\s+)?fn ${name}(?:<[^>]+>)?\\(`);
  const match = signature.exec(agentDiff);
  assert.ok(match?.index, `expected ${name} in agent_diff.rs`);

  const start = match.index + 1;
  const openBrace = agentDiff.indexOf("{", start);
  assert.ok(openBrace > start, `expected ${name} to have a body`);

  let depth = 0;
  for (let index = openBrace; index < agentDiff.length; index += 1) {
    const char = agentDiff[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return agentDiff.slice(start, index + 1);
      }
    }
  }

  assert.fail(`expected ${name} body to close`);
};

const indexOfPattern = (body: string, pattern: string | RegExp) => {
  if (typeof pattern === "string") {
    return body.indexOf(pattern);
  }

  return body.match(pattern)?.index ?? -1;
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
  const beforeIndex = indexOfPattern(body, before);
  const afterIndex = indexOfPattern(body, after);
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("agent diff source hardening uses named high caps", () => {
  for (const cap of [
    "MAX_AGENT_DIFF_CHANGED_BUFFERS",
    "MAX_AGENT_DIFF_BUFFER_DIFF_HUNKS",
    "MAX_AGENT_DIFF_SELECTION_RANGES",
    "MAX_AGENT_DIFF_WORKSPACE_ITEMS",
    "MAX_AGENT_DIFF_REGISTERED_BUFFERS",
    "MAX_AGENT_DIFF_EDITORS_PER_BUFFER",
    "MAX_AGENT_DIFF_UNDO_BUFFERS",
  ]) {
    assert.match(
      productionAgentDiff,
      new RegExp(`const ${cap}: usize = [0-9_]+;`),
      `missing named cap ${cap}`,
    );
  }
});

test("bounded collection helper checks caps before vector materialization", () => {
  const collectItems = functionBody("collect_agent_diff_items");
  assertBefore({
    body: collectItems,
    before: "items.len() >= max_items",
    after: "items.push(item);",
    message: "agent diff vectors must check the cap before push materialization",
  });
  assert.match(collectItems, /log::warn!\([\s\S]*max_items/);
  assert.match(collectItems, /return None;/);

  const collectSet = functionBody("collect_agent_diff_hash_set");
  assertBefore({
    body: collectSet,
    before: "seen_items >= max_items",
    after: "items.insert(item);",
    message: "agent diff sets must check the cap before insert materialization",
  });
  assert.match(collectSet, /log::warn!\([\s\S]*max_items/);
  assert.match(collectSet, /return None;/);
});

test("changed buffers and diff rows are capped before sort and render materialization", () => {
  const updateExcerpts = functionBody("update_excerpts");
  assertBefore({
    body: updateExcerpts,
    before: /collect_agent_diff_items\(\s*changed_buffers,\s*MAX_AGENT_DIFF_CHANGED_BUFFERS/,
    after: "sorted_buffers.sort_by",
    message: "changed buffers must be capped before sorting",
  });
  assertBefore({
    body: updateExcerpts,
    before: /collect_agent_diff_hash_set\([\s\S]*MAX_AGENT_DIFF_CHANGED_BUFFERS/,
    after: "buffers_to_delete.remove",
    message: "existing diff buffer ids must be capped before removal bookkeeping",
  });
  assertBefore({
    body: updateExcerpts,
    before: /collect_agent_diff_items\([\s\S]*MAX_AGENT_DIFF_BUFFER_DIFF_HUNKS/,
    after: "editor.update_excerpts_for_path",
    message: "diff hunk rows must be capped before render excerpt materialization",
  });
  assert.match(updateExcerpts, /fail_agent_diff_materialization/);
  assert.match(updateExcerpts, /clear_materialization_warning/);
});

test("selection and action diff hunk materialization fail closed behind caps", () => {
  for (const name of ["keep_edits_in_selection", "reject_edits_in_selection"]) {
    const body = functionBody(name);
    assert.match(body, /collect_agent_diff_items\([\s\S]*MAX_AGENT_DIFF_SELECTION_RANGES/);
    assert.doesNotMatch(body, /disjoint_anchor_ranges\(\)\s*\.collect::<Vec<_>>\(\)/);
  }

  for (const name of ["keep_edits_in_ranges", "reject_edits_in_ranges"]) {
    const body = functionBody(name);
    assertBefore({
      body,
      before: /collect_agent_diff_items\([\s\S]*MAX_AGENT_DIFF_BUFFER_DIFF_HUNKS/,
      after: "update_editor_selection",
      message: `${name} must cap diff hunks before selection/action materialization`,
    });
    assert.doesNotMatch(body, /diff_hunks_in_ranges\([^)]*\)\s*\.collect::<Vec<_>>\(\)/);
  }

  const rejectRanges = functionBody("reject_edits_in_ranges");
  assertBefore({
    body: rejectRanges,
    before: "ranges_by_buffer.len() >= MAX_AGENT_DIFF_UNDO_BUFFERS",
    after: "undo_buffers.push",
    message: "undo buffer list materialization must be capped before push",
  });
});

test("workspace and single-file review fanout are capped before collecting or registering rows", () => {
  const registerWorkspace = functionBody("register_workspace");
  assertBefore({
    body: registerWorkspace,
    before: /collect_agent_diff_items\([\s\S]*workspace\.items_of_type\(cx\),\s*MAX_AGENT_DIFF_WORKSPACE_ITEMS/,
    after: "for editor in editors",
    message: "workspace items must be capped before editor registration fanout",
  });

  const registerEditor = functionBody("register_editor");
  assertBefore({
    body: registerEditor,
    before: "singleton_editors.len() >= MAX_AGENT_DIFF_REGISTERED_BUFFERS",
    after: ".entry(buffer.clone())",
    message: "registered buffer rows must be capped before hash map entry materialization",
  });
  assertBefore({
    body: registerEditor,
    before: "buffer_editors.len() >= MAX_AGENT_DIFF_EDITORS_PER_BUFFER",
    after: ".entry(weak_editor.clone())",
    message: "per-buffer editor rows must be capped before hash map entry materialization",
  });

  const updateReviewingEditors = functionBody("update_reviewing_editors");
  assertBefore({
    body: updateReviewingEditors,
    before: /collect_agent_diff_items\(\s*action_log\.read\(cx\)\.changed_buffers\(cx\),\s*MAX_AGENT_DIFF_CHANGED_BUFFERS/,
    after: "let mut unaffected = self.reviewing_editors.clone();",
    message: "single-file review changed buffers must be capped before workspace row cloning",
  });
});

test("next reviewed buffer action path is capped before project path materialization", () => {
  const reviewInActiveEditor = functionBody("review_in_active_editor");
  assertBefore({
    body: reviewInActiveEditor,
    before: /collect_agent_diff_items\(\s*changed_buffers,\s*MAX_AGENT_DIFF_CHANGED_BUFFERS/,
    after: ".project_path(cx)",
    message: "next-buffer navigation must cap changed buffers before project path/action materialization",
  });
  assert.doesNotMatch(
    reviewInActiveEditor,
    /changed_buffers\.map\(\|\(buffer, _\)\| buffer\)/,
    "review navigation should not create an uncapped mapped iterator",
  );
});
