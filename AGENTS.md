# Agent Instructions
## Work Tracking Instructions
### Overview
Pearls is a lightweight CLI for managing a task graph. Pearls tasks can be assigned parents, children, and priorities. Parent tasks block child tasks and must be completed and closed before child tasks are ready to be worked.
Database path defaults to ./pearls.db and can be overridden with PEARLS_DB.
Use --json on any command to emit machine-readable output.

Commands:
- pearls tasks list [--state ready,blocked,in_progress,closed]
- pearls tasks claim-next [--assignee <ASSIGNEE>]
- pearls tasks add --title <title> --description <desc> [--assignee <ASSIGNEE>] [--parent-of <id>] [--child-of <id>] [--priority <num>]
- pearls tasks update-metadata --id <id> [--title <title>] [--desc <desc>] [--priority <num>] [--state <state>] [--assignee <ASSIGNEE>] [--no-assignee]
- pearls tasks update-dependency --id <id> [--add-child <id> ...] [--remove-child <id> ...]

### Workflow
- claim the next ready task with `pearls tasks claim-next`
- when done, close the task with `pearls tasks update-metadata`
- if any new subtasks need to be created as a result of working your in progress task, create them with `pearls tasks add` and make sure to set their dependencies appropriately

## Development Instructions 
- Always run `cargo clippy` and fix any issues found
- Always run `cargo test` and fix any issues found. 
