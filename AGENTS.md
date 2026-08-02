# Inquivora repository instructions

## Release workflow

When preparing or publishing a release, treat the user-requested public version as the single source of truth.

- Keep the GitHub release title, Git tag, installer filename, and application version identical: `Inquivora vX.Y.Z`, `vX.Y.Z`, `Inquivora_X.Y.Z_x64-setup.exe`, and `X.Y.Z`.
- Update `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json` before releasing.
- Update README download links, installer filename, current-version heading, and release highlights to match the actual changes in that release.
- Do not rename an older tag's release to a different public version. Create and publish a tag built from the matching internal version.
- Run the complete release preflight before pushing a tag. After GitHub Actions completes, verify the published release state, installer, and `SHA256SUMS.txt`.
- When the user asks to publish or release, include the relevant README update, commit, and push in the same task unless they explicitly limit the scope.
