---
name: version-sync
description: "Atomic version synchronization across GUI version files. Takes a version string or bump type and updates all 3 files."
model: sonnet
---

# Version Sync Agent

## Input

- **Explicit version**: `"3.0.0"` — use as-is
- **Bump type**: `"major"`, `"minor"`, or `"patch"` — compute from current

## Step 1: Read Current Version

```bash
grep '"version"' package.json | head -1
```

## Step 2: Update 3 Files

| File | Pattern |
|------|---------|
| `package.json` | `"version": "OLD"` → `"version": "NEW"` |
| `src-tauri/tauri.conf.json` | `"version": "OLD"` → `"version": "NEW"` |
| `src-tauri/Cargo.toml` | `version = "OLD"` → `version = "NEW"` |

## Step 3: Validate + Commit

```bash
cd src-tauri && cargo check
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: bump version to NEW"
```
