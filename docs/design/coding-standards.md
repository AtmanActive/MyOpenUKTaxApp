### Coding Standards
- Prefer existing dependencies over adding new ones when possible.
- For complex code, always consider using third-party libraries instead of writing new code that has to be maintained.
- Use keyword arguments instead of positional arguments when calling functions and methods.
- Always write code in Allman style (each new brace should go on a new line by itself whenever possible).
- Always use tabs for identation at the line begining.
- Always write variable and function names in all-lowercase snake_case.
- Always use descriptive sentence-long names for all variable and function names.
- Always use code comments as english language intention description before each and every code block. No need to describe function input and return parameters, but do describe why was function built there in the first place.
- Always use code comments at the begining of each and every file to describe what is the role of that file in the project.
- Always use type hints in any language which supports them

### Security
- Always write secure code.
- Never hardcode sensitive data.
- All user input must be validated.
- Never roll your own cryptography system.

### When stuck
- Ask a clarifying question, propose a short plan, or open a draft PR with notes.
- Do not push large speculative changes without confirmation.

### Test first mode
- when adding new features: write or update unit tests first, then code to green
- prefer component tests for UI state changes
- for regressions: add a failing test that reproduces the bug, then fix to green

### Development Worklog
Each and every development session needs to leave a living document in the directory docs/worklog. The filename needs to start with the current date and time and a short title of the session. The filename needs to use underscores instead of spaces. The file format needs to be markdown. 
