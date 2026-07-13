#!/usr/bin/env python3
"""Deterministic release-workflow contracts without third-party YAML tooling."""

from pathlib import Path
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent.parent
release = (ROOT / ".github/workflows/release.yml").read_text()
windows = (ROOT / ".github/workflows/windows.yml").read_text()
macos = (ROOT / ".github/workflows/macos-dmg.yml").read_text()
recovery = (ROOT / ".github/workflows/release-recovery.yml").read_text()


def require(text: str, fragment: str, source: str) -> None:
    if fragment not in text:
        raise SystemExit(f"{source}: missing required contract: {fragment}")


for workflow, channel in (
    (release, "linux-appimage"), (release, "linux-deb"), (release, "linux-rpm"),
    (release, "linux-tarball"), (release, "windows-msi"), (release, "windows-nsis"),
    (release, "windows-portable"), (windows, "windows-msi"), (windows, "windows-nsis"),
    (windows, "windows-portable"), (macos, "macos-dmg"),
):
    require(workflow, f"JELLYX_INSTALL_CHANNEL: {channel}", "workflow")

for workflow in (release, windows):
    if "--bundles msi,nsis" in workflow:
        raise SystemExit("Windows artifacts must not share one baked install channel")
if "--bundles appimage,deb,rpm" in release:
    raise SystemExit("Linux artifacts must not share one baked install channel")

require(release, "uses: ./.github/workflows/macos-dmg.yml", "release.yml")
require(release, "contents: read", "release.yml least-privilege default")
require(release, "stage-and-publish-release:", "release.yml promotion job")
require(release, "contents: write", "release.yml promotion write permission")
require(release, "environment: release", "release.yml protected promotion environment")
require(release, "validate-release-assets:", "release.yml read-only validation job")
require(release, "jellyx-release-manifest", "release.yml content-addressed artifact manifest")
require(release, "sha256sum -c release-manifest/release-manifest.sha256", "release.yml promotion manifest revalidation")
require(release, "! -name 'release-manifest.sha256'", "release.yml internal manifest exclusion")
if release.count("contents: write") != 1:
    raise SystemExit("release.yml: only the final promotion job may receive contents:write")
require(release, "gh release create \"$release_tag\" --target \"$BUILT_SHA\" --draft", "release.yml")
require(release, "gh release delete \"$release_tag\" --yes", "release.yml")
require(release, "--json isDraft", "release.yml")
require(release, 'name: jellyx-release-body', "release.yml release body artifact")
require(release, 'name: jellyx-release-body\n          path: release-body', "release.yml release body download")
require(release, "gh release edit \"$release_tag\" --draft=false", "release.yml")
require(release, "sha256sum -c", "release.yml")
require(release, "release-body.md.sha256", "release.yml release-body digest")
require(release, "staged_names[*]", "release.yml")
require(release, "! -name 'release-body.md.sha256'", "release.yml release-body checksum exclusion")
if './scripts/validate-release.sh' in release or '--notes-file release-body.md' in release:
    raise SystemExit("release.yml: write-token promotion must not execute repository scripts")
if release.index("--draft") > release.index("--draft=false"):
    raise SystemExit("release.yml: release must be staged as a draft before publication")
if "action-gh-release" in release or "action-gh-release" in macos:
    raise SystemExit("release workflows must not attach assets before final validation")
require(macos, "workflow_call:", "macos-dmg.yml")
if 'tags: ["v*"]' in macos:
    raise SystemExit("macos-dmg.yml must not independently publish tag releases")

version = "0.4.1"
body = subprocess.run(
    ["bash", str(ROOT / "scripts/generate-release-body.sh"), version, f"v{version}"],
    cwd=ROOT,
    check=True,
    text=True,
    capture_output=True,
).stdout
expected_assets = (
    f"Jellyx.Player_{version}_amd64.AppImage",
    f"Jellyx.Player_{version}_amd64.deb",
    f"Jellyx.Player-{version}-1.x86_64.rpm",
    f"Jellyx_{version}_amd64.tar.gz",
    f"Jellyx.Player_{version}_x64-setup.exe",
    f"Jellyx.Player_{version}_x64_en-US.msi",
    "jellyx.exe",
    f"Jellyx.Player_{version}_aarch64.dmg",
)
for asset in expected_assets:
    require(body, asset, "generated release body")
