# Professional Fork Maintenance Guide
**Date:** April 7, 2026  
**Repository:** Zed Editor Fork (Highly Active Upstream)  
**Your Fork:** millercarla211-ctrl/zed

---

## Current Setup ✅

### Remote Configuration
```bash
origin    → https://github.com/millercarla211-ctrl/zed (your fork)
upstream  → https://github.com/zed-industries/zed.git (official Zed)
```

### Branch Strategy
- **main**: Mirrors upstream/main exactly (no custom commits)
- **dev**: Your development branch with all customizations

### Git Configuration
```bash
pull.rebase = true          # Use rebase by default
pull.ff = only              # Only allow fast-forward when possible
main tracks upstream/main   # Main syncs with Zed directly
dev tracks origin/dev       # Dev syncs with your fork
```

---

## Daily Workflow (Professional Best Practices)

### 1. Morning Sync (Start of Day)
```bash
# Sync main with upstream Zed (creates NO new commits)
git checkout main
git pull                    # Fast-forward to upstream/main
git push origin main        # Update your fork's main

# Update dev with latest Zed changes
git checkout dev
git rebase main             # Replay your commits on top of latest Zed
# If conflicts occur, resolve them, then:
# git rebase --continue
git push origin dev --force-with-lease  # Safe force push
```

**Why rebase instead of merge?**
- Keeps linear history (easier to review)
- No merge commits cluttering history
- Industry standard for active repositories in 2026
- Makes your PRs cleaner if you contribute back

---

### 2. Working on Features
```bash
# Always work in dev branch
git checkout dev

# Make changes, test, commit
git add .
git commit -m "feat: descriptive message following conventions"

# Push to your fork
git push origin dev
```

---

### 3. Multiple Times Per Day (Active Repo)
Since Zed is highly active, sync 2-3 times daily:

```bash
# Quick sync (while on dev branch)
git fetch upstream
git rebase upstream/main
git push origin dev --force-with-lease
```

**Why `--force-with-lease`?**
- Safer than `--force`
- Prevents overwriting others' work
- Checks remote hasn't changed unexpectedly

---

## Critical Rules for Active Repositories

### ✅ DO:
1. **Sync frequently** (2-3x daily for active repos like Zed)
2. **Use rebase** for integrating upstream changes
3. **Keep main pristine** (never commit directly to main)
4. **Use descriptive commits** following [Conventional Commits](https://www.conventionalcommits.org/)
5. **Test before pushing** to dev
6. **Use `--force-with-lease`** instead of `--force`
7. **Document your changes** in commit messages
8. **Keep dev branch focused** (one feature/fix per branch ideally)

### ❌ DON'T:
1. **Never commit to main** (it should mirror upstream)
2. **Don't use `git pull` on dev** after rebasing (use fetch + rebase)
3. **Don't force push without `--force-with-lease`**
4. **Don't let dev get too far behind** (sync daily)
5. **Don't merge main into dev** (always rebase)
6. **Don't rebase public branches** others depend on
7. **Don't ignore conflicts** (resolve immediately)

---

## Handling Conflicts (Common in Active Repos)

When rebasing causes conflicts:

```bash
# 1. Rebase starts
git rebase main
# Conflict occurs...

# 2. Check what's conflicted
git status

# 3. Open conflicted files, resolve markers:
#    <<<<<<< HEAD
#    ======= 
#    >>>>>>> 

# 4. Mark as resolved
git add <resolved-files>

# 5. Continue rebase
git rebase --continue

# 6. If too messy, abort and try again
git rebase --abort
```

---

## Advanced: Feature Branch Workflow

For larger features, create sub-branches:

```bash
# Create feature branch from dev
git checkout dev
git checkout -b feature/my-awesome-feature

# Work on feature
git commit -m "feat: add awesome feature"

# When ready, rebase onto latest dev
git fetch upstream
git rebase dev

# Merge into dev (or squash)
git checkout dev
git merge feature/my-awesome-feature --squash
git commit -m "feat: complete awesome feature implementation"

# Clean up
git branch -d feature/my-awesome-feature
```

---

## Monitoring Upstream Activity

### Check what's new in upstream:
```bash
git fetch upstream
git log main..upstream/main --oneline  # See new commits
git diff main..upstream/main           # See changes
```

### Check if you're behind:
```bash
git fetch upstream
git status  # Shows "Your branch is behind..."
```

---

## Emergency Procedures

### If you accidentally committed to main:
```bash
git checkout main
git reset --hard upstream/main
git push origin main --force-with-lease
```

### If dev is completely messed up:
```bash
# Create backup
git branch dev-backup

# Reset dev to main + cherry-pick your commits
git checkout main
git branch -D dev
git checkout -b dev
git cherry-pick <your-commit-hashes>
git push origin dev --force-with-lease
```

### If you need to undo last commit:
```bash
git reset --soft HEAD~1  # Keeps changes staged
# or
git reset --hard HEAD~1  # Discards changes (dangerous!)
```

---

## Commit Message Convention

Follow Conventional Commits (industry standard):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting, missing semicolons
- `refactor`: Code restructuring
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Maintenance tasks

**Example:**
```
feat(editor): add low-memory build configuration

- Added .cargo/low-memory-config.toml
- Created justfile for build automation
- Updated theme settings for DX theme

Closes #123
```

---

## Verification Commands

### Check your setup:
```bash
git remote -v                          # Verify remotes
git branch -vv                         # Check branch tracking
git config --get pull.rebase           # Should be "true"
git config --get pull.ff               # Should be "only"
```

### Check sync status:
```bash
git fetch upstream
git log --oneline --graph --all --decorate -20  # Visual history
```

---

## What You're Doing Right ✅

1. ✅ Separate main and dev branches
2. ✅ Main tracks upstream (no divergence)
3. ✅ Using rebase for clean history
4. ✅ Configured pull.rebase and pull.ff
5. ✅ Understanding fast-forward merges

## What to Watch Out For ⚠️

1. ⚠️ **Sync frequency**: With Zed's activity level, sync 2-3x daily
2. ⚠️ **Conflict resolution**: Learn to resolve conflicts quickly
3. ⚠️ **Force push safety**: Always use `--force-with-lease`
4. ⚠️ **Commit granularity**: Keep commits atomic and focused
5. ⚠️ **Testing**: Test before pushing to avoid breaking dev

---

## Resources

- [Conventional Commits](https://www.conventionalcommits.org/)
- [Atlassian Git Rebase Tutorial](https://www.atlassian.com/git/tutorials/merging-vs-rebasing)
- [GitHub Fork Sync Guide](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/working-with-forks/syncing-a-fork)
- [Git Rebase vs Merge (2026)](https://blog.mergify.com/rebase-git-vs-merge/)

---

## Quick Reference Card

```bash
# Daily morning routine
git checkout main && git pull && git push origin main
git checkout dev && git rebase main && git push origin dev --force-with-lease

# Quick sync during day
git fetch upstream && git rebase upstream/main && git push origin dev --force-with-lease

# Check status
git fetch upstream && git status

# Emergency reset main
git checkout main && git reset --hard upstream/main && git push origin main --force-with-lease
```

---

**Last Updated:** April 7, 2026  
**Status:** Production-ready workflow for highly active upstream repository
