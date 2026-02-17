# memory.md — Project Long-Term Memory

## Purpose
This file stores compressed, high-signal knowledge about the project.
It must stay short, current, and actionable.

## Update Rules
- Never append raw conversation logs.
- Always compress new information into existing sections.
- Prefer facts, decisions, constraints, and stable patterns over details.
- If something becomes obsolete, replace or delete it.
- Target size: as small as possible while preserving usefulness.

## Compression Heuristics
- Merge duplicates.
- Generalize specifics into rules or patterns.
- Keep examples only if they clarify a rule.
- Remove one-off experiments unless they define a new direction.

## When updating memory
Ask yourself:
1. Is this a stable fact about the project?
2. Does this change decisions, architecture, or constraints?
3. Can this be expressed as a rule instead of a story? If not — do not store it.

# MEMORY Structure

# memory.md — Project Long-Term Memory

## Project Summary
- One-paragraph description of what this project is and why it exists.

## Goals
- Bullet list of main goals (stable, not sprint tasks).

## Non-Goals
- What this project intentionally does NOT try to solve.

## Architecture & Tech
- Key architectural decisions
- Main technologies/languages/frameworks
- Important constraints (performance, platform, etc.)

## Key Design Decisions
- Decision: ...
  Rationale: ...
- Decision: ...
  Rationale: ...

## Coding & Style Conventions
- Rules that should be followed consistently.

## Performance & Reliability Constraints
- Hard limits, targets, or invariants.

## Learned Patterns
- What works well in this project
- What to avoid

## Open Questions / Risks
- Only long-term, structural uncertainties (not TODOs)

---

---


# Memory starts here ☯️ 🧠

## Compressed snapshot (2026-02-17)
- Event model: repo uses RawPacketModel / DeviceEvent / Packet; no SignalInfo.
- Pipeline: tokio mpsc channels (unbounded in src\lib.rs, bounded in src\windows_bluetooth.rs).
- Concurrency: pervasive Arc<Mutex<...>> for shared state; prefer bounded channels and Arc<[u8]>/Bytes for zero-copy AD data.

