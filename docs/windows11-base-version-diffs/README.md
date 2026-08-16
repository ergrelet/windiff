# Windows 11 version-diff series (amd64)

This directory contains security-oriented WinDiff analyses of adjacent Windows
11 releases on amd64. The companion
[cross-version synthesis](00-cross-version-synthesis.md) connects findings across
the supported series.

## Reports

1. [21H2 → 22H2](01-21H2-to-22H2.md)
2. [22H2 → 23H2](02-22H2-to-23H2.md)
3. [23H2 → 24H2](03-23H2-to-24H2.md)
4. [24H2 → 25H2](04-24H2-to-25H2.md)
5. [25H2 → 26H1 KB5077179](05-25H2-to-26H1.md)

## Scope and method

The first four reports compare BASE images for the named versions. The 26H1
BASE/ISO artifacts are unavailable from Winbindex, so the final report uses the
first chronological 26H1 cumulative update instead: KB5077179, released
February 10, 2026 as OS build 28000.1575. Every report is amd64-only and uses the
same security-relevant core set: `ntoskrnl.exe`, `ntdll.dll`, `win32k.sys`,
`win32kbase.sys`, `win32kfull.sys`, `ci.dll`, and `cng.sys`.

WinDiff databases were generated from the repository's canonical configuration
and diffed with the bundled deterministic `windiff_diff.py` workflow. Reports
interpret additions and removals for EDR, anti-cheat, and vulnerability-research
audiences while separating observed names and fields from hypotheses about
intent.

The BASE records for 23H2 and 25H2 contain serviced binaries from the shared
22621 and 26100 baselines respectively. Each report records the actual binary
metadata and treats these as servicing/enablement comparisons rather than
assuming a completely new kernel baseline.

Although `ci/db_configuration.json` contains a `11-26H1 / BASE / amd64` entry,
Winbindex has none of the seven required BASE binaries. The final report clearly
labels the resulting BASE-to-update scope and records the exact selected file
metadata; it must not be interpreted as an isolated KB5077179 patch diff.

Raw databases and JSON deltas are reproducible scratch artifacts under the
repository's git-ignored `local/` directory; only interpreted reports are kept
here.
