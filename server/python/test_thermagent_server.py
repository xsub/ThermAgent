#!/usr/bin/env python3
from __future__ import annotations

import importlib.machinery
import importlib.util
import pathlib
import tempfile
import unittest


def load_server_module():
    path = pathlib.Path(__file__).with_name("thermagent-server")
    loader = importlib.machinery.SourceFileLoader("thermagent_server", str(path))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return module


server = load_server_module()


class StateStoreTests(unittest.TestCase):
    def test_store_records_inventory_policy_rollout_metrics_and_audit(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            store = server.StateStore(pathlib.Path(tmp) / "state.json")

            inventory = store.upsert_inventory({"node_id": "nano-1", "profile": "jetson-nano"})
            policy = store.upsert_policy({"name": "safe-vision", "max_temp_c": 72})
            rollout = store.create_rollout({"policy": "safe-vision", "selector": {"profile": "jetson-nano"}})
            metrics = store.ingest_metrics({"node_id": "nano-1", "metrics": {"temperature_c": 61.5}})

            self.assertEqual(inventory["node_id"], "nano-1")
            self.assertTrue(policy["local_safety_required"])
            self.assertEqual(rollout["status"], "pending")
            self.assertEqual(metrics["metrics"]["temperature_c"], 61.5)
            self.assertEqual(len(store.state["audit"]), 4)

            reloaded = server.StateStore(pathlib.Path(tmp) / "state.json")
            self.assertIn("nano-1", reloaded.state["inventory"])
            self.assertIn("safe-vision", reloaded.state["policies"])

    def test_rollout_requires_policy_name(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            store = server.StateStore(pathlib.Path(tmp) / "state.json")
            with self.assertRaisesRegex(ValueError, "requires policy"):
                store.create_rollout({})


if __name__ == "__main__":
    unittest.main()
