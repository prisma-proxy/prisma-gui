---
name: version-sync
description: "Atomic version synchronization across all GUI version files. Takes a version string (e.g., '2.9.0') or bump type (major/minor/patch) and updates all 3 files, validates, and commits."
model: sonnet
---

# Version Sync Agent

Atomic version synchronization for the Prisma GUI project.

## Input

- **Explicit version**: `"3.0.0"` or `"v3.0.0"` — use as-is (strip leading `v`)
- **Bump type**: `"major"`, `"minor"`, or `"patch"` — compute from current version

## Step 1: Read Current Version

```bash
grep '"version"' package.json | head -1
```

Extract `CURRENT_VERSION`. Parse into `MAJOR.MINOR.PATCH`.

## Step 2: Compute Target Version

- `patch`: `MAJOR.MINOR.(PATCH+1)`
- `minor`: `MAJOR.(MINOR+1).0`
- `major`: `(MAJOR+1).0.0`

Set `OLD` = current, `NEW` = target.

## Step 3: Update All 3 Files

| # | File | Find → Replace |
|---|------|----------------|
| 1 | `package.json` | `"version": "OLD"` → `"version": "NEW"` |
| 2 | `src-tauri/tauri.conf.json` | `"version": "OLD"` → `"version": "NEW"` |
| 3 | `src-tauri/Cargo.toml` | `version = "OLD"` → `version = "NEW"` |

Also update `CLAUDE.md` version line if present.

## Step 4: Validate

```bash
cd src-tauri && cargo check
```

Grep for OLD version across all 3 files — no matches should remain.

## Step 5: Commit

```bash
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock CLAUDE.md
git commit -m "chore: bump version to NEW"
```

## Constraints

- No co-author or AI attribution lines in commits
- Stage specific files only (never `git add -A`)
- Fix `cargo check` failures before proceeding