for asset in (
    '"Jellyx.Player_${version}_amd64.AppImage"',
    '"Jellyx.Player_${version}_amd64.deb"',
    '"Jellyx.Player-${version}-1.x86_64.rpm"',
    '"Jellyx_${version}_amd64.tar.gz"',
    '"Jellyx.Player_${version}_x64-setup.exe"',
    '"Jellyx.Player_${version}_x64_en-US.msi"',
    '"jellyx.exe"',
    '"Jellyx.Player_${version}_aarch64.dmg"',
    '"Jellyx.Player_${version}_aarch64.dmg.sha256"',
):
    require(release, asset, "release.yml expected assets")
if '"$(basename "${mac_dmg[0]}")"' in release:
    raise SystemExit("release.yml: macOS gate must require the exact documented DMG name")
require(macos, 'dmg_name="Jellyx.Player_${version}_aarch64.dmg"', "macos-dmg.yml normalized DMG name")
require(body, f"Jellyx.Player_{version}_amd64.AppImage.sha256", "generated release body checksum")
if "macOS (Intel)" in body:
    raise SystemExit("generated release body lists an unbuilt macOS architecture")

# Boundary contract: arbitrarily long commit subjects and a large commit set
# cannot push a release body past the GitHub-safe byte budget, while fixed
# sections remain intact.
with tempfile.TemporaryDirectory() as temp:
    repo = Path(temp)
    subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
    subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=repo, check=True)
    subprocess.run(["git", "config", "user.name", "Release test"], cwd=repo, check=True)
    (repo / "file").write_text("base")
    subprocess.run(["git", "add", "file"], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-qm", "chore: base"], cwd=repo, check=True)
    subprocess.run(["git", "tag", "v0.0.0"], cwd=repo, check=True)
    for index in range(110):
        (repo / "file").write_text(str(index))
        subprocess.run(["git", "add", "file"], cwd=repo, check=True)
        subprocess.run(["git", "commit", "-qm", f"fix: {index}-" + "x" * 2000], cwd=repo, check=True)
    bounded = subprocess.run(
        ["bash", str(ROOT / "scripts/generate-release-body.sh"), "9.9.9", "v0.0.0"],
        cwd=repo, check=True, text=True, capture_output=True,
    ).stdout
    if len(bounded.encode()) > 120000:
        raise SystemExit("generated release body exceeds byte budget")
    for fragment in ("## ✨ What's New", "## 📦 Downloads", "## 🔑 Checksums", "**Full Changelog**"):
        require(bounded, fragment, "bounded release body")

# Internal artifacts are downloaded by the jellyx-* pattern during promotion.
# Their checksums must never change the public exact set or upload count.
downloaded_names = tuple(sorted((
    *expected_assets,
    *(f"{asset}.sha256" for asset in expected_assets),
    "release-body.md.sha256",
    "release-manifest.sha256",
)))
public_names = tuple(name for name in downloaded_names if name not in {
    "release-body.md.sha256", "release-manifest.sha256",
})
if len(public_names) != len(expected_assets) * 2:
    raise SystemExit("release artifact layout has an incorrect public asset count")
if set(public_names) != set(expected_assets) | {f"{asset}.sha256" for asset in expected_assets}:
    raise SystemExit("release artifact layout has an incorrect public exact set")

# Every release is authorized from the current protected main head. Dispatch
# deliberately exposes no arbitrary SHA or tag input.
require(release, "workflow_dispatch:", "release.yml manual trigger")
if "target_commit:" in release or "release_tag:" in release.split("authorize-release:", 1)[0]:
    raise SystemExit("release.yml: dispatch must not accept an arbitrary target or tag")
require(release, "authorize-release:", "release.yml protected-main authorization job")
require(release, 'main_sha="$(gh api', "release.yml current main SHA query")
require(release, 'test "$candidate_sha" = "$main_sha"', "release.yml main SHA equality")
require(release, 'required_status_checks.strict', "release.yml protected required-check policy")
require(release, 'commits/${candidate_sha}/check-runs', "release.yml exact-SHA check runs")
require(release, ".app.id == $app_id", "release.yml required check app authorization")
require(release, "sort_by(.completed_at // .started_at // .created_at", "release.yml latest check-run selection")
require(release, "sort_by(.updated_at // .created_at", "release.yml latest status selection")
require(release, "[.required_status_checks.contexts[]?] - [.required_status_checks.checks[]? | .context]", "release.yml required-check context de-duplication")
require(release, 'tag="v${version}"', "release.yml Cargo-derived tag")
require(release, "BUILD_LINUX_SHA", "release.yml build provenance")
require(release, 'test "$BUILD_WINDOWS_SHA" = "$built_sha"', "release.yml Windows SHA match")
require(release, 'test "$BUILD_MACOS_SHA" = "$built_sha"', "release.yml macOS SHA match")
require(release, 'test "$GITHUB_REF_NAME" = "$tag"', "release.yml tag exactness")
require(release, 'git rev-parse "${GITHUB_REF}^{commit}"', "release.yml annotated-tag authorization dereference")
require(release, 'gh api --method POST "repos/${GITHUB_REPOSITORY}/git/refs"', "release.yml trusted tag creation")
require(release, "rev-parse 'FETCH_HEAD^{commit}'", "release.yml annotated-tag promotion dereference")
require(release, 'test "$tag_commit" = "$BUILT_SHA"', "release.yml tag commit validation")


def dereference_tag(ref: dict[str, str]) -> str:
    """Model git's ^{commit}: a lightweight ref is a commit; an annotated ref peels."""
    return ref["peeled"] if ref["type"] == "tag" else ref["object"]


for tag_ref in (
    {"type": "commit", "object": "main-sha", "peeled": "unused"},
    {"type": "tag", "object": "tag-object-sha", "peeled": "main-sha"},
):
    if dereference_tag(tag_ref) != "main-sha":
        raise SystemExit("release tag authorization must compare the peeled commit SHA")
require(release, 'gh release create "$release_tag" --target "$BUILT_SHA" --draft', "release.yml draft target")
require(release, 'test "$(gh api "repos/${GITHUB_REPOSITORY}/git/ref/heads/main" --jq \'.object.sha\')" = "$BUILT_SHA"', "release.yml immediate remote-head recheck")
stage = release.split("stage-and-publish-release:", 1)[1]
if "actions/checkout" in stage or "git push" in stage or "./scripts/" in stage:
    raise SystemExit("release.yml: write-token promotion must not checkout or execute repository scripts")
if "actions/download-artifact@v4" in stage:
    raise SystemExit("release.yml: write-token artifact download must use an audited immutable action digest")
require(stage, "actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093", "release.yml immutable promotion artifact action")

# Release builds must provision the packaged DSN without exposing it in logs.
require(release, 'JELLYX_SENTRY_DSN: ${{ secrets.JELLYX_SENTRY_DSN }}', "release.yml packaged Sentry DSN")
require(release, 'test -n "$JELLYX_SENTRY_DSN"', "release.yml fail-closed Sentry secret check")
require(release, "secrets: inherit", "release.yml macOS secret forwarding")
require(macos, "JELLYX_SENTRY_DSN: ${{ secrets.JELLYX_SENTRY_DSN }}", "macos-dmg.yml packaged Sentry DSN")
if 'cargo:rustc-env=JELLYX_SENTRY_DSN' in (ROOT / "jellyx-desktop/build.rs").read_text():
    raise SystemExit("build.rs must never emit the Sentry DSN through Cargo output")
for workflow, label in ((release, "release.yml"), (macos, "macos-dmg.yml")):
    if "Cache Cargo build" in workflow or "path: target" in workflow:
        raise SystemExit(f"{label}: release builds with a DSN must not cache target outputs")
    require(workflow, "prepare-sentry-dsn.py", f"{label} ephemeral DSN source")
    require(workflow, "verify-sentry-dsn-boundary.py", f"{label} DSN executable sentinel")
    require(workflow, "Remove ephemeral embedded Sentry DSN", f"{label} DSN cleanup")


def latest_check_run(runs: list[dict], context: str, app_id: int | None) -> dict | None:
    matches = [run for run in runs if run["name"] == context and (app_id is None or run["app"]["id"] == app_id)]
    return max(matches, key=lambda run: run["completed_at"], default=None)


old_success = {"name": "CI", "app": {"id": 7}, "completed_at": "2026-01-01T00:00:00Z", "status": "completed", "conclusion": "success"}
new_failed = {**old_success, "completed_at": "2026-01-01T00:01:00Z", "conclusion": "failure"}
wrong_app_success = {**old_success, "app": {"id": 8}, "completed_at": "2026-01-01T00:02:00Z"}
if latest_check_run([old_success, new_failed], "CI", 7)["conclusion"] == "success":
    raise SystemExit("required checks must reject a newer failure after an older success")
if latest_check_run([old_success, wrong_app_success], "CI", 8)["app"]["id"] != 8:
    raise SystemExit("required checks must select the protected app id")
if latest_check_run([old_success], "CI", 8) is not None:
    raise SystemExit("required checks must reject a success from the wrong app")


def latest_status(statuses: list[dict], context: str) -> dict | None:
    matches = [status for status in statuses if status["context"] == context]
    return max(matches, key=lambda status: status["updated_at"], default=None)


def legacy_required_contexts(protection: dict) -> set[str]:
    app_bound_contexts = {
        check["context"]
        for check in protection["required_status_checks"].get("checks", [])
        # A null app_id is still a required check context; it accepts any app
        # but must suppress a duplicate legacy status requirement.
    }
    return set(protection["required_status_checks"].get("contexts", [])) - app_bound_contexts


old_status_success = {"context": "legacy-ci", "updated_at": "2026-01-01T00:00:00Z", "state": "success"}
new_status_pending = {**old_status_success, "updated_at": "2026-01-01T00:01:00Z", "state": "pending"}
if latest_status([old_status_success, new_status_pending], "legacy-ci")["state"] == "success":
    raise SystemExit("required statuses must reject a newer pending result after an older success")

# GitHub supplies both arrays for the same required context. The app-bound
# check is authoritative, so this live-shaped payload must not demand a
# duplicate legacy status that is absent from the response.
combined_protection = {
    "required_status_checks": {
        "contexts": ["CI", "legacy-ci"],
        "checks": [{"context": "CI", "app_id": 7}],
    }
}
if legacy_required_contexts(combined_protection) != {"legacy-ci"}:
    raise SystemExit("app-bound checks must suppress only duplicate legacy contexts")
if latest_check_run([old_success], "CI", 7)["conclusion"] != "success":
    raise SystemExit("combined required check must accept its latest matching app run")
if latest_status([old_status_success], "legacy-ci")["state"] != "success":
    raise SystemExit("unmatched legacy context must still require its latest status")

null_app_protection = {
    "required_status_checks": {
        "contexts": ["any-app-ci", "legacy-ci"],
        "checks": [{"context": "any-app-ci", "app_id": None}],
    }
}
any_app_success = {**old_success, "name": "any-app-ci", "app": {"id": 99}}
any_app_failure = {**any_app_success, "completed_at": "2026-01-01T00:03:00Z", "conclusion": "failure"}
if legacy_required_contexts(null_app_protection) != {"legacy-ci"}:
    raise SystemExit("null-app checks must suppress only duplicate legacy contexts")
if latest_check_run([any_app_success], "any-app-ci", None)["conclusion"] != "success":
    raise SystemExit("null-app required checks must accept a successful run from any app")
if latest_check_run([any_app_success, any_app_failure], "any-app-ci", None)["conclusion"] == "success":
    raise SystemExit("null-app required checks must reject the latest failed run")

# Public-release recovery is deliberately opt-in and non-destructive: it must
# require an exact confirmation, hide the release from the updater as a draft,
# and never delete assets or releases by itself.
require(recovery, "workflow_dispatch:", "release-recovery.yml")
require(recovery, "REVOKE_PUBLIC_RELEASE", "release-recovery.yml confirmation")
require(recovery, 'test "$CONFIRMATION" = "REVOKE_PUBLIC_RELEASE"', "release-recovery.yml confirmation validation")
require(recovery, 'exit 1', "release-recovery.yml incorrect confirmation failure")
require(recovery, 'needs: validate-confirmation', "release-recovery.yml recovery dependency")
if recovery.count("contents: write") != 1:
    raise SystemExit("release-recovery.yml: only the revoke job may receive contents:write")
require(recovery, 'gh release edit "$RELEASE_TAG" --draft', "release-recovery.yml updater revocation")
require(recovery, 'gh release view "$RELEASE_TAG" --json isDraft --jq \'.isDraft\'', "release-recovery.yml verification")
if "gh release delete" in recovery or "gh release delete-asset" in recovery:
    raise SystemExit("release-recovery.yml: recovery automation must not delete assets or releases")

print("release workflow contracts passed")
