import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

const sliceBetween = (source: string, start: string, end: string) => {
  const startIndex = source.indexOf(start);
  assert.notEqual(startIndex, -1, `missing start marker: ${start}`);
  const endIndex = source.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `missing end marker after ${start}: ${end}`);
  return source.slice(startIndex, endIndex);
};

test("eager startup theme loading uses sentinel byte reads before theme parsing", () => {
  const source = read("crates/zed/src/zed.rs");

  assert.match(source, /const MAX_EAGER_THEME_BYTES: u64 = 4 \* 1024 \* 1024;/);
  assert.match(source, /const MAX_EAGER_ICON_THEME_BYTES: u64 = 8 \* 1024 \* 1024;/);

  const helper = sliceBetween(
    source,
    "async fn load_limited_startup_file_bytes",
    "pub(crate) fn eager_load_active_theme_and_icon_theme",
  );
  assert.match(helper, /fs\.open_sync\(path\)\.await\?/);
  assert.match(helper, /take\(max_bytes \+ 1\)/);
  assert.match(helper, /read_to_end\(&mut contents\)\?/);
  assert.match(helper, /contents\.len\(\) as u64 > max_bytes/);
  assert.match(helper, /return Ok\(None\);/);
  assert.doesNotMatch(helper, /load_user_theme|deserialize_icon_theme/);

  const eager = sliceBetween(
    source,
    "pub(crate) fn eager_load_active_theme_and_icon_theme",
    "#[cfg(test)]",
  );
  assert.match(
    eager,
    /load_limited_startup_file_bytes\(\s*&fs,\s*&theme_path,\s*MAX_EAGER_THEME_BYTES,\s*"theme",\s*\)/,
  );
  assert.match(
    eager,
    /load_limited_startup_file_bytes\(\s*&fs,\s*&icon_theme_path,\s*MAX_EAGER_ICON_THEME_BYTES,\s*"icon theme",\s*\)/,
  );
  assert.ok(
    eager.indexOf("MAX_EAGER_THEME_BYTES") <
      eager.indexOf("load_user_theme(theme_registry, &bytes)"),
    "theme bytes must be sentinel-limited before user theme parsing",
  );
  assert.ok(
    eager.indexOf("MAX_EAGER_ICON_THEME_BYTES") <
      eager.indexOf("deserialize_icon_theme(&bytes)"),
    "icon theme bytes must be sentinel-limited before icon theme deserialization",
  );
  assert.doesNotMatch(eager, /fs\.load_bytes\(&theme_path\)/);
  assert.doesNotMatch(eager, /fs\.load_bytes\(&icon_theme_path\)/);
});

test("user keymap reload bounds watched content before migration and parse", () => {
  const source = read("crates/zed/src/zed.rs");

  const keymap = sliceBetween(
    source,
    "const MAX_USER_KEYMAP_BYTES",
    "fn show_keymap_file_json_error",
  );
  assert.match(keymap, /const MAX_USER_KEYMAP_BYTES: usize = 1024 \* 1024;/);
  assert.match(keymap, /fn bounded_user_keymap_content\(content: String\) -> Option<String>/);
  assert.match(keymap, /content\.len\(\) > MAX_USER_KEYMAP_BYTES/);
  assert.match(keymap, /return None;/);
  assert.match(keymap, /Some\(content\)/);
  assert.match(
    keymap,
    /let Some\(content\) = bounded_user_keymap_content\(content\) else \{\s+continue;\s+\};/,
  );
  assert.ok(
    keymap.indexOf("bounded_user_keymap_content(content)") <
      keymap.indexOf("migrate_keymap(&content)"),
    "watched keymap content must be bounded before migration",
  );
  assert.ok(
    keymap.indexOf("bounded_user_keymap_content(content)") <
      keymap.indexOf("KeymapFile::load(&user_keymap_content, cx)"),
    "watched keymap content must be bounded before keymap parsing",
  );
});

test("startup user-theme loading bounds directory scans and theme bytes", () => {
  const source = read("crates/zed/src/main.rs");

  assert.match(source, /const MAX_USER_THEME_BYTES: u64 = 4 \* 1024 \* 1024;/);
  assert.match(source, /const MAX_USER_THEME_DIR_ENTRIES: usize = 256;/);
  assert.match(source, /const MAX_USER_THEME_WATCH_EVENT_PATHS: usize = 256;/);

  const helper = sliceBetween(
    source,
    "async fn load_limited_user_theme_bytes",
    "fn load_user_themes_in_background",
  );
  assert.match(helper, /fs\.open_sync\(path\)\.await\?/);
  assert.match(helper, /take\(MAX_USER_THEME_BYTES \+ 1\)/);
  assert.match(helper, /read_to_end\(&mut bytes\)\?/);
  assert.match(helper, /bytes\.len\(\) as u64 > MAX_USER_THEME_BYTES/);
  assert.match(helper, /return Ok\(None\);/);
  assert.doesNotMatch(helper, /load_user_theme/);

  const loader = sliceBetween(
    source,
    "fn load_user_themes_in_background",
    "fn watch_themes",
  );
  assert.match(loader, /if scanned_entries >= MAX_USER_THEME_DIR_ENTRIES \{/);
  assert.match(loader, /load_limited_user_theme_bytes\(&fs, &theme_path\)/);
  assert.ok(
    loader.indexOf("load_limited_user_theme_bytes(&fs, &theme_path)") <
      loader.indexOf("load_user_theme(&theme_registry, &bytes)"),
    "user theme bytes must be sentinel-limited before theme parsing",
  );
  assert.doesNotMatch(loader, /fs\.load_bytes\(&theme_path\)/);

  const watcher = sliceBetween(
    source,
    "fn watch_themes",
    "#[cfg(debug_assertions)]",
  );
  assert.match(watcher, /paths\.into_iter\(\)\.take\(MAX_USER_THEME_WATCH_EVENT_PATHS\)/);
  assert.match(watcher, /load_limited_user_theme_bytes\(&fs, &event\.path\)/);
  assert.ok(
    watcher.indexOf("load_limited_user_theme_bytes(&fs, &event.path)") <
      watcher.indexOf("load_user_theme(&theme_registry, &bytes)"),
    "watched user theme bytes must be sentinel-limited before theme parsing",
  );
  assert.doesNotMatch(watcher, /fs\.load_bytes\(&event\.path\)/);
});

test("debug language watcher caps startup directory registration", () => {
  const source = read("crates/zed/src/main.rs");

  assert.match(
    source,
    /const MAX_DEBUG_LANGUAGE_WATCH_DIR_ENTRIES: usize = 256;/,
  );

  const watcher = sliceBetween(
    source,
    "fn watch_languages",
    "fn dump_all_gpui_actions",
  );
  assert.match(watcher, /if scanned_entries >= MAX_DEBUG_LANGUAGE_WATCH_DIR_ENTRIES \{/);
  assert.match(watcher, /scanned_entries \+= 1;/);
  assert.ok(
    watcher.indexOf("if scanned_entries >= MAX_DEBUG_LANGUAGE_WATCH_DIR_ENTRIES") <
      watcher.indexOf("watcher.add(&path)"),
    "debug language directories must be capped before registering watchers",
  );
});
