Use this tool to create and manage a structured task list for the current coding session. The list is shown to the user in a chip + popup so they can track progress on complex multi-step work.

## When to use

- Tasks that require 3+ distinct steps or actions.
- User provides multiple things to do (numbered or comma-separated).
- After receiving new instructions — capture them as todos immediately.
- After completing a step — mark it `completed` and add follow-ups discovered during work.

## When NOT to use

- A single trivial step.
- Purely conversational / informational requests.
- Tasks that finish in less than 3 trivial actions.

## Status semantics

Each todo has a `status`:
- `pending` — not started yet.
- `in_progress` — currently working on it. Only one todo should be in_progress at a time.
- `completed` — finished. Stays in the list as a completed item.
- `cancelled` — abandoned (treated like completed for display).

## Calling rules

- **Always pass the full list** every call. This tool overwrites the entire todo list with the `todos` array you provide. Do not pass just the diff.
- The order of items in the array is the display order.
- Use stable, descriptive `content` strings — items are deduplicated by content, so changing the wording will be treated as a different item and reset its history.
- Mark a todo `in_progress` *before* starting work on it; mark it `completed` *immediately* when done — don't batch.

## Example

```json
{
  "todos": [
    { "content": "Read the failing test in foo_tests.rs", "status": "completed" },
    { "content": "Identify the root cause", "status": "in_progress" },
    { "content": "Fix the bug", "status": "pending" },
    { "content": "Run cargo test", "status": "pending" }
  ]
}
```
