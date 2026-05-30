# ThermAgent Server

Planned self-hosted fleet and cluster control plane.

This directory is intentionally a skeleton in v0.1. The first usable server milestone should include:

- inventory API,
- policy store,
- rollout history,
- audit log,
- metrics ingestion adapter.

The server must not bypass local safety checks in `thermagentd`.
