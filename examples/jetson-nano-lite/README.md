# Jetson Nano Lite reference profile

This profile is the first ThermAgent Edge PoC target.

It uses conservative controls:

- `nvpmodel` mode 1 for idle and thermal guard,
- `nvpmodel` mode 0 for active vision workload,
- CPU governor selection,
- local thermal limits,
- thermal hysteresis and short decision hold to avoid rapid mode flapping,
- policy allowlists for CPU governors and `nvpmodel` modes,
- `/var/lib/thermagent/state.json` with the latest decision state.

Run locally:

```sh
thermagentctl validate jetson-nano-lite.policy
```
