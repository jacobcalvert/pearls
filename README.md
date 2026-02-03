# Pearls

Pearls is a lightweight alternative to `beads` for managing long-running task graphs. I found `beads` to be more heavyweight than I needed, so Pearls strips it down to the bare necessities.

It is designed for AI agents (especially coding agents) to organize and plan long-term work. It uses a SQLite database as the backing store and a flock-compatible file lock to prevent simultaneous write access by multiple operators. Output is human-readable by default, with an optional `--json` flag for every command to make it machine-friendly.
It also includes an MCP stdio server so agents can call the same task operations over JSON-RPC.

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
- The MCP server uses the same database and locking behavior as the CLI.

## Configuration

- `PEARLS_DB`: optional path to the SQLite database. If unset, it defaults to `./pearls.db`.

## JSON Output

Add `--json` to any command to output JSON:

```bash
pearls --json tasks list
pearls --json tasks add --title "Example" --description "Example description"
```

## Using Pearls Without MCP (CLI)

Use the CLI directly for local or scripted workflows. The database path can be set with `--db` or `PEARLS_DB`.

```bash
# List tasks
pearls tasks list

# Claim the next ready task
pearls tasks claim-next

# Add a task
pearls tasks add --title "Add feature" --description "Describe the work"

# Close a task
pearls tasks update-metadata --id 1 --state closed

# Update dependencies
pearls tasks update-dependency --id 1 --add-child 2
```

## Using Pearls With MCP

Run the MCP stdio server and point your client at it. Use the same database path you use for the CLI so agents and humans share the same graph.

```bash
# Use PEARLS_DB or --db to control the database location
PEARLS_DB=./pearls.db pearls mcp serve
pearls --db ./pearls.db mcp serve
```

The MCP server exposes tools that mirror the CLI tasks subcommands:
`tasks.list`
`tasks.claim_next`
`tasks.add`
`tasks.update_metadata`
`tasks.update_dependency`

## MCP Server

See "Using Pearls With MCP" above for setup and tool names.
