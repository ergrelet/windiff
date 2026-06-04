# WinDiff analysis: ntoskrnl.exe — Windows 11 25H2 → 26H1 (amd64)

## Scope
- **Old:** Windows 11 **25H2 / BASE / amd64**
- **New:** Windows 11 **26H1 / KB5089570 / amd64** (latest tracked 26H1 update)
- **Binaries compared:** `ntoskrnl.exe`
- **Focus:** new syscalls, mitigation surface, attack surface, EDR-relevant
  telemetry, kernel hardening — general security pass.

> Caveat — this is a single-binary diff. Companion binaries (`ntdll.dll`,
> `win32k*.sys`, `ci.dll`, `cng.sys`, `securekernel.exe`) were not pulled in
> this run; some hypotheses below would be confirmed by their deltas (notably
> the user-mode stub story for new syscalls and the CI-policy side of
> `Feature_TrustedLaunch*`). 26H1 is currently a Canary/Insider-class branch,
> so several "features" below are gated by `Feature_*` staging flags and may
> not be on by default in shipping builds.

## Executive summary

- **Two new ntoskrnl syscalls.** `NtCompleteConnectPort` (#162) splits the ALPC
  client-side connect handshake — likely letting the connect side be completed
  asynchronously. `NtManageWobTicket` (#284) introduces a new "Wob" (workload
  bucket) ticket system with QoS / CPU-priority knobs. Both are **net-new attack
  surface reachable from low-privilege user mode**. **[vuln-research][EDR]**
- **`ZwManageHotPatch` becomes a first-class kernel export**, accompanied by
  ~14 new `MiHotPatch*` routines (`MiPrepareToHotPatchImage`,
  `MiCaptureHotPatchInfo`, `MiLockHotPatchUndoPages`,
  `MiLogHotPatchRundownForProcess`, `MiMakeRestOfImageHot`, …). This is a
  significant expansion of Windows' hot-patching pipeline — both a
  defensive win (faster servicing) and a primitive that EDR/anti-cheat must
  account for, since pages get rewritten in-place. **[EDR][anti-cheat][vuln-research]**
- **New telemetry path for critical-process death.** A whole `_PSP_CRITICAL_PROCESS_DEATH_*`
  family + `PspCriticalProcessDeathBugcheckCallback` walks the user stack,
  collects loaded modules, and identifies the blamed thread before bugcheck.
  This is **new visibility into why csrss/lsass/wininit/etc. died** — directly
  useful for EDR root-cause and for forensic dumps. **[EDR]**
- **New layered-boot subsystem** with hash-verified layers (`_BOOT_LAYER_INFORMATION`,
  `_CIM_FILE_LAYER_INFORMATION` carrying `HashAlgId` / `ExpectedHash`,
  `_VHD_LAYER_INFORMATION`, `_VSMB_LAYER_INFORMATION`, `IoMountBootLayer`,
  `IoGetBootLayers`, `ExIsPreBoot`, `_PREBOOT_CONTEXT`). Looks like the kernel
  side of the rumored CIM/composable boot — a **new trust hierarchy at boot**
  that matters for measured-launch and tamper-evidence. **[EDR][vuln-research]**
- **IOMMU fault handling overhauled.** New `_IOMMU_FAULT_SOURCE_ID`,
  `_IOMMU_DMA_DEVICE_FAULT_POLICY` enum (with `Stage1Masked`),
  `_IOMMU_DMA_DEVICE::Native::SourceId` re-typed to `_EXT_IOMMU_OUTPUT_MAPPING*`,
  `HalIommuReportIommuFault` signature widened, plus
  `Feature_IommuStage1FaultSuppresion`. Continuation of Microsoft's kernel-DMA-
  protection work — finer-grained DMA fault policy. **[vuln-research]**
- **Sanitizer / runtime-instrumentation expansion.** `Kasan*` exports are now
  `*NoInline` (and several were demoted from exports), and a fresh batch of
  atomic `__asan_load*_atomic` / `__asan_store*_atomic` / `__asan_wrap_wcs*`
  appears, plus a new kernel ConcurrencySanitizer entry point (`KcsaniReport`,
  `CsanReadPointerNoCheck` replacing the per-width `CsanRead{8,16,64}NoCheck`).
  These are checked-build / sanitized-build instrumentation — they tell you
  Microsoft is finding more bugs in this area. **[vuln-research]**
- **EDR convenience exports.** `MmIsKernelAddress`, `MmIsUserAddress`,
  `PsEnumProcessThreads`, `ObCloseHandleWithResult`, `PsIsThreadAttachedToSpecificSilo`,
  `PsSwapImpersonationToken`, `PsForceCrashForInvalidAccess`. Several of these
  were previously private — they substantially expand the **stable kernel API
  surface usable by AV/EDR** (with the usual caveat that exports are a
  candidate driver primitive too). **[EDR][vuln-research]**

## New syscalls

`syscalls: +2 new, 198 renumbered (475 → 477)` — the renumbering is build noise;
two genuine additions:

- **`NtCompleteConnectPort`** [id 162] — **Ps/Alpc**. Name mirrors the existing
  server-side connect-completion idiom (`NtAcceptConnectPort` /
  `NtCompleteConnectPort` already exist on the *server* path); the slot being
  newly populated suggests Microsoft is exposing an asynchronous /
  split-phase **client-side** completion of ALPC connects. ALPC is a
  perennially fruitful research target (Process-Explorer-style brokering,
  RPC), so any new state machine in connect handling is interesting. Pair-stub
  in `ntdll.dll` is highly likely (not verified — re-run the skill across
  `ntdll.dll` to confirm). **[vuln-research][EDR]**
  *Confirm by:* reverse `nt!NtCompleteConnectPort` and diff against the
  pre-existing handler of the same name (likely renamed/repurposed) plus
  `AlpcpProcessConnectionRequest`.

- **`NtManageWobTicket`** [id 284] — **Ps/scheduler/QoS**. New helpers
  `PspQueryWobTicketPriority` and `PspQueryWobTicketQos` plus a fresh
  `_WOB_INFORMATION_CLASS` enum (`WobInfoClassQueryQos`,
  `WobInfoClassQueryCpuPriority`) and a new build object `pswob.obj`. "Wob"
  appears to be a workload-bucket / "work-on-behalf" ticket abstraction —
  conceptually similar to existing `_PS_PROCESS_WAKE_INFORMATION` / WNF wake
  channels, but plumbed as an explicit syscall. The set of information classes
  is currently small (Qos, CpuPriority) but the syscall has a generic
  "Manage" verb, so set/clear semantics are coming. New attack surface
  reachable from user mode that touches per-process scheduling state and
  potentially job-attach paths.
  **[vuln-research][anti-cheat]** (anti-cheat care: tickets that influence
  scheduling priority/QoS could be abused by cheats to win timing races).
  *Confirm by:* reverse `NtManageWobTicket`, the two `PspQueryWobTicket*`
  helpers, and inspect the new `_KTHREAD::WpsFeedback` /
  `_KTHREAD_WPS_FEEDBACK` plumbing.

## New / changed security mitigations

This release is **light on classic process-mitigation flag additions** in
`ntoskrnl.exe` — there are no new bits in `_EPROCESS::MitigationFlags` /
`MitigationFlagsValues2` (i.e. `resolved_member_changes` does not surface
mitigation bits this cycle). The mitigation-adjacent changes that *did* land
are:

- **`_KPROCESS::SecureState`** is now a typed `_KPROCESS_SECURE_STATE` union
  (was an unnamed struct). Holds a 2-bit `EntireFlags` plus full-field access.
  This is the secure-kernel (VTL1) tracking state of the process — naming it
  is an internal cleanup, but `EntireFlags : 2` is the actual carrying field.
  **[vuln-research]** for anyone reversing VBS/secure-process flow.
- **`_KALPC_MESSAGE::CommunicationInfoReference : 1`** — new ALPC message
  flag bit. Pair this with the new `NtCompleteConnectPort`: there is plausibly
  a new ALPC message field referencing pre-staged communication info that
  the new connect-completion path consumes. **[vuln-research]** for ALPC
  hardening / fuzzing campaigns.
- **`_PS_ATTRIBUTE_NUM::PsAttributeSmeVectorLength = 0x21`** — new process-
  attribute slot for the **ARM64 SME (Scalable Matrix Extension) vector
  length**. Hardware-feature plumbing. Not a security mitigation per se,
  but it widens the set of attributes a creator can pass to
  `NtCreateUserProcess`, so it's in fuzzing scope. **[vuln-research]**
- **`KeCanonicalizeXStateUserCetPl3Ssp` + `KiUserCetPl3SspCanonicalizeMask`**
  — new helper to canonicalize the user-mode CET shadow-stack pointer (PL3
  SSP) when restoring an XState. Plus `Feature_CET_User_AMD_Canonicalize_Perf_Fix`
  (a feature-gate). This looks like a correctness / perf fix for CET restore
  on AMD silicon, but if the canonicalization is wrong, **CET shadow-stack
  bypass primitives** become possible. **[vuln-research]**
- **`PsForceCrashForInvalidAccess`** — newly exported. The name implies a
  kernel-callable that bugchecks (or kills the process) on a flagged invalid
  access, used by other components as a "fail-stop" primitive. Mitigation-
  flavoured rather than a new mitigation. **[vuln-research]**
- **`IoCheckRedirectionTrustLevel2`** — versioned successor to
  `IoCheckRedirectionTrustLevel`. The original was added to defend against
  symlink/redirection attacks against drivers that follow handles. A "v2"
  almost always means the trust-level enum / context grew. Drivers that
  used the v1 should expect to update. **[vuln-research][EDR]**
  *Confirm by:* reverse and compare the trust-level computation/return value
  to the v1 routine.

(Several other security-relevant fields moved around — see *Notable structure
changes* below — but no new explicit mitigation bit was added to the
documented process flag bitfields in this binary.)

## New security-relevant features & components (beyond mitigations)

### Detection & telemetry

- **`PspCriticalProcessDeath*`** — a new diagnostic subsystem fired when a
  critical process is about to bugcheck the box.
  - `PspCriticalProcessDeathBugcheckCallback` — bugcheck-time callback
  - `PspCriticalProcessDeathInfoCollect` (+ `…ScheduleApc`, `…CollectApc`)
    — APCs into the dying process to walk its user-mode stack
  - `PspCriticalProcessDeathBlamedThreadTryGet`,
    `PspCriticalProcessDeathUserModulesCollect`,
    `PspCriticalProcessDeathIsFrameInModule`
  - New types `_PSP_CRITICAL_PROCESS_DEATH_INFO_HEADER`,
    `_PSP_CRITICAL_PROCESS_DEATH_INFO_1`,
    `_PSP_CRITICAL_PROCESS_DEATH_USER_MODULES`,
    `_PSP_CRITICAL_PROCESS_DEATH_ERROR_CODE` (8 enum values, including
    `…WoWProcess`, `…NoTib`, `…ApcRemoved`).
  - Output is shaped as a versioned `_INFO_1` structure (Header + Modules +
    DumpData + BlamedThread + BlamedThreadTib + ErrorCode + UserStack
    counters), strongly suggesting it is **persisted into the bugcheck
    minidump or a WER record**.
  - **Why it matters:** today, when csrss/lsass/wininit dies, the box bugchecks
    and you get a kernel dump but very little structured info about what the
    user-mode side was doing at the time. This adds first-class user-mode
    state capture to the bugcheck path. **[EDR]** for incident-response and
    detection of credential-attack-induced crashes against lsass.
  - *Confirm by:* reverse `PspCriticalProcessDeathBugcheckCallback`, find what
    consumes the buffer (Wer / minidump writer), and check whether
    `_PSP_CRITICAL_PROCESS_DEATH_INFO_1` is exposed via an ETW event.
- **`EtwWorkQueueProvRegHandle` / `EtwTraceWorkQueueHealthMetrics`** — new
  ETW provider for **executive work-queue health metrics**, including
  bucket upper-bounds (`EtwTraceWorkQueueHealthMetricsBucketUpperBounds`).
  New type `_EX_WORK_QUEUE_HEALTH_METRICS`. Useful for performance
  observability; for EDRs, periodic queue-health events can be a noisy but
  free behavioral signal (queue starvation often correlates with kernel
  contention from rootkits / pathological drivers). **[EDR]**
- **`PoRegisterSstNotificationHandler` / `PoUnregisterSstNotificationHandler`**
  — new power-manager registration API; SST is plausibly "System State
  Transition / Tracking" (paired with new `sstnotif.obj` build artifact).
  Drivers can subscribe to system-state changes. **[EDR]** for
  modern-standby / S0-low-power state-aware detection.

### Notification callbacks / kernel API surface

- **`PsEnumProcessThreads`** (new export) — long-requested clean way to walk
  threads of a target process from a kernel driver. Previously code did this
  via `PsLookupThreadByThreadId` + `_EPROCESS::ThreadListHead` walks. Real
  EDR/anti-cheat win. **[EDR][anti-cheat]**
- **`PsIsThreadAttachedToSpecificSilo`** (new export) — enables silo-aware
  detection (a thread temporarily attached into a server-silo / container
  context). **[EDR]**
- **`PsSwapImpersonationToken`** (new export) — atomic impersonation-token
  swap. Useful primitive for token-guarding code in EDR — but also a *very
  attractive* primitive for token-stealing if abused. **[EDR][vuln-research]**
- **`ObCloseHandleWithResult`** (new export) + new types `_OBJECT_CLOSE_RESULT`,
  `_OBJECT_CLOSE_TYPE` (currently knows only `ObjectCloseFile`) — close a
  handle and learn what kind of object went away. Could feed file-close
  telemetry without an extra `NtQueryObject`. **[EDR]**
- **`MmIsKernelAddress` / `MmIsUserAddress`** (new exports) — promotes two
  trivial-but-ubiquitous predicates to the stable export surface. **[EDR]**
- **`MmGetModuleRoutineAddress`** (new export) — by-name lookup of a routine
  in a loaded module. Promising for AV/EDR resolving optional kernel APIs
  without `MmGetSystemRoutineAddress` quirks. **[EDR][vuln-research]**
- **`KeRcuReadLockAtDpcLevel`** + **`KeTryToAcquireInStackQueuedSpinLockAtDpcLevel`**
  — DPC-level RCU read-side and try-acquire variants. Indicates Microsoft
  is hardening synchronization at higher IRQLs in core paths. **[vuln-research]**
- **`RtlPcToFileImageInfo`** (new export) — given a PC, return image info.
  Stack-unwind / blame-the-module helper, paralleling the
  `PspCriticalProcessDeathIsFrameInModule` pattern. **[EDR]**

### Code integrity / signing / boot trust

- **Layered boot subsystem.** New types: `_BOOT_LAYER_INFORMATION`,
  `_BACKING_LAYER_INFORMATION`, `_RAMDISK_LAYER_INFORMATION`,
  `_VHD_LAYER_INFORMATION`, `_CIM_FILE_LAYER_INFORMATION`,
  `_VSMB_LAYER_INFORMATION`. New exports `IoMountBootLayer`, `IoGetBootLayers`,
  `IoGetBootLayers`. New types `_BOOT_OSL_RAMDISK_ENTRY_V2`,
  `_BOOT_OSL_RAMDISK_INFO_V2`. Pre-boot phase is now explicit:
  `_PREBOOT_CONTEXT`, `ExIsPreBoot`, `HalpIsPrebootMode`,
  `ExInitializeExternalBootSupport`, `ExInitializeBootStructures`.
  - The killer field is `_CIM_FILE_LAYER_INFORMATION::ExpectedHash` /
    `HashAlgId` / `ExpectedHashLength` — the kernel is **verifying CIM-file
    boot-layer integrity by hash**.
  - This is the kernel side of a layered (likely composable) Windows install:
    boot from a stack of CIM/VHD/RAMDISK layers, each hash-verified. Very
    relevant to measured-boot / supply-chain integrity stories.
  - **[EDR][vuln-research]** *Confirm by:* find callers of
    `IoMountBootLayer` (probably `winload`/`osloader` boot driver) and
    inspect how `ExpectedHash` is populated and where measurements are sent.
- **`Feature_CodeIntegrity_TrustedLaunchPolicy`,
  `Feature_TrustedLaunchCiClaim`, `Feature_TrustedLaunchHosts`,
  `VslQueryTrustedAppRuntimeInformation`** — all new. "Trusted launch"
  policy and CI-claim infrastructure (likely intended to consume the boot-layer
  measurements above and feed them into `ci.dll` policy decisions). Re-run
  this skill on `ci.dll` to see the policy side.
- **`Feature_SrtmAntiRollback`** — Static Root-of-Trust-for-Measurement
  anti-rollback feature flag. Counters firmware-rollback attacks against
  measurement state.
- **`Feature_PlutonDynamicUpgrade`** — Pluton (security processor) dynamic-
  upgrade flag, plus `PO_MEMORY_IMAGE::Feature_PlutonDynamicUpgrade_Enabled`.
  Pluton firmware can be hot-upgraded.

### Process / object / handle hardening (anti-tamper-adjacent)

- **`_SEP_LOGON_SESSION_REFERENCES` grows by 8 bytes** with a leading
  `_LUID ShadowBuddyLogonId` field at offset 0x18. Reference count and the
  rest of the layout shifted. "Shadow buddy" pairs a logon with a sibling
  logon — could relate to a separate-but-linked SYSTEM-companion logon used
  by isolation features. Worth reversing for token-impersonation behavior.
  **[vuln-research]**
- **`_OBJECT_CLOSE_RESULT` / `_OBJECT_CLOSE_TYPE`** (with `…CloseFile`) —
  see `ObCloseHandleWithResult` above. Suggests file-handle closes get a
  distinguished path; minifilter / file-system filter authors should look.
- **`_PSP_QUOTA_ENTRY` / `_EPROCESS_QUOTA_BLOCK`** are now **named** types
  (previously unnamed). Quota tracking infrastructure was rebuilt around
  cache-aligned 64-byte entries (`Usage` / `Peak` / `Limit` / `Return` /
  `ExpansionLink`). The old free-form `MMADDRESS_LIST` /
  `MMWORKING_SET_EXPANSION_HEAD` were removed. Internal cleanup, but it's
  the kind of refactor that historically introduces TOCTOU bugs in quota
  paths. **[vuln-research]**

### IOMMU / DMA hardening

- **`_IOMMU_FAULT_SOURCE_ID`** (with `_IOMMU_FAULT_SOURCE_ID_TYPE`:
  `HvLogicalId` / `HalPciRid` / `HalIommuStreamId`) and
  **`_IOMMU_DMA_DEVICE_FAULT_POLICY`** (`Default` / `Masked` /
  `Stage1Masked`) are new.
- `_IOMMU_DMA_DEVICE::Native::SourceId` was a bare `ULONGLONG` and
  `PasidDomainId` — now it's a typed pointer
  `_EXT_IOMMU_OUTPUT_MAPPING* SourceId`. The change in
  `HAL_PRIVATE_DISPATCH::HalIommuReportIommuFault` adds a leading
  `_IOMMU_FAULT_SOURCE_ID*` parameter — every fault now carries structured
  source identification rather than an opaque PRId.
- New `Feature_IommuStage1FaultSuppresion` (sic — "Stage1Masked" enum
  value matches it) and `Feature_IommuInterfacePointerReset`. Many new
  `HalpIommu*` functions including `HalpIommuFaultDeferredRoutine`,
  `HalpIommuFaultIgnoreList`, `HalpIommuMatchFaultDevice`. **[vuln-research]**
  for anyone working on KDP / DMA-attack research against Thunderbolt /
  external GPUs / SR-IOV.

### Hot patching pipeline

`ZwManageHotPatch` is now an exported routine, paired with NTaposyscall
filter handlers and a substantial new body of work in MM:

- `MiCaptureHotPatchInfo`, `MiPrepareToHotPatchImage`,
  `MiPrepareImagePagesForHotPatch`, `MiOpenHotPatchFile`,
  `MiLockHotPatchUndoPages`, `MiLogHotPatchRundownForProcess`,
  `MiMakeRestOfImageHot`, `MiCachedPagesMakeHot`, `MiWalkImageMakePageHot`,
  `MxMarkPfnChannelHot`.
- The new boolean predicates `_MI_ACTIVE_PFN::PageTable::NonPagedBuddyTag`
  and `Leaf::NonPagedBuddyTag` (4 bits each) suggest hot-patched pages get
  a tag in the PFN database for tracking.
- **Why it matters:** hot patching means **kernel image bytes are rewritten
  at runtime**. Defenders that snapshot ntoskrnl text and re-hash it will
  see drift even on a clean system. Anti-cheat systems that PatchGuard-imitate
  (compute image hashes) need to consume the new rundown logs. Vuln research:
  the patch-orchestration code is itself attack surface (any TOCTOU in
  `MiPrepareToHotPatchImage` is catastrophic). **[EDR][anti-cheat][vuln-research]**

### Sanitizer / diagnostic instrumentation

- **`Kasan*NoInline` family** — `KasanMarkAddressValidNoInline`,
  `KasanMarkAddressInvalidNoInline`, `KasanMarkAddressRedZoneNoInline`,
  `KasanTrackAddressNoInline`, `KasanPoolAllocateNoInline`,
  `KasanIsEnabled`, plus user-mode-access wrappers (`KasanUmaCopyToUser`,
  `KasanUmaCopyFromUser`, including `Nontemporal` variants).
- **New `__asan_*_atomic` exports** for 1/2/4/8/16-byte atomic loads/stores,
  plus `__asan_handle_no_return`, `__asan_set_shadow_f1..f5`, and
  `__asan_wrap_wcscat`/`wcscpy`/`wcsncpy` wide-string wrappers.
- **`KcsaniReport` / `CsanReadPointerNoCheck`** — Kernel ConcurrencySanitizer.
  The previous per-width `CsanRead8/16/64NoCheck` and `CsanWrite*NoCheck`
  exports were removed in favour of a single pointer-width entry point.
- **Why it matters:** these only fire on instrumented builds, but their
  presence (and shape change) is a tell for **where Microsoft is
  investing in finding bugs**. The wide-string wrappers in particular
  suggest `wcs*` paths are being validated for OOB. **[vuln-research]**
- **`VfSetVerifierEnabled`** (new export) and `VfKeCriticalRegionTraceActiveCount`
  data — Driver Verifier surface tweak.

### New drivers / modules / components

- **`ext-ms-win-kmpdc-l1-1-0.dll`** — new API-set extension for the
  Power-Dependency Coordinator (PDC). The PDC's existing client objects
  (`activatorcli.obj`, `etwtracing.obj`, `misccli.obj`, `portcli.obj`,
  `taskcli.obj`, `watchdog.obj`) are *gone* from the kernel and replaced
  by a `_dload`-based extension. **PDC is being externalized into an
  apiset extension.** This is a structural change — drivers that linked
  against PDC internals will now resolve through the apiset.
- **`ext-ms-win-ntos-stateseparation-l1-1-1.dll`** — bumped from `-1-1-0`.
  State-separation extension surface grew (one of the silo / WCOS
  separation knobs).
- **`ext-ms-win-ntos-ksr-l1-1-6.dll`** — bumped from `-1-1-5`. KSR (kernel
  soft reset / kernel-mode safe restart) interface revved.
- **`Feature_TrustedLaunchHosts`** — companion to TrustedLaunch CI claim.
- **`Feature_AddMemInfoToBootTrace`** — boot-trace gains memory info.
- **`Feature_UnattendedRebootIdleFix`** — quality-of-life feature flag.
- **`Feature_LookasideDepthManager`** + a fresh `_EXP_LOOKASIDE_MGR` with a
  dedicated worker pool — system-wide lookaside depth is now actively
  managed (rebalanced) instead of statically configured. Net-new internal
  surface; relevant if you previously relied on constant lookaside depths.
- **`Feature_ExtendedWpsTables` / `Feature_WpsExtendedFamilyContainment` /
  `Feature_WpsHybridToNoneZoneFix` / `Feature_ReinitWpsMinEfficiency` /
  `Feature_Servicing_WpsContaintmentDefaultDisabled`** + `_KTHREAD_WPS_FEEDBACK`.
  WPS = "Workload Performance State" / hetero-CPU containment. Not a
  security feature per se, but explains the sweeping changes to
  `_KTHREAD` (HGS feedback fields → WpsFeedback pointer).

### VBS / secure kernel

- **`_KPROCESS_SECURE_STATE`** (named) and **`_KPROCESS::SecureState`** are
  now structured types. Secure-process handling slightly more first-class.
- **`VslQueryTrustedAppRuntimeInformation`** — VSL = VTL Switch Library.
  New runtime-info query for trusted apps (consumes secure-kernel state).

## New attack surface (selected routines)

Beyond the syscalls and hot-patch infrastructure already covered:

- **`IoHotSwapDriverProxyEndpoints`** + **`IoGetDriverProxyExtensionVersion`**
  — driver "proxy" endpoint hot-swap. Explicitly ties into the driver-proxy
  framework that was added a few releases ago. Hot-swap implies endpoint
  rewriting at runtime — fertile ground for race-condition research.
  **[vuln-research]**
- **`ExGetFfaInterface` / `ExFreeFfaInterface`** — FFA = ARM Firmware
  Framework for Armv8-A (PSA / TrustZone secure-partition mediation). New
  kernel-callable to obtain the FFA dispatch interface. ARM64-specific
  attack-surface widening; only meaningful on platforms with FFA-capable
  firmware. **[vuln-research]**
- **`HvlGetLpStatsPageByLpIndex` / `HvlGetVpStatsPageByProcessorIndex` /
  `HvlGetTrustedIoStatus` / `HvlGetSyntheticMachineCheckContext`** —
  hypervisor-stat and trusted-IO query helpers. Useful telemetry for hosts
  running under Hyper-V root, but each new hypercall path is a **new
  parsing surface**. **[vuln-research]**
- **`WheaPrmTranslateNormalizedAddressToPhysicalAddressAmd`** — WHEA / PRM
  address translation on AMD. PRM (Platform Runtime Mechanism) executes
  firmware-supplied code in OS context; address translation surface
  recently received CVEs across vendors. Worth reversing. **[vuln-research]**
- **`KeGetProcessPpmPolicy` / `PsComputeProcessPpmPolicy` /
  `PspSetProcessBamPpmPolicy`** + `_EPROCESS::BamPpmPolicy` /
  `PpmPolicyLock` — Background Activity Moderator (BAM) is being unified
  with the Power-Policy-Manager (PPM) decisions per process. Not directly
  a vuln surface, but **per-process policy state visible from a Job
  Object** can leak signal about process behavior — **[anti-cheat]**
  vendors may want to monitor the lock and policy fields.
- **`PspAssignJobCpuPartitionToProcess` / `PspSetJobCpuPartition`** + new
  `_PS_CPU_PARTITION` and `_EPROCESS::JobCpuPartitionObject` — a job
  object can now own a CPU partition that all attached processes inherit.
  Architectural primitive for resource-limited jobs. **[anti-cheat]** care:
  partitions affect execution affinity/scheduling.
- **`PsApplyDeepFreezeOptimizations` / `PsRemoveDeepFreezeOptimizations`**
  — deep-freeze (process fully suspended, pageable) gains optimization
  toggles. Deep-frozen processes are a known evasion technique; whatever
  these "optimizations" do is worth reversing. **[EDR]**
- **`PsGetProcessUILimits` / `PsGetUILimitJobProvider` /
  `PsSystemSetUILimitJobObject`** — new "UI limits" framework attached to
  job objects. Plausibly window-station / UI-restriction enforcement plumbed
  through job providers — an alternative to the JobObject UI restrictions
  bitfield. **[EDR][anti-cheat]**
- **`PsRefreshUserPresencePpmPolicies`** — power policy refreshed when user
  presence changes. Listens for presence as policy input.

## Notable structure changes

- **`_EPROCESS`** — `NumberOfLockedPages` (volatile counter at +0x290) is
  replaced by `MmReserved2` (also volatile ULONGLONG); `Spare0` at +0x6ef
  becomes `VirtualTimersPaused`; the WNF `WakeChannel` union slot was
  removed and the slot now holds new fields:
  `JobCpuPartitionObject` (+0x808), `FreezeWorkLinks` (+0x810),
  `PpmPolicyLock` (+0x820), `BamPpmPolicy` (+0x828),
  `ProcessPowerThrottlingState` (+0x82c), `ProcessQosCallbackListEntry`
  (+0x838). Net effect: the process gains explicit per-process power /
  throttling / QoS state and a CPU partition reference.
- **`_KPROCESS`** — `PpmPolicy` renamed `ProcessPpmPolicy`; padding fields
  named (`Spare0e`, `IdealGlobalNode` typed `volatile USHORT`); `SecureState`
  becomes `_KPROCESS_SECURE_STATE`.
- **`_ETHREAD`** — `WorkloadClass`, `BalanceSetManager` cross-thread flags
  *removed*; new `Prefetching`, `GenerateDumpOnBadHandleAccess`,
  `PeriodicTrimmerThread` flags added. Trimming is now a first-class
  per-thread role (matches new `MiPeriodicTrimWorkingSet`). The pointer
  `EtwSupport` was renamed/repurposed `SparePointer2` — **the legacy
  per-thread ETW support pointer is gone**, plausibly because ETW state
  moved elsewhere. Worth tracking for EDRs that walked it.
- **`_KTHREAD`** — `Alerted[2]` becomes a union with explicit
  `KernelModeAlerted : 1` / `UserModeAlerted : 1` bits; `Spare24` /
  `Spare27` named (`SharedComputeUnitsUsed`, `CpuSetWorkloadClass`); HGS
  feedback fields (`HgsFeedbackStartTime`, `HgsFeedbackCycles`,
  `HgsInvalidFeedbackCount`, `HgsLowerPerfClassFeedbackCount`,
  `HgsHigherPerfClassFeedbackCount`) are **replaced by a single
  `_KTHREAD_WPS_FEEDBACK* WpsFeedback` pointer**. WPS replaces HGS as the
  feedback model. The new `AbReleasePending : 1` and
  `ChargeOnlySchedulingGroupOverridden : 1` flags are scheduler internals.
- **`_KPROCESSOR_STATE`** grows from 0x5c0 → 0x5e0; `ContextFrame` shifts
  from +0xf0 → +0x110. Anything that hard-codes that offset breaks.
- **`_PS_ATTRIBUTE_NUM`** gains `PsAttributeSmeVectorLength = 0x21`.
- **`_HEAP_SUBALLOCATOR_CALLBACKS`** grows by 8 bytes; new
  `AccessState : ULONGLONG` field (+0x30). RTL HP heap callback table
  is now access-aware. Pair with the new `_RTL_HP_PG_CONFIG` (4-bit
  underflow rate, page-align-large-allocs flag).
- **`PO_MEMORY_IMAGE`** (hibernate image header) reclaims 5 of its 23
  reserved bits for: `HardwareSignatureValid`, `RootPartition`,
  `Feature_SrtmAntiRollback_Enabled`, `Feature_PlutonDynamicUpgrade_Enabled`,
  `DevControlInitialized`. **The hibernation image now records platform
  trust state.** **[vuln-research]** for anyone playing with hiberfile
  rollback / cross-boot-state attacks.
- **`HAL_PRIVATE_DISPATCH`** grows by 0x10:
  `HalIommuReportIommuFault` signature widened (now takes
  `_IOMMU_FAULT_SOURCE_ID*`), and two new dispatch slots:
  `HalTimerQueryInvariantPlatformCounter`, `HalLoadSfsUpdate`.
- **`_MMSECTION_FLAGS` / `_MMSECTION_FLAGS2`** — significant churn. Section
  flags reorganized: `SectionOfInterest`, `GlobalOnlyPerSession`,
  `SystemVaAllocated`, `NotBeingUsed` are gone from `FLAGS`;
  `NoCrossPartitionAccess` and `SubsectionCrossPartitionReferenceOverflow`
  moved into `FLAGS`; the partitioning fields (`PartitionId`) shifted out
  of `FLAGS2`, which now holds image-relocation and code-strength fields.
  Anyone reversing section logic across 25H2/26H1 must redo their offsets.

## Removed / deprecated

- **`KasanMarkAddressInvalid` / `KasanMarkAddressRedZone` / `KasanMarkAddressValid` /
  `KasanTrackAddress` / `KasanValidateAddress`** — demoted from exports;
  `…NoInline` variants are the new shape. Out-of-tree drivers that linked
  against these will break.
- **`CsanRead8/16/64NoCheck`, `CsanWrite8/16/64NoCheck`** removed in favour
  of `CsanReadPointerNoCheck` (single pointer-width entry).
- **`VfFailDeviceNode` / `VfFailDriver` / `VfFailSystemBIOS` /
  `VfIsVerificationEnabled`** removed from exports — Driver Verifier
  surface narrowed (replaced by `VfSetVerifierEnabled`).
- **`__m128` / `__m64`** type definitions disappear (they were debug-only
  intrinsics types; no functional impact).
- **`_MMADDRESS_LIST` / `_MMWORKING_SET_EXPANSION_HEAD`** — quota /
  expansion bookkeeping types replaced by `_PSP_QUOTA_ENTRY` /
  `_EPROCESS_QUOTA_BLOCK`.
- **`_MM_GRAPHICS_VAD_FLAGS`** removed; `_MMVAD_SHORT::u::GraphicsVadFlags`
  no longer exists. Graphics VADs lose a dedicated flag word.
- **`_THREAD_WORKLOAD_CLASS`** removed; replaced by the WPS feedback model.
- **`_BOOT_OSL_RAMDISK_ENTRY` / `_BOOT_OSL_RAMDISK_INFO`** removed in favour
  of `…_V2`.
- PDC's per-client `.obj` files (`activatorcli`, `etwtracing`, `misccli`,
  `portcli`, `taskcli`, `watchdog`) gone — moved into
  `ext-ms-win-kmpdc-l1-1-0.dll`.
