import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (path: string) => readFileSync(path, "utf8");

test("deploy receipt directory scans use named entry caps and bounded latest candidates", () => {
  const source = read("crates/agent_ui/src/dx_deploy_receipt_files.rs");

  assert.match(source, /const DEPLOY_RECEIPT_ROOT_ENTRY_LIMIT: usize = 128;/);
  assert.match(source, /const DEPLOY_RECEIPT_NESTED_ENTRY_LIMIT: usize = 64;/);
  assert.match(source, /const DEPLOY_RECEIPT_LATEST_ROOT_ENTRY_LIMIT: usize = 64;/);
  assert.match(source, /const DEPLOY_RECEIPT_LATEST_NESTED_ENTRY_LIMIT: usize = 64;/);
  assert.match(source, /const DEPLOY_RECEIPT_LATEST_CANDIDATE_LIMIT: usize = 16;/);
  assert.match(source, /limit\.min\(DEPLOY_RECEIPT_LATEST_CANDIDATE_LIMIT\)/);
  assert.match(source, /entries\s*\.flatten\(\)\s*\.take\(DEPLOY_RECEIPT_ROOT_ENTRY_LIMIT\)/);
  assert.match(source, /entries\s*\.flatten\(\)\s*\.take\(DEPLOY_RECEIPT_NESTED_ENTRY_LIMIT\)/);
  assert.match(source, /entries\s*\.flatten\(\)\s*\.take\(DEPLOY_RECEIPT_LATEST_ROOT_ENTRY_LIMIT\)/);
  assert.match(source, /children\s*\.flatten\(\)\s*\.take\(DEPLOY_RECEIPT_LATEST_NESTED_ENTRY_LIMIT\)/);
  assert.match(source, /fn push_bounded_receipt_candidate\(/);
  assert.match(source, /candidate_limit: usize/);
  assert.match(source, /receipts\.sort_by\(newest_first\);[\s\S]*receipts\.truncate\(candidate_limit\);/);
});

test("global DX receipt root scans use named entry caps and bounded latest labels", () => {
  const source = read("crates/agent_ui/src/dx_receipts.rs");

  assert.match(source, /const DX_RECEIPT_BUCKET_ENTRY_LIMIT: usize = 128;/);
  assert.match(source, /const DX_RECEIPT_BUCKET_NESTED_ENTRY_LIMIT: usize = 32;/);
  assert.match(source, /const DX_RECEIPT_LATEST_ROOT_ENTRY_LIMIT: usize = 24;/);
  assert.match(source, /const DX_RECEIPT_LATEST_CHILD_ENTRY_LIMIT: usize = 24;/);
  assert.match(source, /const DX_RECEIPT_LATEST_LABEL_LIMIT: usize = 4;/);
  assert.match(source, /children\s*\.flatten\(\)\s*\.take\(DX_RECEIPT_LATEST_ROOT_ENTRY_LIMIT\)/);
  assert.match(source, /entries\s*\.flatten\(\)\s*\.take\(DX_RECEIPT_LATEST_CHILD_ENTRY_LIMIT\)/);
  assert.match(source, /fn push_bounded_receipt_label\(/);
  assert.match(source, /receipts\.sort_by\(\|left, right\| right\.0\.partial_cmp\(&left\.0\)\.unwrap_or\(Ordering::Equal\)\);[\s\S]*receipts\.truncate\(DX_RECEIPT_LATEST_LABEL_LIMIT\);/);
});

test("receipt history scans cap nested enumeration and shared latest collection", () => {
  const source = read("crates/agent_ui/src/dx_receipt_history/receipt_files.rs");

  assert.match(source, /const RECEIPT_HISTORY_ROOT_ENTRY_LIMIT: usize = 192;/);
  assert.match(source, /const RECEIPT_HISTORY_NESTED_ENTRY_LIMIT: usize = 64;/);
  assert.match(source, /const RECEIPT_HISTORY_LATEST_ROOT_ENTRY_LIMIT: usize = 64;/);
  assert.match(source, /const RECEIPT_HISTORY_LATEST_NESTED_ENTRY_LIMIT: usize = 64;/);
  assert.match(source, /const RECEIPT_HISTORY_LATEST_CANDIDATE_LIMIT: usize = 32;/);
  assert.match(source, /entries\s*\.flatten\(\)\s*\.take\(RECEIPT_HISTORY_ROOT_ENTRY_LIMIT\)/);
  assert.match(source, /entries\s*\.flatten\(\)\s*\.take\(RECEIPT_HISTORY_NESTED_ENTRY_LIMIT\)/);
  assert.match(source, /entries\s*\.flatten\(\)\s*\.take\(RECEIPT_HISTORY_LATEST_ROOT_ENTRY_LIMIT\)/);
  assert.match(source, /children\s*\.flatten\(\)\s*\.take\(RECEIPT_HISTORY_LATEST_NESTED_ENTRY_LIMIT\)/);
  assert.match(source, /fn push_bounded_receipt_label\(/);
  assert.match(source, /receipts\.sort_by\(\|left, right\| \{[\s\S]*std::cmp::Ordering::Equal[\s\S]*\}\);[\s\S]*receipts\.truncate\(RECEIPT_HISTORY_LATEST_CANDIDATE_LIMIT\);/);
});

test("proof freshness scans use named entry caps and bounded latest labels", () => {
  const source = read("crates/agent_ui/src/dx_proof_freshness.rs");

  assert.match(source, /const PROOF_FRESHNESS_RECEIPT_ROOT_ENTRY_LIMIT: usize = 128;/);
  assert.match(source, /const PROOF_FRESHNESS_RECEIPT_NESTED_ENTRY_LIMIT: usize = 64;/);
  assert.match(source, /const PROOF_FRESHNESS_LATEST_ROOT_ENTRY_LIMIT: usize = 64;/);
  assert.match(source, /const PROOF_FRESHNESS_LATEST_NESTED_ENTRY_LIMIT: usize = 64;/);
  assert.match(source, /const PROOF_FRESHNESS_LATEST_CANDIDATE_LIMIT: usize = 8;/);
  assert.match(source, /limit\.min\(PROOF_FRESHNESS_LATEST_CANDIDATE_LIMIT\)/);
  assert.match(source, /entries\s*\.flatten\(\)\s*\.take\(PROOF_FRESHNESS_RECEIPT_ROOT_ENTRY_LIMIT\)/);
  assert.match(source, /entries\s*\.flatten\(\)\s*\.take\(PROOF_FRESHNESS_RECEIPT_NESTED_ENTRY_LIMIT\)/);
  assert.match(source, /entries\s*\.flatten\(\)\s*\.take\(PROOF_FRESHNESS_LATEST_ROOT_ENTRY_LIMIT\)/);
  assert.match(source, /children\s*\.flatten\(\)\s*\.take\(PROOF_FRESHNESS_LATEST_NESTED_ENTRY_LIMIT\)/);
  assert.match(source, /fn push_bounded_receipt_label\(/);
  assert.match(source, /receipts\.sort_by\(\|left, right\| right\.0\.partial_cmp\(&left\.0\)\.unwrap_or\(Ordering::Equal\)\);[\s\S]*receipts\.truncate\(candidate_limit\);/);
});
