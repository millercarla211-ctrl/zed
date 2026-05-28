import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const functionBody = (source: string, name: string) => {
  const start = source.indexOf(`fn ${name}(`);
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

const indexOfPattern = (source: string, pattern: string | RegExp) => {
  if (typeof pattern === "string") {
    return source.indexOf(pattern);
  }

  return source.match(pattern)?.index ?? -1;
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

const assertLspAggregationCaps = ({
  source,
  functionName,
  capName,
  localTaskName,
}: {
  source: string;
  functionName: string;
  capName: string;
  localTaskName: string;
}) => {
  const body = functionBody(source, functionName);
  const localStart = body.indexOf(`let ${localTaskName} =`);
  assert.ok(localStart >= 0, `expected ${localTaskName} local request path`);

  const remote = body.slice(0, localStart);
  const local = body.slice(localStart);

  assert.match(
    remote,
    new RegExp(
      `\\.payload\\s*\\.into_iter\\(\\)\\s*\\.take\\(${capName}\\)\\s*\\.map\\(\\|response\\|`,
    ),
  );
  assertBefore({
    body: remote,
    before: new RegExp(`\\.take\\(${capName}\\)`),
    after: /join_all\(response_tasks\)/,
    message: `${functionName} must cap remote response tasks before join_all`,
  });
  assert.doesNotMatch(
    remote,
    /responses\.payload\s*\.into_iter\(\)\s*\.map\(\|response\|/,
  );

  assertBefore({
    body: local,
    before: new RegExp(`\\.take\\(${capName}\\)`),
    after: /\.collect\(\)/,
    message: `${functionName} must cap local response collection before collect`,
  });
  assert.doesNotMatch(
    local,
    new RegExp(`${localTaskName}\\.await\\s*\\.into_iter\\(\\)\\s*\\.collect\\(\\)`),
  );
};

test("document link aggregation caps LSP responses before joining or collecting", () => {
  const source = read("crates/project/src/lsp_store/document_links.rs");

  assert.match(source, /const MAX_DOCUMENT_LINK_LSP_RESPONSES: usize = 64;/);
  assert.match(
    source,
    /const MAX_DOCUMENT_LINKS_PER_LSP_RESPONSE: usize = 10_000;/,
  );
  assertLspAggregationCaps({
    source,
    functionName: "fetch_document_links_for_buffer",
    capName: "MAX_DOCUMENT_LINK_LSP_RESPONSES",
    localTaskName: "links_task",
  });

  const fetchBody = functionBody(source, "fetch_document_links_for_buffer");
  const localLinksBody = fetchBody.slice(fetchBody.indexOf("let links_task ="));
  assertBefore({
    body: fetchBody,
    before: "cap_document_links_for_response(server_id, links)",
    after: ".collect::<HashMap<_, _>>()",
    message: "remote document links must be capped per server before response-map collection",
  });
  assertBefore({
    body: localLinksBody,
    before: "cap_document_links_for_response(server_id, links)",
    after: ".collect(),",
    message: "local document links must be capped per server before response-map collection",
  });

  const updateBody = functionBody(source, "fetch_document_links");
  assertBefore({
    body: updateBody,
    before: "let server_links =\n                                cap_document_links_for_response(server_id, server_links);",
    after: "by_id.reserve(server_links.len());",
    message: "document links must be capped before reserving and inserting cached link rows",
  });
});

test("document symbol aggregation caps LSP responses before joining or collecting", () => {
  const source = read("crates/project/src/lsp_store/document_symbols.rs");

  assert.match(source, /const MAX_DOCUMENT_SYMBOL_LSP_RESPONSES: usize = 64;/);
  assert.match(
    source,
    /const MAX_DOCUMENT_SYMBOL_TREES_PER_LSP_RESPONSE: usize = 4_096;/,
  );
  assert.match(
    source,
    /const MAX_DOCUMENT_SYMBOL_OUTLINE_ITEMS_PER_RESPONSE: usize = 20_000;/,
  );
  assertLspAggregationCaps({
    source,
    functionName: "fetch_document_symbols_for_buffer",
    capName: "MAX_DOCUMENT_SYMBOL_LSP_RESPONSES",
    localTaskName: "symbols_task",
  });

  const fetchBody = functionBody(source, "fetch_document_symbols_for_buffer");
  const localSymbolsBody = fetchBody.slice(fetchBody.indexOf("let symbols_task ="));
  assertBefore({
    body: fetchBody,
    before: "cap_document_symbol_trees_for_response(server_id, symbols)",
    after: ".collect::<HashMap<_, _>>()",
    message: "remote document symbols must be capped per server before response-map collection",
  });
  assertBefore({
    body: localSymbolsBody,
    before: "cap_document_symbol_trees_for_response(server_id, symbols)",
    after: ".collect(),",
    message: "local document symbols must be capped per server before response-map collection",
  });

  const updateBody = functionBody(source, "fetch_document_symbols");
  assertBefore({
    body: updateBody,
    before: "let truncated = flatten_document_symbols(",
    after: /doc_symbols\s*\.symbols/,
    message: "document symbols must be outline-capped before dedupe/sort collection",
  });

  const flattenBody = functionBody(source, "flatten_document_symbols");
  assertBefore({
    body: flattenBody,
    before: "output.len() >= MAX_DOCUMENT_SYMBOL_OUTLINE_ITEMS_PER_RESPONSE",
    after: "output.push(OutlineItem",
    message: "document symbol flattening must check the output cap before pushing outline rows",
  });
});
