# ThermAgent Edge

ThermAgent Edge is the local enforcement track.

Initial target: Jetson Nano Lite / Jetson Nano-class Linux systems.

Responsibilities:

- collect host telemetry,
- read workload hints,
- evaluate local policies,
- apply allowlisted power and thermal actions,
- expose local metrics,
- reject unsafe or unsupported actions.

Current Edge policy behavior includes thermal hysteresis, a short decision hold interval to reduce mode flapping, and a conservative hot-mode fallback when thermal telemetry is missing.

The local agent must keep final safety control even when policies are distributed by Server or Operator.
