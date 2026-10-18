# NS Plasma

NS Plasma is a Rust-native coding workspace with the supplied NS logo, an integrated file explorer/editor, a provider-backed agent panel, and a matching headless CLI.

The current MVP deliberately uses an honest empty state: it never invents credentials or model IDs. Configure any OpenAI-compatible endpoint through the UI or CLI:

```text
plasma init
plasma provider set --base-url http://localhost:11434/v1 --model <model-id>
plasma provider models
plasma -p "Explain the current project"
```

Remote credentials are read only from the environment variable named by `--api-key-env`; they are never written to `.plasma/settings.json`.

## Build and test

```text
cargo fmt --check
cargo test
cargo run -- doctor
cargo run
```

The app uses the provided [`assets/logo.png`](assets/logo.png) and displays the `NS Plasma` wordmark in the desktop header. Synced reference files under `sources/` are intentionally left untouched.

## Scope

This is the first working Rust foundation for the larger roadmap in the supplied master prompt. The provider adapter, config boundary, workspace-safe file operations, GUI shell, and CLI are implemented first; plugin hosting, MCP UI, persistent sessions, and OS sandboxing remain later milestones.
