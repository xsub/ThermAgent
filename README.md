# ThermAgent

**Adaptive thermal and power policy stack for edge-AI Linux devices and GPU nodes.**

ThermAgent is an open-source thermal and power policy stack for AI workloads on Linux. It starts with a small local agent for edge devices and grows into an optional fleet and cluster control plane.

The project is organized as one umbrella repository with four tracks:

- **ThermAgent Edge** — local Rust daemon for policy enforcement on edge-AI devices and GPU nodes.
- **ThermAgent Lab** — Python tooling for policy validation, trace replay, simulation and benchmark reports.
- **ThermAgent Server** — planned self-hosted control plane for fleet inventory, policy rollout, metrics aggregation and audit history.
- **ThermAgent Operator** — planned Kubernetes/K3s integration for policy rollout and workload hints.

Design rule:

```text
ThermAgent Server coordinates.
ThermAgent Operator integrates.
ThermAgent Lab validates.
ThermAgent Edge enforces.
Local safety always wins.
```

## Repository description for GitHub

```text
Adaptive thermal and power policy stack for edge-AI Linux devices and GPU nodes: Rust edge agent, Python policy lab, Yocto layer, and planned server/operator control plane.
```

Suggested GitHub topics:

```text
linux edge-ai power-management thermal-management yocto rust python jetson-nano embedded-linux nvidia-jetson systemd prometheus kubernetes k3s
```

## Current PoC

The current PoC targets Jetson Nano Lite / Jetson Nano-class boards.

It includes:

- `thermagentd`: std-only Rust daemon.
- `thermagentctl`: Python helper for validation, simulation and workload hints.
- `meta-thermagent`: Yocto/OpenEmbedded layer.
- systemd service with conservative hardening.
- Jetson Nano Lite reference policy using `nvpmodel` and CPU governors.

The PoC intentionally avoids direct GPU clocks, fan PWM, voltage, device tree and bootloader changes. It starts with safer controls: telemetry, thermal guard, `nvpmodel` and CPU governor selection.

## Quick local checks

```sh
lab/python/thermagentctl validate edge/policies/jetson-nano-lite.policy
lab/python/thermagentctl simulate edge/policies/jetson-nano-lite.policy --trace lab/traces/sample-jetson-camera.jsonl
```

Run the daemon once in dry-run mode:

```sh
cd edge/rust/thermagentd
cargo run -- --config ../../policies/jetson-nano-lite.policy --dry-run --once
```

Emit a workload hint on a target device:

```sh
thermagentctl emit-workload   --busy 1   --fps 12.5   --phase vision   --path /run/thermagent/workload.metrics
```

## Yocto

```sh
bitbake-layers add-layer /path/to/thermagent/packaging/yocto/meta-thermagent
cat /path/to/thermagent/packaging/yocto/local.conf.fragment >> conf/local.conf
bitbake core-image-minimal
```

See [`packaging/yocto/build-notes.md`](packaging/yocto/build-notes.md).

## Roadmap

See [`ROADMAP.md`](ROADMAP.md).

## License

Apache-2.0.
