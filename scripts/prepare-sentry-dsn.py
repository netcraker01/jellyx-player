#!/usr/bin/env python3
"""Create the ignored, compile-time Sentry DSN source without logging its value."""

import json
import os
from pathlib import Path


dsn = os.environ.get("JELLYX_SENTRY_DSN", "")
if not dsn.strip():
    # The DSN is optional. When the repository secret is absent or empty
    # (PR/push CI builds, or releases without a configured Sentry project),
    # skip embedding any DSN: no file is created, build.rs never sets the
    # `jellyx_sentry_dsn` cfg, and telemetry stays completely off.
    raise SystemExit(0)

path = Path("jellyx-desktop/.sentry-dsn.rs")
path.write_text(f"Some({json.dumps(dsn)}.to_owned())\n", encoding="utf-8")
path.chmod(0o600)
