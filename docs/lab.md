# ThermAgent Lab

ThermAgent Lab contains Python tooling for people writing and testing policies.

Current tool:

```sh
lab/python/thermagentctl validate edge/policies/jetson-nano-lite.policy --schema schemas/thermagent-policy.schema.json
lab/python/thermagentctl simulate edge/policies/jetson-nano-lite.policy --trace lab/traces/sample-jetson-camera.jsonl
lab/python/thermagentctl report edge/policies/jetson-nano-lite.policy --trace lab/traces/sample-jetson-camera.jsonl
```

Reports summarize FPS/W, estimated joules/inference when power data is present, thermal guard events and policy decision counts.

Lab is optional on tiny images. For production edge images, `thermagentd` should be able to run without Python.
