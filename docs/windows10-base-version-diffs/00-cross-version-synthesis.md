# Windows 10 BASE security evolution: 1507 → 22H2 (amd64)

## Scope

This synthesis connects the 13 adjacent-version reports in this directory. Every
comparison uses the amd64 BASE entry and the same seven binaries:
`ntoskrnl.exe`, `ntdll.dll`, `win32k.sys`, `win32kbase.sys`, `win32kfull.sys`,
`ci.dll`, and `cng.sys`. It summarizes the strongest longitudinal conclusions;
the linked reports retain the exact binary metadata, evidence, extraction
caveats, and raw counts.

## Executive summary

- **Windows 10's native and GUI attack surface grows mainly in the full feature
  releases through 2004.** Enclaves, registry transactions, hotpatching,
  extended VM calls, Cross-VM objects, modern input/composition stacks, and
  large win32k migrations all appear in those jumps. Beginning with 1909, the
  shared servicing baseline produces much smaller public-interface deltas.
- **Exploit mitigation evolves as a sequence, not a one-release event.** 1703
  adds strict CFG and loader-integrity state; 1709 adds broad enforce/audit
  policy words; 1803/1809 add speculative-execution isolation and the first CET
  state; 1903 scopes CET to user shadow stacks; 2004 adds auditing and
  set-context IP validation; 21H1 adds strict mode and non-CET image blocking.
- **Kernel threat telemetry progressively follows offensive primitives.** The
  TI provider begins in 1703 with executable-memory, APC, and thread-context
  events; later releases add cross-process VM, suspend/resume, driver/device
  loading, and queued user APC coverage. Mitigation-specific ETW then covers CET
  failures and, in 22H2, Redirection Trust policy.
- **Code Integrity becomes a policy platform.** The series moves from signing
  cache and SI-policy parsing through per-app rules, HVCI audit/compliance,
  refreshable and supplemental policies, multi-policy composition, transactional
  deployment, dynamic catalogs, and finally explicit file-origin/AppId claims.
- **Isolation broadens at several layers.** Server silos mature into a
  kernel-wide lifecycle; enclaves gain a complete create/load/call/terminate
  path; KVA/VBS/IOMMU/DMA Guard become increasingly explicit; Cross-VM event and
  mutant objects appear; late servicing improves ALPC cancellation and silo
  state isolation.
- **Small later deltas are not empty.** The 21H1 CET expansion and 22H2
  Redirection Trust/CI provenance work are high-value security changes despite
  unchanged syscall tables.

## Chronological map

| Transition | Highest-signal security result | Surface character |
|---|---|---|
| [1507 → 1511](01-1507-to-1511.md) | Enclave create/initialize/load services; process-notify `Ex2`; image-map and child-process policy | New protected-execution and callback surface |
| [1511 → 1607](02-1511-to-1607.md) | Registry transactions, signing-policy APIs, dynamic-code opt-out, WinSystem PPL, mature silo framework | Broad kernel/CI and GUI feature growth |
| [1607 → 1703](03-1607-to-1703.md) | First TI injection telemetry; strict CFG/loader integrity; native hotpatch load; modern MIT/RIM/GPU queues | Major security visibility and win32k migration |
| [1703 → 1709](04-1703-to-1709.md) | Enclave call/terminate; VM read/write telemetry; ROP/EAF/IAF audit/enforce policies; image-notify `Ex` | Mitigation and telemetry consolidation |
| [1709 → 1803](05-1709-to-1803.md) | KVA shadow/speculation isolation; extended VM calls; DMA/VBS expansion; CI PPL hardening | Major architecture-hardening release |
| [1803 → 1809](06-1803-to-1809.md) | Explicit SSBD/branch-isolation/CET state; TI suspend/resume events; managed hotpatching | Policy catches up with new hardware threats |
| [1809 → 1903](07-1809-to-1903.md) | User-CET/fiber plumbing; TI driver/device/APC events; Cross-VM event; WDAC multi-policy | Full feature growth with strong defensive telemetry |
| [1903 → 1909](08-1903-to-1909.md) | No public-interface delta; staged enablement, CPU-control state, DMA/CI servicing | Shared 18362 enablement baseline |
| [1909 → 2004](09-1909-to-2004.md) | CET audit/IP validation; Cross-VM objects; alternate syscall dispatch; IOMMU/DMA and HVCI expansion | Last large Windows 10 platform jump |
| [2004 → 20H2](10-2004-to-20H2.md) | Clipboard-read audit; dynamic CI catalogs; EH-continuation metadata | Small shared-19041 hardening delta |
| [20H2 → 21H1](11-20H2-to-21H1.md) | CET strict mode, non-CET blocking/audit, dynamic ranges; win32k lifetime refactoring | High-value mitigation work without new syscalls |
| [21H1 → 21H2](12-21H1-to-21H2.md) | Process component filtering; silo time-zone isolation; EC/public-key validation | Focused policy and validation delta |
| [21H2 → 22H2](13-21H2-to-22H2.md) | Redirection Trust enforce/audit path; CI origin claims; ALPC cancellation state | Focused provenance and lifetime hardening |

