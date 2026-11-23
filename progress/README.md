# progress/ — one note per PR

Pull requests must **not** edit `PROGRESS.md`. Every PR touches that file at the same place, so
after one PR merges all the others conflict. Instead, each PR adds **one new file** here. New
files never conflict with each other.

## Naming

`progress/<yyyy-mm-dd>-<issue-or-pr-number>.md`, for example `progress/2026-11-22-212.md`. Use the
date the work was done and the issue number (or the PR number when there is no issue).

## Contents

Short, plain text, in the same style as the lines in `PROGRESS.md`:

```
- 2026-11-22 (issue 214): what was done and what it means for users; how it was checked.

Next:
1. First next step.
2. Second next step.
3. Third next step.
```

Nightly or maintenance runs use the same file name with `nightly` in place of the number if there
is no issue, for example `progress/2026-11-22-nightly.md`.

## Roll-up

`PROGRESS.md` stays the human-readable summary. Only the **progress roll-up PR** edits it (daily
or weekly, by the nightly/maintenance agent):

1. List the notes: `ls progress/*.md` (leave this `README.md` alone).
2. Append each note's "done" line to `PROGRESS.md`, oldest first.
3. Replace the next steps in `PROGRESS.md` with the latest note's next steps.
4. Delete the folded note files (`git rm`).
5. Open one PR titled "Progress roll-up <date>". It changes only `PROGRESS.md` and `progress/`.
