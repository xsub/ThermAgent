# ThermAgent Operator

ThermAgent Operator is the planned Kubernetes/K3s integration.

Planned CRDs:

- `ThermAgentPolicy`,
- `ThermalProfile`,
- `PowerBudget`,
- `WorkloadHint`.

The operator should never write hardware controls directly. It should distribute policies and hints to local ThermAgent Edge agents.
