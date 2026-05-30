# ThermAgent Operator

ThermAgent Operator is the planned Kubernetes/K3s integration.

Planned CRDs:

- `ThermAgentPolicy`,
- `ThermalProfile`,
- `PowerBudget`,
- `WorkloadHint`.

Current MVP:

```sh
kubectl apply -f operator/crds/thermagent.io_crds.yaml
operator/python/thermagent-operatorctl render-policy \
  --base-policy edge/policies/jetson-nano-lite.policy \
  --resource operator/crds/thermagentpolicy.example.yaml
operator/python/thermagent-operatorctl render-workload-hint \
  --resource operator/samples/workloadhint.example.yaml
```

The operator should never write hardware controls directly. It should distribute policies and hints to local ThermAgent Edge agents.
