---
name: Release
description: Complete release automation - builds installer, commits changes, creates git tag, pushes to GitHub, and publishes release with installer. Use when you have a stable version or urgent bugfix ready to ship.
disable-model-invocation: true
---

# Release - Full Release Automation

You are tasked with executing a complete production release of KeyToMusic. This combines building the installer, committing any changes, creating git tags, and publishing to GitHub releases. Optional user context: {{PLACEHOLDER}}

**IMPORTANT:**
- This is a PRODUCTION release workflow with git push and public GitHub release
- Only use this when the code is stable and tested
- Automatically handles version increment, build, git operations, and GitHub release
- **DO NOT run this on experimental/untested code**
- The entire process takes 5-15 minutes depending on build cache

---

## Step 1: Pre-flight checks

Run these checks in parallel:

1. **Git status check:**
   ```bash
   git status --short
   ```

2. **GitHub CLI authentication:**
   ```bash
   gh auth status
   ```

**Evaluate results:**
- If there are uncommitted changes to tracked files (modified/deleted, NOT untracked), ask the user:
  - "You have uncommitted changes. Should I commit them first with a message, or do you want to handle them manually?"
  - Wait for user decision before proceeding

- If `gh auth status` fails, tell the user:
  - "GitHub CLI is not authenticated. Please run: `gh auth login`"
  - STOP here

---

## Step 2: Determine version increment

Parse the optional user argument (from `{{PLACEHOLDER}}`):
- If it contains `major`, increment major version (e.g., 1.0.1 → 2.0.0)
- If it contains `minor`, increment minor version (e.g., 1.0.1 → 1.1.0)
- If it contains a specific version like `v1.2.3` or `1.2.3`, use that exact version
- **Otherwise, increment patch version by default** (e.g., 1.0.1 → 1.0.2)

Read the current version from `package.json` (line 4).

**Confirm with user:**
- "Ready to release KeyToMusic v[NEW_VERSION]. This will build, tag, push, and publish to GitHub. Proceed?"
- If user says no, STOP

---

## Step 3: Commit pending changes (if user agreed in Step 1)

If there were uncommitted changes and the user asked to commit them:

```bash
git add -A && git commit -m "chore: prepare release v[NEW_VERSION]"
```

Otherwise, skip this step.

---

## Step 4: Update version numbers

Use the Edit tool to update the version in **all three files**:
1. `package.json` - line 4: `"version": "[NEW_VERSION]"`
2. `src-tauri/Cargo.toml` - line 3: `version = "[NEW_VERSION]"`
3. `src-tauri/tauri.conf.json` - line 9: `"version": "[NEW_VERSION]"`

**All three files must have the same version number.**

---

## Step 5: Commit version bump and create git tag

Run these git commands **sequentially**:

```bash
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json && git commit -m "chore: bump version to v[NEW_VERSION]" && git tag -a "v[NEW_VERSION]" -m "Release v[NEW_VERSION]"
```

Replace `[NEW_VERSION]` with the actual version number.

---

## Step 6: Build production installer

Execute the Tauri build command:

```bash
npm run tauri build
```

This will:
- Build the frontend with Vite (TypeScript compilation + bundling)
- Compile the Rust backend in release mode (optimizations enabled)
- Create a Windows NSIS installer in `src-tauri/target/release/bundle/nsis/`

**Expected build time:** 3-5 minutes on first build, 1-2 minutes on subsequent builds.

If the build fails:
- Show the error to the user
- STOP and ask: "Build failed. Do you want to fix the issue and retry?"
- Do NOT proceed with git push or release creation

---

## Step 7: Get installer details

After successful build, get the installer file size:

```bash
ls -lh "src-tauri/target/release/bundle/nsis/KeyToMusic_[NEW_VERSION]_x64-setup.exe"
```

Store the file path and size for the final confirmation.

---

## Step 8: Push to GitHub

Push both the commits and the tag:

```bash
git push origin main && git push origin v[NEW_VERSION]
```

If this fails:
- Show the error to the user
- STOP and ask: "Git push failed. This might be a network issue or branch protection. Please resolve manually."
- Do NOT proceed with release creation

---

## Step 9: Create GitHub release

Generate release notes based on recent commits:

```bash
git log --oneline --no-decorate $(git describe --tags --abbrev=0 HEAD^)..HEAD 2>/dev/null || git log --oneline --no-decorate -5
```

Then create the GitHub release with the installer attached:

```bash
gh release create v[NEW_VERSION] \
  "src-tauri/target/release/bundle/nsis/KeyToMusic_[NEW_VERSION]_x64-setup.exe" \
  --title "KeyToMusic v[NEW_VERSION]" \
  --notes "[GENERATED_NOTES]

## Installation
Download and run the Windows installer: KeyToMusic_[NEW_VERSION]_x64-setup.exe

## Features
- Multi-track audio soundboard for manga reading
- Global keyboard detection with customizable shortcuts
- YouTube download integration
- Waveform visualization with momentum detection
- Discovery system for new sounds
- Multi-key chord support

For more information, see the [project documentation](https://github.com/MehdiSenhajYnov/KeyToMusicRustTauri)."
```

Replace `[GENERATED_NOTES]` with a formatted list of commits (if available), or use a generic message like "Bug fixes and improvements".

If this fails:
- Show the error to the user
- Note that the git tag and commits are already pushed
- Suggest: "Release creation failed, but code is pushed. You can create the release manually or retry."

---

## Step 10: Confirm completion

Tell the user:

```
✓ Release v[NEW_VERSION] published successfully!

Version: v[NEW_VERSION]
Installer: KeyToMusic_[NEW_VERSION]_x64-setup.exe ([FILE_SIZE])
GitHub Release: [RELEASE_URL]

The release is now live and ready for distribution:
- Users can download the installer from GitHub
- Git tag v[NEW_VERSION] is pushed
- All changes are committed and pushed to main

Next steps:
- Announce the release to users
- Monitor for any issues
- Start working on the next version
```

Include the actual GitHub release URL from the `gh release create` output.
