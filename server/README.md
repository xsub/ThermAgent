# ThermAgent Server

Planned self-hosted fleet and cluster control plane.

First usable server milestone:

- inventory API,
- policy store,
- rollout history,
- audit log,
- metrics ingestion adapter.

Run locally:

```sh
server/python/thermagent-server --listen 127.0.0.1:9980 --state /tmp/thermagent-server-state.json
curl http://127.0.0.1:9980/api/v1
```

The server must not bypass local safety checks in `thermagentd`.
