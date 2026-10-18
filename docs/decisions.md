# Decisions

## 2026-10-18 — Rust-native first milestone

The direct request to use Rust takes precedence over the pasted prompt's TypeScript/Electron stack. NS Plasma therefore starts as one Rust binary with an eframe desktop shell and a CLI entry point. Keeping the core provider/config/workspace code outside the UI makes a later split into a dedicated engine and richer front end straightforward.

## 2026-10-18 — Provider honesty

The app uses an OpenAI-compatible adapter without bundling keys or model IDs. Model discovery comes from the configured endpoint's `/models` response. Missing configuration is displayed as an empty state instead of a fake provider.

## 2026-10-18 — REA integration boundary

The REA Codex registration is aligned, but the live REA MCP tools were not present in this agent session. The official `rea-agents@6.5.0` CLI fallback is used for post-build Rust artifact inspection. The missing live MCP connection is a session limitation, not evidence that REA is unavailable on the machine.

## 2026-10-18 — Verification environment

`cargo fmt --check` and `cargo metadata` pass. A full Windows link was not possible in this session because `link.exe`/Windows SDK libraries are absent; the installed MSVC and GNU Rust toolchains both stop at linking build scripts. The source remains formatted and Cargo-resolvable, but a complete `cargo test` and Plasma executable inspection require a Windows C linker or a Linux build image with a C toolchain.
