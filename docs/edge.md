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

The local agent must keep final safety control even when policies are distributed by Server or Operator.
