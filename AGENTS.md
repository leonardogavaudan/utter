# Utter

Context-aware voice typing: Tauri 2 app with a Rust workspace and a React UI. Read `README.md` for the architecture and privacy model.

## Where code goes

- Logic that does not need the operating system goes in `crates/utter-core`, with tests. HTTP clients are tested against `wiremock` servers, never real providers.
- Operating-system calls go in `crates/utter-os`, with the platform split by `#[cfg(target_os = ...)]` behind one public function per capability (`context::capture`, `insert::stage_text`, `permissions::accessibility_granted`).
- `src-tauri` stays thin: commands, the dictation state machine, windows, tray, shortcut. It calls the two crates and holds no provider or prompt logic.

## Conventions

- Types the UI uses derive `ts_rs::TS` with `#[ts(export)]`. Run `bun run bindings` after changing them and never edit `src/bindings` by hand. Use `u32` rather than `u64` for numbers the UI reads, since ts-rs maps `u64` to `bigint`.
- Never log dictated text, transcripts, or context; log sizes and timings only.
- No code comments: put the reason for a choice in the commit message.
- Before finishing a change, run `bun run check` (rustfmt, clippy with `-D warnings`, all tests, TypeScript, Biome).

## Releasing

1. Set the new version in `[workspace.package]` in the root `Cargo.toml`. It is the only place the version lives; `tauri.conf.json` and `package.json` have none.
2. Commit, then push a matching tag: `git tag v0.2.0 && git push origin v0.2.0`.
3. `.github/workflows/release.yml` checks that the tag matches the version, builds a universal app, signs it with the Developer ID certificate, notarizes it, and creates a draft GitHub Release with the `.dmg`, the updater archive and `latest.json`.
4. Check the draft, then publish it with `gh release edit v0.2.0 --draft=false`. Installed copies find it through `latest.json` within six hours of their next check.

The signing certificate, notarization key and updater key are GitHub Actions secrets (`gh secret list`). Losing the updater private key means installed copies can no longer accept updates, so keep its backup.

## Testing on macOS

- Permissions belong to the process macOS sees: `bun run dev` runs under your terminal's permissions. Build the bundle to test Accessibility, microphone, and Keychain prompts as a user gets them.
- If the Mac has the "Utter Local Signing" identity (`security find-identity -p codesigning`; recipe in the README), build with `APPLE_SIGNING_IDENTITY="Utter Local Signing" bunx tauri build --debug --bundles app`. Ad-hoc builds lose the Accessibility and microphone grants on every rebuild. `codesign` fails with `errSecInternalComponent` inside a sandboxed agent shell, so run the build outside the sandbox.
- Computer Use cannot click macOS permission prompts or password dialogs (UserNotificationCenter and SecurityAgent are blocked), and its synthetic key presses do not trigger global shortcuts. Ask the user for those clicks and for a real test of the dictation shortcut.
- The system-wide Accessibility element (`AXUIElementCreateSystemWide`) returned `kAXErrorCannotComplete` for the focused app when tested on macOS 26 (2026/11/21). Find the frontmost app with `NSWorkspace` and query it with `AXUIElementCreateApplication(pid)`.
- `cargo run -p utter-os --example inspect_context -- 5` prints what Utter reads from the field you focus.
