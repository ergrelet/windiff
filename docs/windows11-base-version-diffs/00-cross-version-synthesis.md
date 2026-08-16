# Windows 11 core security evolution: 21H2 → 26H1 (amd64)

## Scope

This synthesis connects the adjacent Windows 11 amd64 reports in this directory.
Every comparison uses the same seven binaries:
`ntoskrnl.exe`, `ntdll.dll`, `win32k.sys`, `win32kbase.sys`, `win32kfull.sys`,
`ci.dll`, and `cng.sys`.

All five transitions now have PE/PDB-derived data. The first four use the
configured BASE records. Because the configured 26H1 BASE/ISO tuple has no
matching Winbindex artifacts for any of the seven binaries, the final transition
uses the first chronological 26H1 cumulative update: KB5077179, released
February 10, 2026 as OS build 28000.1575. It is therefore a serviced-25H2 →
26H1-update comparison, not BASE → BASE and not an isolated KB patch diff.

## Executive summary

- **Windows 11 alternates full platform jumps with shared-baseline enablement or
  servicing transitions.** 21H2 → 22H2 moves from 22000 to 22621, and 23H2 →
  24H2 moves from 22621 to 26100; 25H2 → 26H1 then moves to 28000. All three
  jumps carry substantial native, GUI, isolation, or CI change. The intervening
  23H2 and 25H2 snapshots remain on 22621 and 26100 respectively and must not be
  interpreted as clean marketing-release deltas.
- **Scheduling and device isolation deepen across the series.** 22H2 adds CPU
  partition objects and a restrict-core-sharing process mitigation. 23H2 expands
  secure PASID/SVM/ATS and speculative-execution controls. 24H2 adds a
  VBS-connected driver-proxy subsystem and trusted-app/object trust state. 25H2
  continues first-stage IOMMU and inline-storage-crypto work. 26H1 expands the
  proxy/interposition model and adds structured IOMMU fault attribution.
- **Threat telemetry broadens at the 24H2 platform jump.** New TI producers cover
  impersonation, reversion, and selected syscall usage. In 25H2, mitigation ETW
  adds FSCTL-prohibition logging, while the prior reversion producer disappears
  as a named symbol and needs runtime validation for possible consolidation.
  The selected 26H1 files add no named TI producer, so telemetry growth should
  not be assumed from the scale of the platform jump.
- **Code Integrity remains a major moving boundary in every release.** Smart App
  Control and dynamic-code trust dominate 22H2; 23H2 adds SAC/SmartScreen and
  AppId servicing; 24H2 restructures SI policy, HVCI, revocation, and VBS policy;
  25H2 adds highly specific PPL/CI race and synchronization fixes, new signing
  authorities, CA-remapping policy, and Trusted Launch validation. 26H1 adds
  hotpatch target rules, Trusted Launch host policy, and ML-DSA verification.
- **The public attack surface migrates rather than simply grows.** New native
  services concentrate in 22H2 and 24H2. Win32k changes in every supported
  transition, often replacing older interfaces with capability-aware,
  CoreMessaging, input, composition, or window-policy variants.
- **26H1 adds a different kind of platform security surface.** Its one confirmed
  native addition manages work-on-behalf QoS/priority tickets; callback-unload
  validation, driver-isolation/verifier wrappers, IOMMU fault callbacks,
  hotpatch-aware CI policy, and ML-KEM/ML-DSA support carry more security signal
  than the syscall count. No new named `EtwTi*` producer or process-mitigation
  bit appears in the selected files.

## Chronological map

