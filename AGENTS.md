# Agent Instructions
## Work Tracking Instructions
### Overview
Pearls is a lightweight CLI for managing a task graph.
Database path defaults to ./pearls.db and can be overridden with `--db <path>` or `PEARLS_DB`.
Use `--json` on any command to emit machine-readable output.

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

## Development Instructions 
- Always run `cargo clippy` and fix any issues found
- Always run `cargo test` and fix any issues found. 