## Cross-version findings

### Threat Intelligence and defensive visibility

The strongest telemetry progression starts in
[1703](03-1607-to-1703.md): `EtwTiLogAllocExecVm`, `MapExecView`,
`ProtectExecVm`, `QueueApcThread`, and `SetContextThread` cover a recognizable
injection chain. [1709](04-1703-to-1709.md) adds local/remote VM reads and writes
and a separate Security Mitigations provider. [1809](06-1803-to-1809.md) adds
process/thread suspension and VAD-query context. [1903](07-1809-to-1903.md) adds
driver/device-object load/unload and queued user APC insertion.

Later shared-baseline releases shift toward policy-specific events rather than
new general TI primitives. [21H1](11-20H2-to-21H1.md) logs non-CET image blocks,
control-protection return mismatches, and IP-validation failures;
[22H2](13-21H2-to-22H2.md) logs Redirection Trust decisions. Separately,
[20H2](10-2004-to-20H2.md) adds a win32k clipboard-read audit producer. EDR and
anti-cheat implementations should treat this as a timeline of newly observable
behavior, then validate provider GUIDs, ACLs, schemas, enablement conditions, and
loss behavior on each target build.

### Mitigation lineage: CFG, speculation, and CET

The process-mitigation story is cumulative:

1. [1511](01-1507-to-1511.md) adds remote/low-integrity image-map restrictions
   and signature-policy opt-in.
2. [1607](02-1511-to-1607.md) adds controlled dynamic-code opt-out and child
   policy override state.
3. [1703](03-1607-to-1703.md) adds strict/export-suppression CFG and loader
   integrity continuity, including audit state.
4. [1709](04-1703-to-1709.md) reorganizes `_EPROCESS` around explicit
   mitigation words and adds ROP, EAF/EAF+, IAF, child-process, and module-tamper
   policies with audit/enforce pairs.
5. [1803](05-1709-to-1803.md) adds KVA shadow, branch-prediction controls,
   cache/security domains, and broader VBS/DMA isolation.
6. [1809](06-1803-to-1809.md) exposes process bits for branch restriction,
   SSBD, page-combine disablement, domain isolation, and early CET shadow stacks.
7. [1903](07-1809-to-1903.md) clarifies user-shadow-stack scope and adds fiber
   lifecycle support.
8. [2004](09-1909-to-2004.md) adds CET audit/logged state and set-context IP
   validation.
9. [21H1](11-20H2-to-21H1.md) adds strict mode, non-CET/EH-continuation
   compatibility policy, dynamic API restrictions, relaxed validation mode, and
   dynamic enforced-address ranges.

This lineage matters when interpreting a late symbol: a 21H1 CET routine is an
extension of state introduced across 1809–2004, not evidence that CET appeared
from scratch in 21H1.

### Code Integrity, WDAC, and provenance

CI moves from individual validation decisions toward lifecycle-managed policy:

