#!/usr/bin/env python3
"""Create the ignored, compile-time Sentry DSN source without logging its value."""

import json
import os
from pathlib import Path


dsn = os.environ.get("JELLYX_SENTRY_DSN", "")
if not dsn.strip():
    # PR/push CI builds do not have access to the release Sentry DSN secret.
    # Skip embedding the DSN; the release pipeline enforces its presence at
    # .github/workflows/release.yml before invoking this workflow.
    raise SystemExit(0)

path = Path("jellyx-desktop/.sentry-dsn.rs")
path.write_text(f"Some({json.dumps(dsn)}.to_owned())\n", encoding="utf-8")
path.chmod(0o600)
