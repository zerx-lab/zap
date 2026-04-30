Read the contents of one or more files from the local filesystem.

Usage:
- The `files[].path` parameter should be an absolute path when possible. Relative paths are resolved against the working directory.
- Optionally pass `line_ranges` per file to read specific 1-indexed inclusive line ranges. To read later sections of a long file, call this tool again with a different range.
- Contents are returned with each line prefixed by its line number for clarity. For directories, entries are listed one per line.
- Lines longer than ~2000 characters may be truncated.
- Call this tool with multiple files in `files` (or call it multiple times in parallel) when you know there are several files you want to read.
- Avoid tiny repeated slices (30-line chunks). If you need more context, ask for a larger window.
- For very large files, prefer `grep` to locate the relevant section first, then `read_files` with a `line_ranges` covering that area.
- Use `file_glob` to look up filenames by pattern if you are unsure of the correct path.
- This tool can read text files, images, and PDFs and return them as attachments.