- `_SEP_LOWBOX_NUMBER_MAPPING`, `_SESSION_LOWBOX_MAP` removed; lowbox
  bookkeeping refactored.
- `_OBJECT_REF_TRACE`, `_STACK_TABLE` — internal trace types gone.

## Research leads

1. **Reverse `nt!NtCompleteConnectPort` and `nt!NtManageWobTicket`** end-to-end,
   then re-run the diff with `ntdll.dll` to grab the matching user-mode stubs
   and any new ALPC-message fields. ALPC + a new connect verb = high-value
   fuzzing target.
2. **Reverse `PspCriticalProcessDeath*`** and identify whether the
   `_PSP_CRITICAL_PROCESS_DEATH_INFO_1` blob is exposed via ETW or only in
   the bugcheck minidump. EDR vendors should hook this if exposed.
3. **Map the hot-patch pipeline** — start at `ZwManageHotPatch` →
   `NtManageHotPatch` → `MiCaptureHotPatchInfo` → `MiPrepareToHotPatchImage`
   → `MiMakeRestOfImageHot`. Look for TOCTOU between
   `MiLockHotPatchUndoPages` and the page rewrite. Cross-check with
   `_MI_ACTIVE_PFN::NonPagedBuddyTag` to see how patched pages are tagged.
4. **Re-run the diff against `ci.dll`** to get the consumer side of
   `Feature_CodeIntegrity_TrustedLaunchPolicy`, `Feature_TrustedLaunchCiClaim`,
   `Feature_TrustedLaunchHosts`, and the boot-layer hash verification.
