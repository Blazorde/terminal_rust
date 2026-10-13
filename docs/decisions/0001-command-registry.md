# ADR-0001 — Central built-in command registry

- Status: Accepted
- Date: 2026-10-09
- Decision makers: Terminal Rust team

## Context

Commands must feed dispatch, help and completion without separate lists drifting.

## Decision

Declare built-ins in `COMMANDS` in `src/app/command.rs`. Each entry defines a
name, syntax, summary, argument limit and handler. `command_names()` feeds
completion and `command_help()` generates help from the same source. Domain logic
stays in `src/commands/`; handlers only adapt arguments, state and errors.

## Options considered

- One large runner `match`: initially simple but mixes lifecycle and commands.
- Separate execution/help/completion registries: explicit but easy to desynchronize.
- Dynamic plugins: premature for the MVP.

## Consequences

The runner remains stable, completion exposes executable names and shared
argument validation is centralized. Handlers must fit a shared signature, and
external contributions require an isolated adapter.

## Validation

A new command must execute, appear in completion and be documented by `help`.
