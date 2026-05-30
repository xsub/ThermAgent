# Roadmap

## v0.1 — ThermAgent Edge

- Rust daemon: `thermagentd`.
- systemd unit and conservative hardening.
- Jetson Nano Lite / Jetson Nano-class policy.
- Telemetry: thermal zones, CPU frequency/governors, devfreq discovery, INA3221-style power sensors where exposed.
- Actuation: `nvpmodel -m`, CPU governor writes.
- Local Prometheus/OpenMetrics endpoint.
- Dry-run and single-sample modes.

## v0.2 — ThermAgent Lab

- Python policy validation.
- Trace replay and simulation.
- Workload hint writer.
- Benchmark report templates for FPS/W, joules/inference and thermal-throttling events.
- Policy schema validation.

## v0.3 — meta-thermagent

- Yocto/OpenEmbedded layer.
- Jetson Nano-class reference image integration.
- Optional Python bytecode precompilation for read-only-ish images.
- Hadron/Kairos packaging notes.

## v0.4 — ThermAgent Server

- Self-hosted API and UI skeleton.
- Fleet inventory.
- Policy distribution.
- Metrics aggregation.
- Rollout history and audit log.
- Server never bypasses local agent safety limits.

## v0.5 — ThermAgent Operator

- Kubernetes/K3s CRDs.
- DaemonSet packaging for `thermagentd`.
- `ThermAgentPolicy`, `ThermalProfile`, `PowerBudget` and `WorkloadHint` resources.
- Integration with ThermAgent Server as optional control plane.
