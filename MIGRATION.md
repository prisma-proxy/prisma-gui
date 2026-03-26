# Migration Guide: Extracting prisma-gui to Standalone Repository

Step-by-step instructions for splitting `apps/prisma-gui/` from the monorepo into `github.com/prisma-proxy/prisma-gui`.

## Prerequisites

- `git-filter-repo` installed: `pip install git-filter-repo`
- Push access to both repos

## Step 1: Create the New Repository

```bash
gh repo create prisma-proxy/prisma-gui --private --description "Desktop + mobile client for Prisma encrypted proxy"
```

## Step 2: Extract with Git History

```bash
# Clone a fresh copy of the monorepo (don't use your working copy)
git clone https://github.com/prisma-proxy/prisma.git prisma-extract
cd prisma-extract

# Extract only apps/prisma-gui/ with full history, move to repo root
git filter-repo --subdirectory-filter apps/prisma-gui/

# Verify: root should now contain package.json, src/, src-tauri/, etc.
ls
```

## Step 3: Add Prisma Core as Git Submodule

```bash
git submodule add https://github.com/prisma-proxy/prisma.git prisma
git submodule update --init --recursive
```

## Step 4: Update Cargo Path Dependencies

Edit `src-tauri/Cargo.toml`:

```diff
-prisma-ffi  = { path = "../../../crates/prisma-ffi" }
-prisma-core = { path = "../../../crates/prisma-core" }
+prisma-ffi  = { path = "prisma/crates/prisma-ffi" }
+prisma-core = { path = "prisma/crates/prisma-core" }
```

Verify:
```bash
cd src-tauri && cargo check
```

## Step 5: Verify CI Workflows

The `.github/workflows/ci.yml` and `release.yml` files are already prepared for the standalone repo structure. They use `submodules: recursive` in checkout steps and correct working directories.

Verify locally:
```bash
npm ci
npx tsc --noEmit
cd src-tauri && cargo check && cargo clippy --all-targets && cargo fmt --all -- --check
```

## Step 6: Push to New Repository

```bash
git remote set-url origin https://github.com/prisma-proxy/prisma-gui.git
git push -u origin main
```

## Step 7: Update the Monorepo

In the original `prisma` repository:

1. **Remove the GUI directory**:
   ```bash
   git rm -r apps/prisma-gui
   ```

2. **Update `.github/workflows/ci.yml`**: Remove the `gui-check` job and all `apps/prisma-gui` path triggers.

3. **Update `.github/workflows/release.yml`**: Remove the `build-gui` job entirely.

4. **Update `CLAUDE.md`**: Remove `prisma-gui` from the workspace layout table. Add a note pointing to the new repo.

5. **Update `.claude/agents/version-sync.md`**: Remove the 3 GUI version files from the sync list (files 2-4).

6. **Update `.claude/agents/frontend-engineer.md`**: Remove the `prisma-gui` section. Keep only console/CLI/docs.

7. **Commit and push**:
   ```bash
   git add -A
   git commit -m "chore: extract prisma-gui to standalone repository"
   git push
   ```

## Step 8: Tag First Release in New Repo

```bash
cd prisma-gui
git tag v2.26.0  # Match the last monorepo version
git push origin --tags
```

## Step 9: Verify

- [ ] `prisma-gui` repo CI passes (TypeScript + Rust check)
- [ ] `prisma-gui` release workflow builds for all 3 platforms
- [ ] Monorepo CI still passes without GUI jobs
- [ ] Submodule update works: `cd prisma && git pull origin master`
- [ ] `cargo check` works after submodule update

## Ongoing Workflow

### Updating core crates in the GUI repo

```bash
cd prisma-gui/prisma
git pull origin master
cd ..
git add prisma
git commit -m "chore: update prisma submodule to latest"
```

### Developing core + GUI simultaneously

```bash
# In the GUI repo, make changes to prisma/crates/prisma-ffi/
cd prisma-gui/prisma/crates/prisma-ffi
# Edit files...
cd prisma-gui
cargo check -p prisma-gui  # Builds with local changes

# When ready, push core changes from the submodule
cd prisma-gui/prisma
git add . && git commit -m "feat: ..."
git push origin master

# Then update the submodule reference
cd prisma-gui
git add prisma && git commit -m "chore: update prisma submodule"
```
