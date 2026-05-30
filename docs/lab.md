# ThermAgent Lab

ThermAgent Lab contains Python tooling for people writing and testing policies.

Current tool:

```sh
lab/python/thermagentctl validate edge/policies/jetson-nano-lite.policy
lab/python/thermagentctl simulate edge/policies/jetson-nano-lite.policy --trace lab/traces/sample-jetson-camera.jsonl
```

Lab is optional on tiny images. For production edge images, `thermagentd` should be able to run without Python.
