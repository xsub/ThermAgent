# Architecture

ThermAgent is split into four tracks rather than long-lived Git branches.

```text
ThermAgent
  ThermAgent Edge      local enforcement agent
  ThermAgent Lab       policy validation, simulation and benchmarking
  ThermAgent Server    self-hosted fleet and cluster control plane
  ThermAgent Operator  Kubernetes/K3s integration
```

The safety model is local-first:

```text
Server coordinates.
Operator integrates.
Lab validates.
Edge enforces.
Local safety always wins.
```

## ThermAgent Edge

`thermagentd` runs on the host as a small privileged daemon. It collects telemetry and applies only allowlisted actions. The initial Jetson Nano-class backend supports `nvpmodel` and CPU governor selection.

## ThermAgent Lab

Python tools validate policies, replay traces, simulate decisions and generate reports. Python is optional on tiny production images.

## ThermAgent Server

The planned server stores inventory, policies, rollout history, metrics and audit logs. It distributes intent, not raw hardware writes.

## ThermAgent Operator

The planned operator maps Kubernetes/K3s resources into local ThermAgent policies and workload hints.
