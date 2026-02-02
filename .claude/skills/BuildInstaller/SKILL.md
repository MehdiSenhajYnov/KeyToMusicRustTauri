---
name: BuildInstaller
description: Build and package a production Windows installer for KeyToMusic with automatic version increment and git tagging
disable-model-invocation: true
---

# BuildInstaller - Build Production Installer

You are tasked with creating an up-to-date production Windows installer for KeyToMusic. Optional user context: pour créer un installer a jour

**IMPORTANT:**
- This creates a PRODUCTION installer with optimizations and release settings
- Automatically increments the version number (patch by default)
- Creates a git commit and tag for the release
- The build process takes several minutes (Rust release build + bundling)
- **DO NOT run this on uncommitted code changes** - warn the user first

---

## Step 1: Check git status

Run `git status --short` to verify there are no uncommitted changes.

- If there are uncommitted files, **STOP** and ask the user:
  - "You have uncommitted changes. Should I commit them first, or do you want to stash/discard them?"
  - Wait for user decision before proceeding

---

## Step 2: Determine version increment

Parse the optional user argument `pour créer un installer a jour`:
- If it contains `major`, increment major version (e.g., 1.0.0 → 2.0.0)
- If it contains `minor`, increment minor version (e.g., 1.0.0 → 1.1.0)
- If it contains a specific version like `v1.2.3` or `1.2.3`, use that exact version
- **Otherwise, increment patch version by default** (e.g., 1.0.0 → 1.0.1)

Read the current version from `package.json` (line 4) and `src-tauri/Cargo.toml` (line 3).

---

## Step 3: Update version numbers

Use the Edit tool to update the version in **both files**:
1. `package.json` - line 4: `"version": "X.Y.Z"`
2. `src-tauri/Cargo.toml` - line 3: `version = "X.Y.Z"`
3. `src-tauri/tauri.conf.json` - line 9: `"version": "X.Y.Z"`

**All three files must have the same version number.**

---

## Step 4: Commit version bump

Run the following git commands **sequentially**:

```bash
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json && git commit -m "chore: bump version to vX.Y.Z" && git tag -a "vX.Y.Z" -m "Release vX.Y.Z"
```

Replace `X.Y.Z` with the new version number.

---

## Step 5: Run production build

Execute the Tauri build command:

```bash
npm run tauri build
```

This will:
- Build the frontend with Vite (TypeScript compilation + bundling)
- Compile the Rust backend in release mode (optimizations enabled)
- Create a Windows NSIS installer in `src-tauri/target/release/bundle/nsis/`

**Expected build time:** 3-5 minutes on first build, 1-2 minutes on subsequent builds.

Monitor for errors. If the build fails:
- Check the error output for missing dependencies or compilation errors
- Common issues: missing Rust toolchain, node_modules not installed, TypeScript errors

---

## Step 6: Locate installer

After successful build, find the installer at:
```
src-tauri/target/release/bundle/nsis/KeyToMusic_X.Y.Z_x64-setup.exe
```

Use the Bash tool to get the exact file path and size:
```bash
ls -lh src-tauri/target/release/bundle/nsis/*.exe
```

---

## Step 7: Confirm completion

Tell the user:

```
✓ Production installer created successfully!

Version: vX.Y.Z
Location: src-tauri/target/release/bundle/nsis/KeyToMusic_X.Y.Z_x64-setup.exe
Size: [file size from ls command]

Git tag: vX.Y.Z (local only - not pushed)

Next steps:
- Test the installer on a clean Windows machine
- Push the tag: git push origin vX.Y.Z
- Upload to GitHub releases or distribution platform
```

**DO NOT push the git tag automatically** - let the user decide when to publish.
