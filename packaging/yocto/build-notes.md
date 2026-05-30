# Yocto build notes

Reference target: Jetson Nano Lite / Jetson Nano-class boards.

Recommended starting point for the PoC is an L4T R32.x-compatible Yocto setup, for example OE4T `kirkstone-l4t-r32.7.x`. Keep the first build conservative: systemd, `nvpmodel`, CPU governor control and read-only telemetry.

Minimal integration:

```sh
bitbake-layers add-layer /path/to/thermagent/packaging/yocto/meta-thermagent
cat /path/to/thermagent/packaging/yocto/local.conf.fragment >> conf/local.conf
bitbake core-image-minimal
```

On target:

```sh
systemctl status thermagentd
journalctl -u thermagentd -f
thermagentctl validate /etc/thermagent/policy.d/jetson-nano-lite.policy
curl http://127.0.0.1:9920/metrics
```
