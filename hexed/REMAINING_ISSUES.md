# Remaining Compilation Issues in sidebar.rs

## Status: 20 errors remaining in sidebar.rs

### Fixed So Far
✅ multi_workspace.rs - All errors fixed
✅ pane_group.rs - All errors fixed  
✅ agent_panel.rs - All errors fixed
✅ repl/remote_kernels.rs - async-tungstenite conflict RESOLVED!

### Remaining Issues in sidebar.rs

#### 1. Import Issues (Fixed in last commit)
- ❌ Duplicate `ProjectGroupKey` import
- ❌ Non-existent `SerializedProjectGroupKey` 
- ❌ Non-existent `ShowFewerThreads`, `ShowMoreThreads`
- ❌ Unused `linked_worktree_short_name`

**Fix Applied**: Removed duplicate and non-existent imports

#### 2. Missing Variables/Values
- `hover_color` → should be `hover_solid` (line 2116)
- `multi_workspace` → should be `self.multi_workspace` (line 4671)
- `ToggleArchive` → action not found (lines 4916, 4919)

#### 3. Missing Enum Variants
- `IconName::ThreadImport` → doesn't exist (line 4895)
- `ListEntry::DraftThread` → doesn't exist (line 1947)
- `ActiveEntry::Draft` → doesn't exist (line 2130)

#### 4. Type Mismatches
- `restoring_tasks` expects `SessionId` but getting `ThreadId` (lines 3212, 3272, 3327, 3347, 6068)
- This suggests `restoring_tasks` type changed from `HashMap<ThreadId, _>` to `HashMap<SessionId, _>`

#### 5. Missing Fields
- `collapsed_groups` field doesn't exist on Sidebar (lines 6201, 6247)
- `expanded_groups` field doesn't exist on Sidebar (lines 6207, 6252)

## Root Cause
These errors indicate that the merge between local (forge) and remote (main) branches had significant API changes:
- Thread management changed from `ThreadId` to `SessionId`
- Sidebar state management changed (collapsed_groups, expanded_groups removed)
- Some UI elements were removed or renamed (ThreadImport icon, DraftThread, etc.)

## Recommended Approach
1. Check what fields actually exist on Sidebar struct
2. Update all `ThreadId` references to `SessionId` where appropriate
3. Remove or replace references to deleted enum variants
4. Update serialization code to match current Sidebar structure
5. Find replacement for `ToggleArchive` action or remove the feature

## Build Progress
- 1597/1639 crates attempted (97%)
- Only sidebar.rs blocking completion
- All other crates compile successfully

## Key Achievement
✅ **async-tungstenite version conflict SOLVED** - This was the major blocker and is now completely resolved!
