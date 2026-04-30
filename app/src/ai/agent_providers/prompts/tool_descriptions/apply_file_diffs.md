Batch-edit files. Each entry in `operations` is one of:

- `edit` — Perform an exact string replacement in an existing file. Provide the file path, the existing string to replace (`old_string`), the new string (`new_string`), and `replace_all` if every occurrence should be replaced.
- `create` — Create a new file at the given path with the given content. Fails if the file already exists.
- `delete` — Delete the file at the given path.

# Edit guidelines

- You MUST have read the file with `read_files` at least once in this conversation before editing it. Editing without reading first will fail.
- Preserve the exact indentation (tabs/spaces) of the surrounding lines. The `old_string` must match byte-for-byte.
- The edit fails if `old_string` is not found, or if it appears more than once and `replace_all` is false. In the latter case, either pass a longer `old_string` with more surrounding context, or set `replace_all: true` if you really want to change every match.
- ALWAYS prefer editing existing files to creating new ones.
- NEVER proactively create documentation files (`*.md`, `README*`) unless the user explicitly asked.
- Only use emojis if the user explicitly requested it. Avoid emojis in code unless asked.

# Create guidelines

- This is the right tool for writing brand-new files. Do not use a shell `cat <<EOF` heredoc or `echo >` for this.
- Default to ASCII when creating files. Only introduce non-ASCII characters when there is a clear justification.

# Delete guidelines

- Be conservative — only delete files when the user explicitly asked or when it's an obvious cleanup of files you yourself just created and no longer need.

# Batching

You can pass multiple operations in a single call. They will be applied atomically (or as close as the underlying file system allows). Prefer batching when you are making related edits across files in the same task.
