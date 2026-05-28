import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const source = readFileSync("crates/zed/src/zed/open_listener.rs", "utf8");

const functionBody = (bodySource: string, name: string) => {
  const start = bodySource.indexOf(`fn ${name}(`);
  assert.ok(start >= 0, `expected ${name}`);

  const bodyStart = bodySource.indexOf("{", start);
  assert.ok(bodyStart > start, `expected ${name} body`);

  let depth = 0;
  for (let index = bodyStart; index < bodySource.length; index += 1) {
    const char = bodySource[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return bodySource.slice(start, index + 1);
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

test("open listener validates raw url and path strings before request materialization", () => {
  assert.match(source, /const MAX_OPEN_REQUEST_URLS: usize = 1024;/);
  assert.match(source, /const MAX_OPEN_REQUEST_PATHS: usize = 4096;/);
  assert.match(source, /const MAX_OPEN_REQUEST_DIFF_PAIRS: usize = 2048;/);
  assert.match(source, /const MAX_OPEN_REQUEST_PATH_BYTES: usize = 16 \* 1024;/);
  assert.match(source, /const MAX_OPEN_REQUEST_URL_BYTES: usize = 128 \* 1024;/);
  assert.match(
    source,
    /const MAX_OPEN_REQUEST_TOTAL_PATH_BYTES: usize = 16 \* 1024 \* 1024;/,
  );
  assert.match(
    source,
    /const MAX_OPEN_REQUEST_TOTAL_URL_BYTES: usize = 16 \* 1024 \* 1024;/,
  );

  const parse = functionBody(source, "parse");
  assertBefore({
    body: parse,
    before: "validate_open_request_urls(&urls)?;",
    after: "for url in urls",
    message: "raw URLs must be count and byte capped before URL fanout parsing",
  });
  assertBefore({
    body: parse,
    before: "validate_open_request_diff_paths(&diff_paths)?;",
    after: "this.diff_paths = diff_paths;",
    message: "diff paths must be capped before request storage",
  });

  const parseFilePath = functionBody(source, "parse_file_path");
  assert.match(parseFilePath, /-> Result<\(\)>/);
  assert.match(parseFilePath, /self\.push_open_path\(decoded\.into_owned\(\)\)\?;/);
  assert.doesNotMatch(parseFilePath, /self\.open_paths\.push/);

  const pushOpenPath = functionBody(source, "push_open_path");
  assertBefore({
    body: pushOpenPath,
    before: "self.open_paths.len() < MAX_OPEN_REQUEST_PATHS",
    after: "self.open_paths.push(path);",
    message: "parsed file paths must be count capped before storing",
  });
  assertBefore({
    body: pushOpenPath,
    before: "path.len() <= MAX_OPEN_REQUEST_PATH_BYTES",
    after: "self.open_paths.push(path);",
    message: "parsed file paths must be byte capped before storing",
  });
});

test("cli open paths are capped before PathBuf collections and is_dir fanout", () => {
  const handleCliConnection = functionBody(source, "handle_cli_connection");
  assertBefore({
    body: handleCliConnection,
    before: "validate_open_request_materialization(&paths, &diff_paths)",
    after: "resolve_open_behavior(",
    message: "CLI paths must be validated before default-open behavior inspection",
  });
  assertBefore({
    body: handleCliConnection,
    before: "validate_open_request_materialization(&paths, &diff_paths)",
    after: "open_workspaces(",
    message: "CLI paths must be validated before workspace opening",
  });

  const resolveOpenBehavior = functionBody(source, "resolve_open_behavior");
  assert.match(
    source,
    /const MAX_OPEN_BEHAVIOR_PATHBUFS: usize = MAX_OPEN_REQUEST_PATHS;/,
  );
  assert.match(
    source,
    /const MAX_OPEN_BEHAVIOR_IS_DIR_FANOUT: usize = MAX_OPEN_REQUEST_PATHS;/,
  );
  assertBefore({
    body: resolveOpenBehavior,
    before: ".take(MAX_OPEN_BEHAVIOR_PATHBUFS)",
    after: ".collect();",
    message: "open behavior pathbufs must be capped before collection",
  });
  assertBefore({
    body: resolveOpenBehavior,
    before: ".take(MAX_OPEN_BEHAVIOR_IS_DIR_FANOUT)",
    after: ".map(|p| app_state.fs.is_dir(Path::new(p)))",
    message: "is_dir fanout must be capped before futures are materialized",
  });
  assert.doesNotMatch(
    resolveOpenBehavior,
    /join_all\(\s*paths\s*\.iter\(\)\s*\.map\(\|p\| app_state\.fs\.is_dir/,
  );
});

test("workspace path and item materialization stays bounded before collections", () => {
  const openWorkspaces = functionBody(source, "open_workspaces");
  assertBefore({
    body: openWorkspaces,
    before: "validate_open_request_materialization(&paths, &diff_paths)?;",
    after: "PathList::new(&paths.into_iter().map(PathBuf::from).collect::<Vec<_>>())",
    message: "workspace PathBuf materialization must follow request validation",
  });

  const openLocalWorkspace = functionBody(source, "open_local_workspace");
  assertBefore({
    body: openLocalWorkspace,
    before: "validate_open_request_materialization(&workspace_paths, &diff_paths)",
    after: "derive_paths_with_position(app_state.fs.as_ref(), workspace_paths)",
    message: "local workspace path derivation must follow request validation",
  });
  assert.match(
    source,
    /const MAX_OPEN_REQUEST_ITEM_RELEASE_WAITS: usize = MAX_OPEN_REQUEST_MATERIALIZED_ITEMS;/,
  );
  assertBefore({
    body: openLocalWorkspace,
    before: "push_item_release_future(&mut item_release_futures, release_rx)",
    after: "future::try_join_all(item_release_futures)",
    message: "item-release wait futures must be capped before join_all",
  });

  const directReleasePushes = [
    ...source.matchAll(/item_release_futures\.push\(release_rx\)/g),
  ];
  assert.equal(
    directReleasePushes.length,
    1,
    "item release futures should only be pushed through the cap helper",
  );

  const openPathsWithPositions = functionBody(source, "open_paths_with_positions");
  assertBefore({
    body: openPathsWithPositions,
    before: "validate_path_position_materialization(path_positions)?;",
    after: "let paths = path_positions",
    message: "path positions must be count capped before path collection",
  });
  assertBefore({
    body: openPathsWithPositions,
    before: "validate_open_request_diff_paths(diff_paths)?;",
    after: "MultiDiffView::open(diff_paths.to_vec(), workspace, window, cx)",
    message: "diff paths must be capped before diff view vector materialization",
  });

  const navigation = openPathsWithPositions.slice(
    openPathsWithPositions.indexOf("let items_for_navigation"),
  );
  assertBefore({
    body: navigation,
    before: ".take(MAX_OPEN_REQUEST_MATERIALIZED_ITEMS)",
    after: ".collect::<Vec<_>>()",
    message: "workspace navigation items must be capped before collection",
  });
});

test("path derivation bounds iterator materialization before parsing paths", () => {
  const derivePaths = functionBody(source, "derive_paths_with_position");

  assert.doesNotMatch(
    derivePaths,
    /path_strings:\s*Vec<_>\s*=\s*path_strings\.into_iter\(\)\.collect/,
  );
  assertBefore({
    body: derivePaths,
    before: ".min(MAX_OPEN_REQUEST_PATHS)",
    after: "Vec::with_capacity(capacity)",
    message: "path derivation capacity must be capped",
  });
  assertBefore({
    body: derivePaths,
    before: "path_strings.take(MAX_OPEN_REQUEST_PATHS)",
    after: "PathWithPosition::parse_str(path_str)",
    message: "path derivation must cap iterator fanout before parsing",
  });
  assertBefore({
    body: derivePaths,
    before: "path_str.len() > MAX_OPEN_REQUEST_PATH_BYTES",
    after: "PathWithPosition::parse_str(path_str)",
    message: "path strings must be byte checked before path parsing",
  });
});
