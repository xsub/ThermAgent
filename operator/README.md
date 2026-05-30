# ThermAgent Operator

Planned Kubernetes/K3s integration.

The operator should translate Kubernetes resources into ThermAgent policies and workload hints consumed by local agents.

Current MVP artifacts:

- `crds/thermagent.io_crds.yaml`: CRD definitions for `ThermAgentPolicy`, `ThermalProfile`, `PowerBudget` and `WorkloadHint`.
- `python/thermagent-operatorctl`: dependency-free translator for local testing and early K3s integration.
- `samples/`: example resources.

Render a local policy:

```sh
operator/python/thermagent-operatorctl render-policy \
  --base-policy edge/policies/jetson-nano-lite.policy \
  --resource operator/crds/thermagentpolicy.example.yaml
```

Render workload hints:

```sh
operator/python/thermagent-operatorctl render-workload-hint \
  --resource operator/samples/workloadhint.example.yaml
```
