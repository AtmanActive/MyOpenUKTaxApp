The MyOpenUKTaxApp is a cross-platform desktop Tauri2 app.
The UI framework is Tauri 2 + Vite + React + TypeScript + Tailwind CSS + shadcn/ui.
The UI code should include all material icons.
The database is SQLite.
All source code should live in the directory `src`.
Build CI/CD should be handled by Github to create automatic releases on each and every build.
Build versioning should follow source code versioning which is defined in `version.txt`.
On each new build, the version in `version.txt` should be incremented by one.
No double digits should ever be used in version number, if a minor number is at 9, then we increase the next major number and reset the minor number to 0.
