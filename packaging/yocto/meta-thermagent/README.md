# meta-thermagent

Yocto/OpenEmbedded layer for ThermAgent Edge.

This layer packages:

- `thermagentd`: Rust daemon for local thermal and power policy enforcement.
- `thermagent-lab`: optional Python helper for validation, simulation and workload hints.
- `packagegroup-thermagent`: convenience package group.

Reference target for the PoC is a Jetson Nano-class board using an L4T R32.x-compatible Yocto stack, such as OE4T `kirkstone-l4t-r32.7.x`.

```sh
bitbake-layers add-layer /path/to/thermagent/packaging/yocto/meta-thermagent
IMAGE_INSTALL:append = " packagegroup-thermagent"
bitbake core-image-minimal
```
