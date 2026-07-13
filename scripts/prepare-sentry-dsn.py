#!/usr/bin/env python3
"""Create the ignored, compile-time Sentry DSN source without logging its value."""

import json
import os
from pathlib import Path


dsn = os.environ.get("JELLYX_SENTRY_DSN", "")
if not dsn.strip():
    raise SystemExit("JELLYX_SENTRY_DSN is required")

path = Path("jellyx-desktop/.sentry-dsn.rs")
path.write_text(f"Some({json.dumps(dsn)}.to_owned())\n", encoding="utf-8")
path.chmod(0o600)
