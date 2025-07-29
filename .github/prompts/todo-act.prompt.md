---
mode: agent
model: Claude Sonnet 4
---

Find the first unchecked (sub?)item in TODO.md and act on it.
Start by checking if it is still applicable.
If it looks like a bigger task first split it up into subtasks and add add those as subtasks to TODO.md, then ask the user for feedback on the subtasks before continuing.
If it looks like a small task just do it.
Stop after you've completed the item, unless it was a small change the next item looks directly related and also looks small.
Make sure you have followed all the instructions from your instructions file before considering the task complete.