- [1607](02-1511-to-1607.md) exposes signing/security-policy operations and adds
  WHQL, weak-crypto, HVCI, and SI-policy parsing.
- [1703](03-1607-to-1703.md) adds per-app/Smartlocker policy and trusted-origin
  claims; [1709](04-1703-to-1709.md) adds refreshable, supplemental, and
  runtime-UMCI policy.
- [1803](05-1709-to-1803.md) adds explicit PPL/interpreter hardening and dynamic
  trust claims; [1809](06-1803-to-1809.md) expands HVCI relocation and
  supplemental-policy handling.
- [1903](07-1809-to-1903.md) adds multi-policy composition, authorization
  callbacks, signed policy tokens, and synthetic-EA verdict caching.
- [2004](09-1909-to-2004.md) adds HVCI compliance diagnostics and transactional
  policy deployment; [20H2](10-2004-to-20H2.md) adds dynamic catalogs and
  publisher-name handling.
- [22H2](13-21H2-to-22H2.md) adds file-object origin claims, AppId/Smartlocker
  hashing and EA validation, signed-policy requirements, and more specific
  certificate/timestamp revocation helpers.

For EDR and BYOVD research, the recurring high-value questions are policy
precedence, audit-versus-enforce drift, cache/EA binding to file identity,
refresh/rollback atomicity, and signer or timestamp boundary behavior.

### Attack-surface migration

Raw counts alone can mislead. Feature releases often replace one interface family
with another: 1607 consolidates DirectComposition operations behind a batch
parser; 1703 removes legacy DirectDraw/video-port services while adding modern
input and GPU queues; 1803–2004 add stateful flip, VAIL, activation, MIT/minuser,
Moderncore, DXG, and Cross-VM protocols. A negative net syscall count can still
hide a more complex parser, and a positive count can include lower-risk
functional calls.

The later shared-baseline releases have stable service tables, so vulnerability
research should pivot from “new syscall” discovery to patch-diffing existing
handlers: win32k reference/lifetime changes in 21H1, component-filter enforcement
in 21H2, and ALPC cancellation plus path-redirection handling in 22H2 are good
examples.

## Highest-priority research leads

1. Reconstruct the full CET policy state machine from 1809 through 21H1,
   especially audit/enforce divergence, compatibility downgrade, dynamic address
   ranges, and context/fiber transitions.
2. Validate the TI and mitigation-event timeline at runtime, including protected,
   siloed, enclave, kernel-originated, and high-event-rate cases.
3. Audit CI policy refresh/deployment, multi-policy precedence, dynamic catalogs,
   synthetic EAs, and origin claims as one evolving trust-cache system.
4. Fuzz the stateful GUI transitions introduced in 1607–2004, prioritizing batch
   parsers, input routing/injection, duplicated GPU handles, and asynchronous
   object teardown.
5. Trace isolation-boundary authorization and teardown across server silos,
   enclaves, IOMMU/DMA Guard, Cross-VM objects, and ALPC.
6. Map 22H2 Redirection Trust end to end and stress canonicalization, reparse,
   mount-point, Unicode/case, hash-cache, and time-of-check/time-of-use behavior.

## Evidence and limitations

- “Added” means present in the newer selected PE/PDB-derived database and absent
  from the older one. It does not by itself prove default enablement or
  unprivileged reachability.
- Syscalls are treated as confirmed when the kernel table, user stub/export, and
  surrounding implementation agree. The reports explicitly retain known
  extraction discontinuities and non-`Nt*` false positives.
- Anonymous type IDs are ignored as rebuild noise; named parent/member changes
  and resolved anonymous bitfields carry the mitigation evidence.
- 1909 and the 20H2–22H2 BASE entries use serviced binaries from a shared build
  line. Their reports record actual per-binary metadata and avoid attributing all
  marketing-release behavior to a small binary diff.
- Symbol names support subsystem and intent hypotheses, not proof of semantics.
  The reports mark inferences and provide disassembly, runtime ETW, or fuzzing
  steps for confirmation.
