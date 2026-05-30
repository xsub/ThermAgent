.PHONY: check-python simulate validate tree

validate:
	lab/python/thermagentctl validate edge/policies/jetson-nano-lite.policy

simulate:
	lab/python/thermagentctl simulate edge/policies/jetson-nano-lite.policy --trace lab/traces/sample-jetson-camera.jsonl

check-python:
	python3 -m py_compile lab/python/thermagentctl

tree:
	find . -maxdepth 4 -type f | sort
