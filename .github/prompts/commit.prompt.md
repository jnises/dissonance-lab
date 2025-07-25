---
mode: agent
model: GPT-4.1
---

Commit all pending changes to git.

Before composing the commit message, run `git diff HEAD` changes. Base the commit message on the actual code and file changes about to be committed, not just the contents of this conversation.

Write the commit message in good git style: a concise summary on the first line, then a blank line, then details if needed.

Always wrap the entire commit message in quotes.

Escape all special characters (such as backticks, quotes, dollar signs, etc.) so the commit command is valid and does not break.

Double-check that no unescaped characters remain in the message.

If `TODO.md` is among the changes, include it in the commit but do not mention it in the commit message. Also don't mention that you shouldn't mention TODO.md. In fact you must not say the word TODO.md at all!
