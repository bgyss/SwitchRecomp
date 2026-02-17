# LLM-Assisted Decompilation Long-Tail Notes

## Source
- Chris Lewis, "The Long Tail of LLM-Assisted Decompilation", published 2026-02-16:
  - https://blog.chrislewis.au/the-long-tail-of-llm-assisted-decompilation/

## Why This Matters Here
SwitchRecomp is moving from early pipeline bring-up to harder tail work where decode/lift/cleanup progress can stall. This source gives concrete operational patterns for that stage and highlights failure classes we should model explicitly.

## Extracted Findings
1. Difficulty-only prioritization stops working in the long tail.
   - Once only hard functions remain, reordering by estimated "easy first" provides little gain.
   - Similarity-guided retrieval of already-solved neighbors improved throughput.
2. Exact opcode-sequence similarity can be practical at this scale.
   - With only thousands of candidates, bounded Levenshtein-style opcode distance is feasible and can complement embedding retrieval.
3. Specialist tooling changes outcomes for domain-heavy code.
   - Graphics macro/disassembly helpers and well-scoped skills improved performance on project-specific patterns.
4. Cleanup work improves future matching.
   - Cleaner matched functions become better exemplars for similarity-guided attempts.
5. Unattended loops need hard technical guardrails, not prompt-only policy.
   - Hooks blocking protected edits, skipped tests, and disallowed build paths prevented silent corruption.
6. Parallel worktrees plus orchestration are required for scale.
   - Structured task runners with candidate queues, retries, and sharding reduce operator overhead.
7. Long-tail blockers cluster by function type.
   - Very large functions, graphics-macro-heavy logic, and matrix/vector math became recurring stall classes.

## Implications for SwitchRecomp
- Candidate selection should use deterministic similarity traces in automation metadata, not ad hoc manual ordering.
- Automation should explicitly separate lanes (`general`, `gfx`, `math`, `cleanup`) with per-lane retry budgets.
- Cleanup/documentation passes are not optional "polish"; they are dependency quality improvements for later automated attempts.
- Guardrails should be enforced by hooks and policy checks before commit/publish paths.
- Run manifests and triage reports should track stall categories and attempt percentiles to prevent invisible long-tail drift.

## Proposed Integration Map
- `specs/SPEC-210-AUTOMATED-RECOMP-LOOP.md`:
  - add similarity-guided candidate ordering, lane metadata, and long-tail metrics.
- `specs/SPEC-250-AUTOMATION-SERVICES.md`:
  - add candidate-selection service and long-tail event requirements.
- `specs/SPEC-260-AGENT-PIPELINE-SECURITY.md`:
  - add guarded edits, required checks, scope limits, and retry budget controls.
- `PLANS.md`:
  - add cross-cutting long-tail work items and escalation rules.
- `RESEARCH.md`:
  - add a dedicated research direction for long-tail strategy and this source entry.

## Limits and Cautions
- This is a field report from one N64 project, not a controlled benchmark.
- Similarity retrieval can amplify brittle patterns if exemplars are low quality.
- Retry and orchestration wins can increase cost unless explicit ceilings and escalation rules exist.
