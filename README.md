# ownrs

Three-way ownership reconciliation across GitHub CODEOWNERS, Backstage `catalog-info.yaml`, and GitHub Teams.

Finds repos where ownership metadata is missing, mismatched, or references teams that no longer exist.

## Install

```bash
cargo install --path .
```

## Usage

Audit an entire org:

```bash
ownrs org my-org
ownrs org my-org --detail
ownrs org my-org --status stale,mismatched --format csv
ownrs org my-org --team legacy-team
```

Audit a single repo:

```bash
ownrs repo my-org/my-repo
ownrs repo  # auto-detects from git remote
```

## Alignment States

| Status | Meaning |
|--------|---------|
| `aligned` | Both sources present, match, team exists |
| `mismatched` | Both present but disagree on owner |
| `catalog-only` | Only `catalog-info.yaml` has ownership |
| `codeowners-only` | Only `CODEOWNERS` has ownership |
| `stale` | Referenced team(s) don't exist in GitHub |
| `missing` | No ownership metadata anywhere |

## Authentication

Set `GITHUB_TOKEN` or pass `--token`. The token needs `read:org` and `repo` scopes.

## Options

```
Global:
  --refresh            Force re-fetch (bypass 24h cache)
  --cache-ttl <SECS>   Cache TTL in seconds (default: 86400)
  --token <TOKEN>      GitHub token (default: GITHUB_TOKEN env var)

org subcommand:
  --limit <N>          Audit only first N repos
  --sort <ORDER>       stale (default), active, name
  --team <TEAM>        Filter to repos referencing this team
  --status <STATUS>    Filter by alignment status (comma-separated)
  --format <FMT>       table (default), csv, json
  --detail             Show per-repo breakdown

repo subcommand:
  --status <STATUS>    Filter by alignment status
  --format <FMT>       table (default), json
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All repos pass (or no repos matched filter) |
| 1 | Filter matched — drift detected |
| 2 | Runtime error |

## License

[PolyForm Shield 1.0.0](LICENSE)