| Transition | Highest-signal security result | Surface character |
|---|---|---|
| [21H2 → 22H2](01-21H2-to-22H2.md) | CPU-partition services and restrict-core-sharing; syscall providers; KASAN; Smart App Control; GPU doorbells/fences | Full 22000 → 22621 platform expansion |
| [22H2 → 23H2](02-22H2-to-23H2.md) | FSCTL process restriction; SRSO/branch-confusion controls; secure PASID/ATS; shell handwriting and hot-key services | Serviced 22621 enablement delta |
| [23H2 → 24H2](03-23H2-to-24H2.md) | TI impersonation/syscall telemetry; driver proxy/VBS; trusted-app/object trust state; three native and 31 GUI services; XFG deprecation | Full 22621 → 26100 platform expansion |
| [24H2 → 25H2](04-24H2-to-25H2.md) | CI/PPL servicing fixes; FSCTL mitigation telemetry; signing-root migration; process Win32 capabilities; 17 new GUI services | Mixed 26100 servicing/enablement delta |
| [25H2 → 26H1 KB5077179](05-25H2-to-26H1.md) | WOB ticket service; callback-unload validation; driver isolation/IOMMU faults; hotpatch CI; ML-KEM/ML-DSA; 11 GUI services | Full 26100 → 28000 platform jump using first update because BASE is unavailable |

## Cross-version findings

### Native and GUI attack-surface evolution

[22H2](01-21H2-to-22H2.md) adds five corroborated native services:
`NtCopyFileChunk` and four CPU-partition operations. The CPU-partition object is
not merely scheduler plumbing; it creates a new securable namespace, information
classes, process-assignment state, and concurrency rules. The same transition
adds eleven win32k services centered on GPU doorbells/native fences, graphics
process visibility, clipboard metadata, and window registrations.

The shared-22621 [23H2](02-22H2-to-23H2.md) snapshot has no confirmed new native
service. Its four real win32k additions form a shell handwriting-delegation
lifecycle and a shell hot-key path. The kernel extractor's non-`Nt*` substitutions
are retained as raw data but correctly discounted.

[24H2](03-23H2-to-24H2.md) adds three corroborated native services for batched or
extended thread alerting and event signaling, plus 31 win32k calls. The GUI
surface spans capability assignment, synthetic-input v2, CoreMessaging IOCP,
display-mux state, composition synchronization, DWM/window suppression, and an
explicit UMPD-sandbox transition. These calls warrant capability/session/desktop
matrices rather than name-only fuzzing.

[25H2](04-24H2-to-25H2.md) leaves the native table unchanged but adds 17 and
removes nine GUI entries. New surface includes DWM LPC messaging, composition
buffer collections, intercept-window state, window-station creation, process
Win32 capabilities, touchpad injection, and CoreMessaging endpoint registration.
The recurring pattern is interface migration: removals often have plausible
capability-aware or stateful successors and should not be counted as eliminated
functionality without call-graph confirmation.

[26H1](05-25H2-to-26H1.md) adds one corroborated native service,
`NtManageWobTicket`, for work-on-behalf priority/QoS state. Its kernel-only
`NtCompleteConnectPort` row is discounted because the older API has no new ntdll
stub. Eleven new win32k entries add remote texture lifecycles, user-mode GPU
hardware queues/ring resizing, keyboard metadata, DC-transfer state, and job UI
policy. Ten removed analog-token and desktop visual/input-sink entries continue
the series-wide pattern of GUI contract migration.

### Process, scheduler, VBS, and device isolation

The [22H2](01-21H2-to-22H2.md) release ties
`PS_MITIGATION_OPTION_RESTRICT_CORE_SHARING` and an `_EPROCESS` runtime bit to
CPU-partition and core-isolation machinery. This is a coherent SMT/co-scheduling
isolation control, relevant to side-channel reduction and hostile co-residency.
The same release adds per-process syscall-provider state with VSL publication,
creating an important kernel/VBS dispatch control plane whose registration and
telemetry semantics need reversing.

[23H2](02-22H2-to-23H2.md) expands PASID-aware IOMMU domains, SVM state, secure
ATS configuration, device power callbacks, and Hyper-V integration. It also adds
per-CPU state for RSB flushing, kernel CET, branch-confusion handling, and SRSO.
These names establish new controls and selection logic; actual activation remains
CPU, microcode, policy, and feature-state dependent.

