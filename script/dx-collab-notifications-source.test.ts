import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path) => readFileSync(path, "utf8");

const functionBody = (source, name) => {
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

const assertBefore = ({ body, before, after, message }) => {
  const beforeIndex = body.indexOf(before);
  const afterIndex = body.indexOf(after);
  assert.ok(beforeIndex >= 0, `missing ${before}`);
  assert.ok(afterIndex >= 0, `missing ${after}`);
  assert.ok(beforeIndex < afterIndex, message);
};

test("contact finder clamps stale selections after async result replacement", () => {
  const source = read("crates/collab_ui/src/collab_panel/contact_finder.rs");
  const clamp = functionBody(source, "clamp_selected_index_to_potential_contacts");
  const updateMatches = functionBody(source, "update_matches");

  assert.match(clamp, /self\.potential_contacts\.len\(\)\.saturating_sub\(1\)/);
  assert.match(
    clamp,
    /self\.selected_index\s*=\s*self\.selected_index\.min\(last_selectable_index\);/,
  );

  assertBefore({
    body: updateMatches,
    before: "picker.delegate.potential_contacts = potential_contacts.into();",
    after: "picker.delegate.clamp_selected_index_to_potential_contacts();",
    message: "contact finder must clamp selection immediately after replacing results",
  });
  assertBefore({
    body: updateMatches,
    before: "picker.delegate.clamp_selected_index_to_potential_contacts();",
    after: "cx.notify();",
    message: "contact finder should notify only after selection is source-clamped",
  });
});
