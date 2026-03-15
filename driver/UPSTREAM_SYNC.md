# Upstream Sync Status

This is a vendored fork of [4ndv/mirajazz](https://github.com/4ndv/mirajazz) (MPL-2.0),
extended with a JSON-based device registry system. We intentionally stay on synchronous
`hidapi` and do not adopt upstream's async migration (v0.4.0+).

**Feature parity with upstream v0.10.0** (as of 2026-03-13):

## Ported from upstream

- `protocol_version` replacing `is_v2` boolean (v0.8.0)
- `set_mode()` for multimodal devices (v0.7.0)
- ACK prefix validation in `read_input()` (v0.9.0)
- Separate `supports_both_encoder_states` from button states (v0.9.0/v0.10.0)
- Buffer reuse optimization in `write_image_data_reports()` (v0.7.0)

## Not adopted (intentional)

- async-hid migration (v0.4.0) — we use synchronous hidapi
- DeviceWatcher (v0.5.0) — we handle device lifecycle in keydeck daemon
- DeviceQuery-based list_devices (v0.6.0) — replaced by our JSON registry
- Protocol version 0 fallback for very old firmware (v0.8.1) — no such devices in use
