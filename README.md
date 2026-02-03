# Pearls

Pearls is a lightweight alternative to `beads` for managing long-running task graphs. I found `beads` to be more heavyweight than I needed, so Pearls strips it down to the bare necessities.

It is designed for AI agents (especially coding agents) to organize and plan long-term work. It uses a SQLite database as the backing store and a flock-compatible file lock to prevent simultaneous write access by multiple operators. Output is human-readable by default, with an optional `--json` flag for every command to make it machine-friendly.

## Installation

### Install via Cargo

```bash
cargo install pearls
```

### Download a Release Binary

1. Download the archive for your platform from the GitHub Releases page.
2. Extract the archive.
3. Make the binary executable (macOS/Linux only):

```bash
chmod +x pearls
```

4. Move it somewhere on your `PATH` (example):

```bash
mv pearls /usr/local/bin/pearls
```

## Intended Use

- Add the usage snippet to your `AGENTS.md` so your agent(s) know how to use Pearls.
- Store the database somewhere convenient (configurable via `PEARLS_DB`).
- Load your task graph.
- Turn your agent(s) loose.

## AGENTS.md Snippet

Add this to your `AGENTS.md` file:

```text
## Work Tracking Instructions
### Overview
Pearls is a lightweight CLI for managing a task graph.
Database path defaults to ./pearls.db and can be overridden with PEARLS_DB.
Use --json on any command to emit machine-readable output.

Commands:
- pearls tasks list [--state ready,blocked,in_progress,closed]
- pearls tasks claim-next
- pearls tasks add --title <title> --description <desc> [--parent-of <id>] [--child-of <id>] [--priority <num>]
- pearls tasks update-metadata --id <id> [--title <title>] [--desc <desc>] [--priority <num>] [--state <state>]
- pearls tasks update-dependency --id <id> [--add-child <id> ...] [--remove-child <id> ...]

### Workflow
- claim the next ready task with `pearls tasks claim-next`
- when done, close the task with `pearls tasks update-metadata`
    - YOU MUST ALWAYS CLOSE THE TASK AT THE END OF YOUR SESSION
- if any new subtask need to be created as a result of working your in progress task, create them with `pearls tasks add`
```

## Behavior Notes

- `tasks list` includes parent and child IDs for each task.
- A task is reported as `blocked` if any of its parents are not `closed`.
- `tasks list` defaults to `ready,blocked,in_progress` and accepts a comma-separated `--state` list (include `closed` explicitly if you want it).
- Writes (`add`, `update-metadata`, `update-dependency`) take an exclusive file lock. Reads do not.

## Configuration

- `PEARLS_DB`: optional path to the SQLite database. If unset, it defaults to `./pearls.db`.

## JSON Output

Add `--json` to any command to output JSON:

```bash
pearls --json tasks list
pearls --json tasks add --title "Example" --description "Example description"
```
