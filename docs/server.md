# ThermAgent Server

ThermAgent Server is the planned self-hosted control plane for fleets and clusters.

Planned responsibilities:

- device and node inventory,
- policy distribution,
- rollout tracking,
- telemetry aggregation,
- decision audit logs,
- benchmark comparison.

Current MVP:

```sh
server/python/thermagent-server --listen 127.0.0.1:9980 --state /tmp/thermagent-server-state.json
```

API surface:

- `GET /healthz`
- `GET /api/v1`
- `GET` / `POST /api/v1/inventory`
- `GET` / `POST /api/v1/policies`
- `GET` / `POST /api/v1/rollouts`
- `GET` / `POST /api/v1/metrics`
- `GET /api/v1/audit`

Server coordinates. Edge enforces. Local safety always wins.
