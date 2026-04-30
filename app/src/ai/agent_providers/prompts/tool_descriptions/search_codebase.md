Semantic search over the indexed codebase using embeddings + symbol index.

- Takes a natural-language query and returns the most relevant code chunks/symbols across the project.
- Use this when you don't know exactly what to grep for — for example, "where is request retry logic" or "how does authentication flow work".
- For exact identifier or string lookup, prefer `grep`. For navigating by filename, prefer `file_glob`.
- This tool is only available in local sessions where the codebase has been indexed. If it returns empty for a clearly-relevant query, the index may not be ready — fall back to `grep` + `file_glob`.
