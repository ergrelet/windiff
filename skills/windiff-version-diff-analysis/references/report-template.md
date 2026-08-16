# Report template

Use this structure. Lead with the highest-signal security findings, group by
component and feature, and give every nontrivial item an interpretation — not just
a name. Drop sections that have no findings rather than padding them.

```markdown
# WinDiff analysis: <binary(ies)> — <old version> → <new version> (<arch>)

## Scope
- Old: <version / update / build> · New: <version / update / build> · Arch: <arch>
- Binaries compared: <list>
- Focus: <syscalls / mitigations / attack surface / component / general>

## Executive summary
3–6 bullets: the most security-relevant changes and the overall theme of the
update (e.g. "hardening pass on win32k", "new process mitigation for X", "new
EtwTi event for EDR", "expanded ETW telemetry"). State confidence where it matters.
Tag findings with the audience that should care: **[EDR]**, **[anti-cheat]**,
**[vuln-research]** (a finding may carry more than one tag).

## New syscalls
For each (ntoskrnl `Nt*` and win32k `NtUser*`/`NtGdi*`):
- **`NtXxx`** [id N] — <subsystem from prefix>. <hypothesis on purpose>.
  Paired ntdll stub: yes/no. Impact: <new attack surface / capability>. **[tags]**.
  Confirm by: <reverse the handler / check public symbols>.
(Note renumbering only if relevant to the audience; don't list it as a feature.)

## New / changed security mitigations
For each new bitfield, enum value, or policy field:
- **<struct>::<field>** (in <binary>) — <which mitigation it implements>. **[tags]**.
  Width/offset if shown. Audit-only vs enforcing if inferable. Companion change
  in <other binary/struct> if any. Impact + how to confirm.

## New security-relevant features & components (beyond mitigations)
This section exists so EDR and anti-cheat findings aren't buried under mitigations.
See `references/windows-internals.md` §7. Cover, where present:
- **Detection & telemetry** — new ETW providers/events, especially `EtwTi*`
  threat-intelligence channels. **[EDR]** (often **[anti-cheat]**). What new
  event/visibility appeared, or what blind spot was closed.
- **Notification callbacks** — new/extended `Ps*NotifyRoutine`, `ObRegisterCallbacks`,
  `CmRegisterCallback*` surface. **[EDR][anti-cheat]**.
- **Code integrity / signing / boot trust** — `Ci*`/WDAC/ELAM/PPL-signer changes.
  **[EDR][vuln-research]**.
- **Process/object/handle & anti-tamper hardening** — **[anti-cheat][vuln-research]**.
- **VBS / secure kernel** — **[all]**.
- **New drivers/modules/components** — a new binary or a cluster under a new prefix;
  hypothesize its role and flag it for follow-up reversing.
For each: <component → subsystem → likely function → who cares and why → confirm-by>.

## New attack surface (routines, callbacks, object types)
Grouped by component (Ps/Ke/Mm/Se/Ci/...). For each notable routine or type:
- **`Name`** — <component → subsystem → likely function → security relevance>. **[tags]**.

## Notable structure changes
New named fields in core structs (`_EPROCESS`, `_TOKEN`, CI policy, ...) that
reveal an existing object gaining capability. Discount `_unnamed_0x...`-only diffs.

## Removed / deprecated
Anything retired, with a note on why it might have been removed.

## Research leads
Concrete next steps for a researcher: which routines to reverse, which findings
are hypotheses needing confirmation, suggested tooling (IDA/Ghidra/BinDiff,
public symbol servers, Microsoft docs).

## Appendix: raw counts
Per binary: +added / -removed / ~modified for exports, symbols, syscalls, types.
```

## Interpretation quality bar

- Never present a bare list of symbol names as the analysis. Names without
  interpretation are what the raw diff already provides; the skill's job is the *why*.
- Tie each finding to a subsystem via its prefix and to a plausible feature.
- Separate what the data *shows* (a name/field exists) from what you *infer*
  (its purpose). Use hedged language for inferences.
- Prefer a few well-reasoned, high-signal findings over an exhaustive dump. If
  there are many similar additions, characterize the group and call out exemplars.
