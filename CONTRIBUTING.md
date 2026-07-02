# Contributing to Helix

Thanks for your interest in Helix! This guide covers how to report bugs, suggest features, build the project, and submit changes.

## Quick links

- [Issue tracker](https://github.com/netcraker01/helix/issues)
- [Releases](https://github.com/netcraker01/helix/releases)
- [Architecture overview](docs/ARCHITECTURE.md)
- [Building from source](docs/BUILDING.md)
- [Packaging guide](docs/packaging.md)
- [Design tokens](assets/brand/design-tokens.json)

## How to contribute

1. **Open an issue first** for bugs, features, or large refactors so we can align on direction.
2. **Fork the repository** and create a branch from `main`.
3. **Make your changes** following the conventions below.
4. **Test your changes** — run the app locally and make sure the relevant flows still work.
5. **Open a pull request** with a clear description and a link to the related issue.

## Development setup

See [docs/BUILDING.md](docs/BUILDING.md) for full instructions. The short version:

```bash
git clone https://github.com/netcraker01/helix
cd helix
cargo tauri dev
```

## Code conventions

- Rust: follow `cargo fmt` and `cargo clippy`.
- Svelte/TypeScript: keep components small and focused; follow the existing feature-based structure under `ui/src/features/`.
- UI copy and public comments are in **English**.
- Commit messages use [Conventional Commits](https://www.conventionalcommits.org/):
  - `feat:` for new features
  - `fix:` for bug fixes
  - `docs:` for documentation
  - `chore:` for maintenance tasks
  - `ci:` for CI/build changes

## Design and branding

If your change touches UI, visuals, or public-facing assets, check the brand system:

- Colors, gradients, and typography: [`assets/brand/design-tokens.json`](assets/brand/design-tokens.json)
- CSS variables: [`assets/brand/theme.css`](assets/brand/theme.css)
- Logo SVG: [`assets/brand/icon.svg`](assets/brand/icon.svg)

## Contributor License Agreement

By contributing to Helix, you agree to the [Contributor License Agreement (CLA)](CLA.md), which grants the project owner permission to include your contribution under both AGPL-3.0 and commercial licenses. You retain ownership of your work and will be credited in [AUTHORS.md](AUTHORS.md).

## Pull requests

- Open an issue first for large changes so we can align on direction.
- Keep PRs focused on a single concern.
- Include manual verification steps or test results.
- Reference the issue your PR closes.

## Questions?

Open a [discussion](https://github.com/netcraker01/helix/discussions) or reach out via the issue tracker.

Thank you for helping make Helix better!
