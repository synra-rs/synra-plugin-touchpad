# Synra Touchpad

Phone touchpad plugin for Synra, with the identity `@synra-plugin/touchpad`.

This independent repository contains a Rust headless plugin scaffold and a pure gesture
recognizer in `src/gestures/`. The recognizer covers single-finger cursor movement, two-finger
scrolling, a left tap, a right two-finger tap, and double-tap hold to drag.

Recognition produces ordered intentions, not native mouse input. The Wasm guest currently
exports the lifecycle and refuses all actions; the recognizer is not yet connected to the
guest, phone UI, authenticated input sessions or a desktop input backend. No phone-to-PC
control is claimed by these tests.

## Development

Rust is pinned in `rust-toolchain.toml`. SDK and author CLI use core revision
`9c7c78b9788e5357ccb4537c9ff1f8d3abedea58`, with a committed Cargo.lock.

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo clippy --locked --target wasm32-unknown-unknown -- -D warnings
cargo install synra-plugin-cli --git https://github.com/synra-rs/synra-core.git --rev 9c7c78b9788e5357ccb4537c9ff1f8d3abedea58 --locked
synra-plugin build
synra-plugin verify
git diff --exit-code -- Cargo.lock
```

The CLI converts the SDK-generated module to a real component and verifies its binary ABI
and manifest. `plugin.wasm` and Cargo output are ignored. CI checks development artifacts
without releasing or publishing packages.

## Recognizer contract

- Feed complete snapshots of at most two contacts with stable ids, CSS-pixel coordinates and
  one controller's monotonic milliseconds; include stationary frames to detect a held second tap.
- Thresholds are centralized in `gestures.rs`: tap 250 ms, double-tap interval 300 ms,
  second-tap hold 120 ms, movement 8 px and maximum double-tap separation 24 px.
- Motion accumulates sub-threshold displacement; scrolling follows the two-contact centroid.
  Reordering contacts is harmless. Lifting two fingers sequentially still produces one right tap.
- A drag emits left down before motion and releases once on lift or cancel. Adding a finger
  during a drag releases and suppresses input until all contacts lift.
- Invalid contacts, replaced ids and clock reversal cancel the gesture; dispatch cleanup
  commands even when a step also contains a refusal. A new session starts with a fresh recognizer.
- Call `cancel()` on target changes, pointer cancellation, backgrounding, disconnection,
  permission revocation and plugin stop/update. This is an intention; receiver-side button
  release and lease expiry must work independently of the controller.

## Remaining integration

Build the Vue touch surface and a headless bridge, then connect the authenticated session
to shared native host input. Bind peer, target, full plugin id, session, grant generation and
sequence number. Require explicit PC approval; reject replay, expired sessions and revoked grants.

Start with a bounded 60 Hz input stream, accumulating relative deltas without crossing button
boundaries. Use 250 ms heartbeats and a 1 s receiver lease, with immediate disconnect/revocation
cleanup. Queue overflow or injection failure ends the session rather than retrying clicks.

Validate all five gestures on an actual phone and computer, including disconnect and cancel
while dragging. Record platform/network/load and sample-to-execution-ack RTT percentiles using
one monotonic clock; do not subtract unsynchronized device clocks for one-way latency.

See [the full official-plugin contract](https://github.com/synra-rs/design-docs/blob/main/29-official-plugins.md).
