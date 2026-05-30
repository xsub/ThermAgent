# Hadron/Kairos packaging notes

ThermAgent Edge should be included as a host-level systemd service in immutable edge images.

Recommended production image contents:

```text
/usr/sbin/thermagentd
/etc/thermagent/policy.d/jetson-nano-lite.policy
/usr/lib/systemd/system/thermagentd.service
/var/lib/thermagent
/run/thermagent
```

Keep Python tooling out of tiny production images unless policy simulation is required on-device.

See `thermagent.edge.manifest.yaml` for a concrete host package manifest skeleton.
