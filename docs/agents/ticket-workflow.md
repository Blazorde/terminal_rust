# Ticket-to-pull-request workflow

This guide defines the expected autonomous contribution flow.

## 1. Qualify the ticket

- Read the goal, suggested files and every acceptance criterion.
- Inspect the real state of `develop`; the work may already be partly complete.
- Identify dependencies and neighboring tickets.
- Ask only when different answers materially change the product or compatibility.

## 2. Define the batch

One ticket normally maps to one branch. Group tickets only when they share the
same technical boundary, cannot be validated usefully in isolation, and remain
easy to review and revert. Do not mix unrelated documentation, optional
refactors and product features.

## 3. Implement

- Branch from an up-to-date `develop`.
- Add tests with behavior.
- Preserve Windows, macOS and Linux compatibility.
- Update affected user and technical documentation.
- Add an ADR when a decision constrains later tickets.

## 4. Verify

Run the suite in [testing.md](../development/testing.md), then review the diff
for secrets or generated files, out-of-scope changes, consistent errors and
alignment between code and documentation.

## 5. Deliver

- Use an English Conventional Commit.
- Push the branch and open a pull request against `develop`.
- Include a summary and validation evidence.
- After merge, update local `develop` with `--ff-only`.

## 6. Update project tracking

Use `Not started` before work, `In progress` while active and `Done` only after
merge and synchronization. Record the pull-request link, decisions and limits.
Create a separate ticket for discovered follow-up work instead of silently
expanding scope.
