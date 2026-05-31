# ThermAgent Server

Planned self-hosted fleet and cluster control plane.

First usable server milestone:

- inventory API,
- policy store,
- agent check-in with desired policy assignment,
- rollout history,
- audit log,
- metrics ingestion adapter.

Run locally:

```sh
server/python/thermagent-server --listen 127.0.0.1:9980 --state /tmp/thermagent-server-state.json
curl http://127.0.0.1:9980/api/v1
```

Minimal policy distribution flow:

```sh
curl -X POST http://127.0.0.1:9980/api/v1/policies \
  -H 'content-type: application/json' \
  -d '{"name":"safe-vision","max_temp_c":72}'
curl -X POST http://127.0.0.1:9980/api/v1/rollouts \
  -H 'content-type: application/json' \
  -d '{"policy":"safe-vision","selector":{"profile":"jetson-nano"}}'
curl -X POST http://127.0.0.1:9980/api/v1/agents/check-in \
  -H 'content-type: application/json' \
  -d '{"node_id":"nano-1","inventory":{"profile":"jetson-nano"},"metrics":{"temperature_c":61.5}}'
```

The server must not bypass local safety checks in `thermagentd`.