5. **Re-run against `securekernel.exe`** for the secure-side of
   `_KPROCESS_SECURE_STATE` and `VslQueryTrustedAppRuntimeInformation`.
6. **Reverse `IoCheckRedirectionTrustLevel2`** vs v1 to enumerate the new
   trust levels / context fields.
7. **Reverse `IoMountBootLayer` and friends** — the rumored layered boot is
   a major change to the boot trust model; compare to recent
   `winload`/`osloader` symbols.
8. **Audit the `HalIommuReportIommuFault` signature change** in any
   third-party HAL extensions / IOMMU drivers — out-of-tree binaries
   linking against the old signature will misbehave.

## Appendix: raw counts

| kind     | added | removed | modified | old → new |
|----------|------:|--------:|---------:|----------:|
| exports  | +47   | -17     | —        | 3350 → 3380 |
| symbols  | +5836 | -2176   | —        | 49262 → 52922 |
| modules  | +102  | -56     | —        | 2065 → 2111 |
| syscalls | +2    | 0       | 198 renumbered (not a feature) | 475 → 477 |
| types    | +127  | -19     | ~233 (+6 anon-member changes) | 2056 → 2164 |

Generated from:
- Old DB: `ntoskrnl.exe_11-25H2_BASE_amd64.json.gz`
- New DB: `ntoskrnl.exe_11-26H1_KB5089570_amd64.json.gz`
- Diff JSON: `local/diff_ntoskrnl.json`
