# Pearls

Pearls is a lightweight alternative to `beads` for managing long-running task graphs. I found `beads` to be more heavyweight than I needed, so Pearls strips it down to the bare necessities.

It is designed for AI agents (especially coding agents) to organize and plan long-term work. It uses a SQLite database as the backing store and a flock-compatible file lock to prevent simultaneous write access by multiple operators. Output is human-readable by default, with an optional `--json` flag for every command to make it machine-friendly.

## Intended Use

- Add the usage snippet to your `AGENTS.md` so your agent(s) know how to use Pearls.
- Store the database somewhere convenient (configurable via `PEARLS_DB`).
- Load your task graph.
- Turn your agent(s) loose.

## AGENTS.md Snippet

Add this to your `AGENTS.md` file:

```text
Pearls is a lightweight CLI for managing a task graph.
Database path defaults to ./pearls.db and can be overridden with PEARLS_DB.
Use --json on any command to emit machine-readable output.

Commands:
- pearls tasks list [--state ready|blocked|in_progress|closed]
- pearls tasks ready
- pearls tasks add --title <title> --description <desc> [--parent-of <id>] [--child-of <id>] [--priority <num>]
- pearls tasks update-metadata --id <id> [--title <title>] [--desc <desc>] [--priority <num>] [--state <state>]
- pearls tasks update-dependency --id <id> [--add-parent <id>] [--remove-parent <id>] [--add-child <id>] [--remove-child <id>]
```

## Behavior Notes

- `tasks list` includes parent and child IDs for each task.
- A task is reported as `blocked` if any of its parents are not `closed`.
- Writes (`add`, `update-metadata`, `update-dependency`) take an exclusive file lock. Reads do not.

## Configuration

- `PEARLS_DB`: optional path to the SQLite database. If unset, it defaults to `./pearls.db`.

## JSON Output

Add `--json` to any command to output JSON:

```bash
pearls --json tasks list
pearls --json tasks add --title "Example" --description "Example description"
```
