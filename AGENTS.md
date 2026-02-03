# Agent Instructions
## Work Tracking Instructions
### Overview
Pearls is a lightweight CLI for managing a task graph.
Database path defaults to ./pearls.db and can be overridden with PEARLS_DB.
Use --json on any command to emit machine-readable output.

Commands:
- pearls tasks list [--state ready|blocked|in_progress|closed]
- pearls tasks ready
- pearls tasks add --title <title> --description <desc> [--parent-of <id>] [--child-of <id>] [--priority <num>]
- pearls tasks update-metadata --id <id> [--title <title>] [--desc <desc>] [--priority <num>] [--state <state>]
- pearls tasks update-dependency --id <id> [--add-parent <id>] [--remove-parent <id>] [--add-child <id>] [--remove-child <id>]

### Workflow
- see ready tasks with `pearls ready`
- pick a task by ID and mark it in-progress with `pearles update-metadata`
    - YOU MUST MARK IT IN PROGRESS IMMEDIATELY
- when done, close the task with `pearls update-metadata`
    - YOU MUST ALWAYS CLOSE THE TASK AT THE END OF YOUR SESSION
- start this process over again

## Development Instructions 
- Always run `cargo clippy` and fix any issues found
- Always run `cargo test` and fix any issues found. 