Load a specialized skill when the task at hand matches one of the skills listed in the system prompt.

Use this tool to inject the skill's instructions and resources into the current conversation. The output may contain detailed workflow guidance as well as references to scripts, files, etc. in the same directory as the skill.

Pass the skill's `name` exactly as shown in the `<available_skills><skill><name>…</name></skill></available_skills>` block of your system prompt. If the system prompt does not currently list any skills, this tool is a no-op — do not call it.
