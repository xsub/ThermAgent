# Jetson Nano Lite reference profile

This profile is the first ThermAgent Edge PoC target.

It uses conservative controls:

- `nvpmodel` mode 1 for idle and thermal guard,
- `nvpmodel` mode 0 for active vision workload,
- CPU governor selection,
- local thermal limits.

Run locally:

```sh
thermagentctl validate jetson-nano-lite.policy
```
