# Development

- This is an independent Synra plugin repository. Keep OS effects behind the Synra SDK.
- Use the pinned toolchain, commit Cargo.lock and pin SDK/CLI to the same reviewed core revision.
- The plugin is unpublished: refactor contracts directly without compatibility branches.
- Use `cargo fmt --all --check`, `cargo clippy --locked --all-targets -- -D warnings` and `cargo test --locked`.
- Build a real component with the pinned `synra-plugin build`, then run `synra-plugin verify`.
- During iteration, run affected tests first; run all repository checks at a phase boundary.
- Tests of parsers or gestures do not prove cross-device delivery or native OS effects.
- No release, tag, deploy or package publishing as part of development.
- Commits are GPG signed, English Conventional Commits subjects of at most 50 characters and no body.
