#!/usr/bin/env python3
"""Verify the least-privilege candidate-validation workflow contract."""

from pathlib import Path

import yaml


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github" / "workflows" / "validate-candidate.yml"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"{WORKFLOW}: {message}")


workflow = yaml.load(WORKFLOW.read_text(), Loader=yaml.BaseLoader)
require(isinstance(workflow, dict), "must parse as a YAML mapping")

triggers = workflow.get("on")
require(isinstance(triggers, dict), "must define pull_request and push triggers")
require("pull_request" in triggers, "must trigger on pull_request")
require(triggers.get("push", {}).get("branches") == ["main"], "must trigger on pushes to main only")
require(workflow.get("permissions") == {"contents": "read"}, "must grant only contents: read")

jobs = workflow.get("jobs")
require(isinstance(jobs, dict) and set(jobs) == {"validate-candidate"}, "must define one validation job")
job = jobs["validate-candidate"]
require(job.get("name") == "Validate candidate", "job name must be 'Validate candidate'")
require(workflow.get("concurrency", {}).get("cancel-in-progress") == "true", "must cancel superseded runs")

steps = job.get("steps", [])
checkout = next((step for step in steps if step.get("uses") == "actions/checkout@v4"), None)
require(checkout is not None, "must use actions/checkout@v4")
require(checkout.get("with", {}).get("persist-credentials") == "false", "checkout must not persist credentials")

commands = "\n".join(step.get("run", "") for step in steps)
for command in (
    "cargo fmt --all -- --check",
    "cargo check --workspace",
    "cargo test --workspace",
    "pnpm install --frozen-lockfile",
    "pnpm test",
    "pnpm run check",
    "pnpm run build",
    "python scripts/test-release-workflow.py",
    "python scripts/test-windows-icons.py",
):
    require(command in commands, f"missing validation command: {command}")

for forbidden in ("git commit", "git push", "gh release", "secrets.", "contents: write", "agent"):
    require(forbidden not in WORKFLOW.read_text(), f"must not contain {forbidden!r}")

print("candidate validation workflow contract passed")
