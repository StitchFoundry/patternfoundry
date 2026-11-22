# Dependabot auto-merge

`.github/workflows/dependabot-automerge.yml` turns on GitHub auto-merge (squash) for safe
Dependabot pull requests. The workflow never merges anything itself: GitHub still waits for
every required status check and branch rule on `main`.

## When it enables auto-merge
Only for PRs opened by `dependabot[bot]`, and only when **every** dependency in the PR is:
- a `version-update:semver-patch` or `version-update:semver-minor` bump,
- not a 0.x **minor** bump (0.x minors are treated as breaking; 0.x patches are fine),
- not in the `github_actions` ecosystem (workflow-file bumps stay manual).

A grouped PR is checked dependency by dependency; one major or 0.x-minor bump in the group
means no auto-merge for the whole PR.

## When it does not
It does nothing except post one comment (marked, so it is never repeated) naming the
dependencies and reasons.

## Agent and human PRs
The job runs only when the PR author is `dependabot[bot]`. Pull requests from agents or
people are never auto-merged by it.

## Security notes
Trigger is `pull_request` (not `pull_request_target`), no secrets beyond the job's
`GITHUB_TOKEN` (`contents: write`, `pull-requests: write`, job only). Dependabot-supplied
values reach the shell through environment variables, never inline `${{ }}` expansion.
The metadata action is pinned to a full commit SHA.

## Turning it off
Delete the workflow file, or disable "Dependabot auto-merge" under the repository's Actions
tab. Pending auto-merges can be cancelled per PR with `gh pr merge --disable-auto <PR>`.

## Requirements
"Allow auto-merge" must be enabled in repository settings and `main` must have required
status checks, otherwise `gh pr merge --auto` fails (the job then shows red; nothing merges).
