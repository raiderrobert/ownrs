# ownrs

Find out who owns what across your GitHub org.

Reconciles three ownership signals — CODEOWNERS, Backstage `catalog-info.yaml`, and GitHub team permissions — and tells you where they agree, disagree, or have gone stale.

## Install

```bash
curl -sSL https://raw.githubusercontent.com/raiderrobert/ownrs/main/install.sh | sh
```

Or with cargo:

```bash
cargo install --path .
```

Requires a GitHub token with `read:org` and `repo` scopes. If you have the [GitHub CLI](https://cli.github.com) installed and authenticated, ownrs picks up your token automatically. Otherwise:

```bash
export GITHUB_TOKEN=<your-token>
```

## Quick Start

```bash
# See the full picture for your org
ownrs org my-org --detail

# Which repos does my team own?
ownrs org my-org --team platform

# What's broken? Show stale and mismatched repos
ownrs org my-org --status stale,mismatched --format csv

# Check a single repo
ownrs repo my-org/my-repo
ownrs repo  # auto-detects from git remote
```

## What It Checks

ownrs compares three sources for each repo:

- **CODEOWNERS** — who reviews PRs (`* @org/team-name`)
- **catalog-info.yaml** — who's listed in the Backstage service catalog (`spec.owner`)
- **GitHub admin teams** — which teams actually have admin access to the repo

By default, if there's any team that appears across all present sources, the repo is **aligned**. Use `--strict` to require all sources to list the exact same teams.

## Alignment States

| Status | What it means |
|--------|--------------|
| `aligned` | Sources agree (at least one team in common) |
| `mismatched` | Sources present but no team overlap |
| `catalog-only` | Only `catalog-info.yaml` declares ownership |
| `codeowners-only` | Only `CODEOWNERS` declares ownership |
| `admin-only` | Team has admin access but no metadata files |
| `stale` | A referenced team no longer exists in the org |
| `missing` | No ownership signal at all |

## Options

```
Global:
  --refresh            Force re-fetch (bypass 24h cache)
  --strict             Require exact team set match across sources
  --cache-ttl <SECS>   Cache TTL in seconds (default: 86400)
  --token <TOKEN>      GitHub token (default: GITHUB_TOKEN env var)

org subcommand:
  --limit <N>          Audit only first N repos
  --sort <ORDER>       stale (default), active, name
  --team <TEAM>        Filter to repos where this team appears in any source
  --status <STATUS>    Filter by alignment status (comma-separated)
  --format <FMT>       table (default), csv, json
  --detail             Show per-repo breakdown

repo subcommand:
  --status <STATUS>    Filter by alignment status
  --format <FMT>       table (default), csv, json
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (or no repos matched filter) |
| 1 | Filter matched results |
| 2 | Runtime error |

## License

[PolyForm Shield 1.0.0](LICENSE)
