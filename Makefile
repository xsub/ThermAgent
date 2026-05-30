.PHONY: check check-operator check-packaging check-python check-server report simulate validate tree

validate:
	lab/python/thermagentctl validate edge/policies/jetson-nano-lite.policy --schema schemas/thermagent-policy.schema.json

simulate:
	lab/python/thermagentctl simulate edge/policies/jetson-nano-lite.policy --trace lab/traces/sample-jetson-camera.jsonl

report:
	lab/python/thermagentctl report edge/policies/jetson-nano-lite.policy --trace lab/traces/sample-jetson-camera.jsonl

check-python:
	python3 -m py_compile lab/python/thermagentctl
	python3 -m py_compile server/python/thermagent-server
	python3 -m py_compile operator/python/thermagent-operatorctl

check-packaging:
	cmp edge/rust/thermagentd/src/main.rs packaging/yocto/meta-thermagent/recipes-thermagent/thermagentd/files/thermagentd/src/main.rs
	cmp lab/python/thermagentctl packaging/yocto/meta-thermagent/recipes-thermagent/thermagent-lab/files/thermagentctl
	cmp edge/policies/jetson-nano-lite.policy packaging/yocto/meta-thermagent/recipes-thermagent/thermagentd/files/jetson-nano-lite.policy
	cmp edge/policies/jetson-nano-lite.policy examples/jetson-nano-lite/jetson-nano-lite.policy

check-server:
	python3 server/python/test_thermagent_server.py

check-operator:
	python3 operator/python/test_thermagent_operatorctl.py
	operator/python/thermagent-operatorctl render-policy --base-policy edge/policies/jetson-nano-lite.policy --resource operator/crds/thermagentpolicy.example.yaml >/dev/null
	operator/python/thermagent-operatorctl render-workload-hint --resource operator/samples/workloadhint.example.yaml >/dev/null

check: check-python validate simulate report check-packaging check-server check-operator

tree:
	find . -maxdepth 4 -type f | sort
