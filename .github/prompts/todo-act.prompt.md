---
mode: agent
model: Claude Sonnet 4
---

Find the first unchecked (sub?)item in TODO.md and act on it.
Start by checking if it is still applicable.
If the task appears to be larger or more complex, break it down into smaller, actionable subtasks and add these as subtasks to TODO.md. Then, ask the user for feedback on the proposed subtasks before proceeding. Be sure to incorporate the user's feedback by updating the subtasks as needed, rather than starting work immediately.
If the task appears small and straightforward, complete it immediately.
If the following task is closely related and also small, proceed with it as well; otherwise, stop and await further instructions.
If your solution to the task differs from the original description in the todo, add a brief comment to the task explaining how you addressed it.
Before marking a top level item as complete, verify that you have run all checks as detailed in your instructions file.
Whenever you defer a task for future implementation, add a corresponding item to TODO.md describing what remains to be done.