[24H2](03-23H2-to-24H2.md) adds a driver-proxy contract connected to secure-kernel
endpoint-wrapper services, explicit trusted-app process state, and object-type
mandatory-label/trust-constraint masks. Together they suggest a shift toward
moving selected driver interactions behind VBS and applying more specific trust
constraints to object access. The same platform adds RCU/SRCU lifetime
infrastructure, which is valuable but also makes migration sites prime UAF
patch-diff targets.

[25H2](04-24H2-to-25H2.md) adds an explicit first-stage IOMMU paging mode and a
larger inline-storage-encryption contract. These are privileged driver/device
boundaries rather than ordinary-user syscalls; descriptor negotiation, key
namespace binding, invalidation, power transition, reset, and fallback handling
are the priority review areas.

[26H1](05-25H2-to-26H1.md) makes the 24H2 driver-proxy direction much more
concrete: hundreds of `DifNt*`/`DifZw*`, `pXdv*`, and `Verifier*` wrappers sit
beside proxy endpoint version/hot-swap exports and VSL image state. This is best
treated as a mediated driver execution/interposition subsystem, not hundreds of
new callable services. IOMMU fault callbacks and a structured output-mapping
source ID add attribution and recovery state, while callback-registration
tracking adds an unload-time lifetime boundary for kernel extensions.

### Mitigations and telemetry

The process-policy sequence is smaller than in Windows 10 but still coherent:

1. [22H2](01-21H2-to-22H2.md) adds restrict-core-sharing process policy and
   runtime state.
2. [23H2](02-22H2-to-23H2.md) adds enforce/audit bits for prohibiting selected
   FSCTL system calls, plus a classifier and feature gate.
3. [24H2](03-23H2-to-24H2.md) explicitly deprecates XFG process-policy bits and
   adds a different SCP/CFG-page and invalid-target-validation path; it also adds
   TI producers for impersonation/reversion and selected syscall use.
4. [25H2](04-24H2-to-25H2.md) adds `EtwTimLogProhibitFsctlSystemCalls`, which is
   visibility for the existing FSCTL policy rather than proof of a newly created
   mitigation. It also exposes an `Mm_OriginalPteRace` servicing gate and no new
   process-mitigation bit.
5. [26H1](05-25H2-to-26H1.md) adds no named process-policy bit or TI producer.
   Its hardening is lower-level: callback-registration validation, an ALPC
   communication-reference bit, PFN buddy tags, and a redesigned pool manager.
   The 25H2 original-PTE servicing gate disappears, consistent with integration
   into the newer baseline rather than proof of rollback.

KASAN/ASAN support in 22H2 and KCSAN/volatile-ASan support in 24H2 are testing and
verification infrastructure. They materially improve bug discovery when enabled
or instrumented but should not be represented as transparent protection for all
production code.

### Code Integrity and trust policy

CI evolves continuously across both platform and servicing jumps:

- [22H2](01-21H2-to-22H2.md) adds the Smart App Control/Night's Watch policy
  cluster, Defender-driven enforcement transitions, verified-and-reputable
  decisions, dynamic-code trust claims/logging, policy refresh, and additional
  CI providers.
- [23H2](02-22H2-to-23H2.md) adds SmartScreen/SAC decision instrumentation,
  AppId-tagging enforcement, UMCI revalidation controls, signature-expiry events,
  and CI network-activity providers.
- [24H2](03-23H2-to-24H2.md) restructures SI policy into explicit state,
  serialization, signing, UMCI, and VBS modules; expands HVCI image/hotpatch
  handling; and synchronizes revocation state with SKCI.
