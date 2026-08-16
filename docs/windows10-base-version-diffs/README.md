# Windows 10 BASE version-diff series (amd64)

This directory contains security-oriented WinDiff analyses of every adjacent
Windows 10 BASE release on amd64, ordered from the original 1507 release through
22H2. The companion [cross-version synthesis](00-cross-version-synthesis.md)
connects findings across the full series.

## Reports

1. [1507 → 1511](01-1507-to-1511.md)
2. [1511 → 1607](02-1511-to-1607.md)
3. [1607 → 1703](03-1607-to-1703.md)
4. [1703 → 1709](04-1703-to-1709.md)
5. [1709 → 1803](05-1709-to-1803.md)
6. [1803 → 1809](06-1803-to-1809.md)
7. [1809 → 1903](07-1809-to-1903.md)
8. [1903 → 1909](08-1903-to-1909.md)
9. [1909 → 2004](09-1909-to-2004.md)
10. [2004 → 20H2](10-2004-to-20H2.md)
11. [20H2 → 21H1](11-20H2-to-21H1.md)
12. [21H1 → 21H2](12-21H1-to-21H2.md)
13. [21H2 → 22H2](13-21H2-to-22H2.md)

## Scope and method

Each report compares the BASE image for the two named versions on amd64 and
uses the same security-relevant core set: `ntoskrnl.exe`, `ntdll.dll`,
`win32k.sys`, `win32kbase.sys`, `win32kfull.sys`, `ci.dll`, and `cng.sys`.
WinDiff databases were generated from the repository's canonical configuration
and diffed with the bundled deterministic `windiff_diff.py` workflow. Reports
interpret additions and removals for EDR, anti-cheat, and vulnerability-research
audiences, while separating observed names/fields from hypotheses about intent.

The BASE records for 1909 and the enablement-package releases can contain
component build numbers from the shared servicing baseline rather than a single
marketing-version build number. Each report therefore records the actual binary
metadata and treats small deltas as servicing-baseline changes, not proof that a
marketing release introduced no other user-visible functionality.

Raw databases and JSON deltas are reproducible scratch artifacts under the
repository's git-ignored `local/` directory; only the interpreted reports are
kept here.
