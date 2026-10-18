# Release checklist

The release workflow publishes only version tags matching `vMAJOR.MINOR.PATCH`.

Before tagging:

1. `cargo fmt --check`
2. `cargo test --all-targets`
3. `cargo build --release`
4. `cargo run --release -- doctor`
5. Review the three-pane UI with keyboard navigation and a configured provider.

The GitHub Actions release job repeats the checks before publishing the Linux x86-64 `plasma` binary. Missing provider configuration is intentional and remains an honest empty state; a release must not bundle credentials or fabricated model IDs.
