#!/usr/bin/env python3
"""Assert that the configured DSN is embedded in a final executable only.

This intentionally emits neither the DSN nor a byte excerpt, including on
failure, so it is safe to run in a release job.
"""

import os
import sys
from pathlib import Path


dsn = os.environ.get("JELLYX_SENTRY_DSN", "").encode()
if not dsn:
    # PR/push CI builds do not embed the release Sentry DSN, so there is no
    # boundary to verify. The release pipeline enforces its presence at
    # .github/workflows/release.yml before invoking this workflow.
    raise SystemExit(0)

for candidate in map(Path, sys.argv[1:]):
    if candidate.is_file():
        if dsn not in candidate.read_bytes():
            raise SystemExit("final executable does not contain the configured Sentry DSN")
        raise SystemExit(0)

raise SystemExit("final executable was not found for Sentry DSN boundary verification")
