# ADR-0003 — Segment-based prompt engine

- Status: Accepted
- Date: 2026-10-09
- Decision makers: Terminal Rust team

## Context

An Oh My Posh-inspired prompt needs paths, Git, styles and configuration without
turning `app/prompt.rs` into untestable string concatenation.

## Decision

Collect data into `PromptContext`, then render an ordered list of independent
segments. A segment may disappear when data or capabilities are missing. Themes
convert semantic segments to final text. Git context is cached for 250 ms and a
directory change forces refresh.

## Options considered

- Direct string construction: low initial cost and rapid design debt.
- Expression templates: flexible but too complex and risky for the MVP.
- Typed segments: testable, extensible and compatible with declarative settings.

## Consequences and validation

The model enables terminal-free tests, graceful fallback and configurable order
and themes, at the cost of more types and careful width/cache rules. Validation
covers clean, dirty and unavailable Git, plus plain rendering without color or
Nerd Fonts and injected-time cache tests.
