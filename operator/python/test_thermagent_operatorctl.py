#!/usr/bin/env python3
from __future__ import annotations

import importlib.machinery
import importlib.util
import pathlib
import unittest


def load_operator_module():
    path = pathlib.Path(__file__).with_name("thermagent-operatorctl")
    loader = importlib.machinery.SourceFileLoader("thermagent_operatorctl", str(path))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return module


operatorctl = load_operator_module()
ROOT = pathlib.Path(__file__).resolve().parents[2]


class OperatorCtlTests(unittest.TestCase):
    def test_render_policy_applies_resources(self) -> None:
        policy = operatorctl.parse_policy(ROOT / "edge/policies/jetson-nano-lite.policy")
        for resource in [
            ROOT / "operator/crds/thermagentpolicy.example.yaml",
            ROOT / "operator/samples/thermalprofile.example.yaml",
            ROOT / "operator/samples/powerbudget.example.yaml",
        ]:
            operatorctl.apply_policy_resource(policy, operatorctl.load_resource(resource))

        rendered = operatorctl.render_policy(policy)

        self.assertIn("name: jetson-nano-lite-safe-vision", rendered)
        self.assertIn("max_temp_c: 72", rendered)
        self.assertIn("active_nvpmodel_mode: 0", rendered)
        self.assertIn("hot_cpu_governor: powersave", rendered)

    def test_render_workload_hint(self) -> None:
        resource = operatorctl.load_resource(ROOT / "operator/samples/workloadhint.example.yaml")
        body = operatorctl.render_workload_hint(resource)

        self.assertIn("busy=1", body)
        self.assertIn("fps=12.500", body)
        self.assertIn("phase=vision", body)


if __name__ == "__main__":
    unittest.main()