- [25H2](04-24H2-to-25H2.md) adds named servicing gates for a WinTcb PPL TOCTOU,
  CI policy deadlock, HVCI/control-area synchronization, VBS policy-count failure,
  and cached enterprise signing levels. It also introduces 2024 PCA identities,
  CA-remapping policy, Trusted Launch catalog validation, and policy-options v2.
- [26H1](05-25H2-to-26H1.md) adds hotpatch settings, target-existence and
  object-authorization rules; Trusted Launch host SID/dev-test-signing paths;
  EndpointSec and IVAS policy IDs; and ML-DSA verification. CNG's simultaneous
  public KEM exports and ML-KEM/ML-DSA implementation make post-quantum support a
  cross-component platform theme rather than CI-only symbol churn.

For defenders and vulnerability researchers, the highest-value cross-version
questions are policy-state authenticity, audit/enforce divergence, refresh and
rollback atomicity, file/cache identity binding, signer/root migration, and PPL
validation-to-use races. The 26H1 data adds hotpatch sequence/target binding,
Trusted Launch host identity, and classical/post-quantum algorithm policy to
that list.

## Highest-priority research leads

1. Recover CPU-partition access masks and information classes, then measure the
   restrict-core-sharing mitigation's real scheduler behavior under SMT load.
2. Reverse syscall-provider registration, per-process inheritance, VSL
   publication, rundown, and visibility to EDR/anti-cheat telemetry.
3. Build a cross-version PASID/IOMMU/ATS harness covering creation failure,
   surprise removal, power cycling, fault storms, and secure-kernel handoff.
4. Capture the 24H2 TI impersonation/reversion/syscall events and the 25H2 FSCTL
   mitigation event; verify provider ACLs, schemas, filters, protected/silo
   coverage, and whether reversion remains visible after its named producer is
   removed.
5. Fuzz the capability-aware win32k migrations across AppContainer, integrity,
   session, desktop, broker, input-injection, and object-teardown boundaries.
6. Reconstruct the 25H2 WinTcb PPL TOCTOU and `Mm_OriginalPteRace` fixes from
   guarded old/new control flow without assigning unsupported CVE identities.
7. Exercise CI policy refresh, multi-policy parsing, HVCI control-area ownership,
   2024 signing-root migration, CA remapping, and Trusted Launch cache behavior.
8. Reverse 26H1 WOB ticket classes and callback-registration tracking; test
   cross-process QoS changes, callback invocation/unload races, and bypasses in
   callback families not covered by the registration tree.
9. Map DIF/XDV mediation and IOMMU fault callbacks across VBS/HVCI states, proxy
   endpoint hot-swap, device reset/removal, and malformed driver arguments.
10. Validate CI hotpatch target/sequence policy and exercise ML-KEM/ML-DSA
    import, verification, decapsulation-failure, zeroization, and policy paths.

## Evidence and limitations

- “Added” means present in the selected newer PE/PDB-derived database and absent
  from the selected older one. It does not prove default enablement,
  low-privilege reachability, or exact first shipment in a marketing release.
- Native syscalls are considered confirmed only when kernel and ntdll evidence
  agree. Non-`Nt*` and unpaired rows are retained in raw counts but classified as
  extraction artifacts in the reports.
- Anonymous type IDs are discounted. Named parent/member changes and resolved
  bitfields carry mitigation and structure claims.
- The 23H2 and 25H2 BASE entries use serviced binaries from the shared 22621 and
  26100 lines; 25H2 also mixes two file revisions. Their reports describe exact
  selected artifacts, not an atomic enablement package.
- The 24H2 `win32k.sys` database lacks reconstructed types. Its apparent removal
  of all old win32k types is an extraction discontinuity, not a real ABI purge.
- Every requested 26H1 BASE binary is unavailable in Winbindex. The final report
  therefore uses KB5077179, the earliest official cumulative update, and labels
  every finding as an accumulated platform comparison. Five of its generated
  binary records have blank embedded version strings, although the selected
  tuple and official package identify build 28000.1575.
