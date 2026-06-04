# Windows Internals: inferring intent from a version diff

This is the reasoning toolkit for turning a raw symbol/type/syscall delta into a
security interpretation. The goal for each finding is to answer: *what subsystem
is this, what does it likely do, and why does it matter for security?*

## Table of contents
1. API name prefixes (routine namespaces)
2. Routine name suffixes and decorations
3. Syscalls: what additions/removals/renumbering mean
4. Security mitigation flags — where they live and how to spot new ones
5. Key structures to watch
6. How to phrase an interpretation (and admit uncertainty)
7. Security-relevant features & components beyond mitigations (EDR / anti-cheat / vuln research)

---

## 1. API name prefixes

Windows kernel/native routines are namespaced by a 2–3 letter prefix naming the
owning component. The prefix tells you the subsystem; the rest hints at the action.

| Prefix | Subsystem | Security relevance of new entries |
|--------|-----------|-----------------------------------|
| `Nt` / `Zw` | System call interface (user↔kernel). `Nt` = the syscall entry; `Zw` = same with kernel-mode previous-mode semantics | New `Nt*` = **new attack surface reachable from user mode**. Highest priority. |
| `Ps` | Process/thread management (Process Structure) | Process/thread creation, tokens, mitigations, protection (PPL). Watch for new mitigation/protection logic. |
| `Ke` | Kernel core (scheduling, sync, interrupts, CPU) | Low-level primitives; new CET/shadow-stack, speculation, or APC changes appear here. |
| `Mm` | Memory Manager | VAD, paging, sections, pool. New memory-safety/isolation features (e.g. kernel CFG, pool hardening). |
| `Ob` | Object Manager | Handle/object security, callbacks. New object types or handle-hardening. |
| `Se` | Security Reference Monitor | Tokens, privileges, access checks, ACLs, AppContainer. New `Se*` often = **authz/sandbox changes**. |
| `Cm` | Configuration Manager (registry) | Registry security, virtualization, callbacks. |
| `Cc` / `Fs` / `Io` / `Iop` | Cache / filesystem / I/O manager | Driver-facing surface, IRP handling, filter callbacks. |
| `Alpc` / `Lpc` | Advanced Local Procedure Call (IPC) | New ALPC surface is a classic LPE target. |
| `Etw` / `Wmi` | Event Tracing for Windows | New providers/events usually = **new telemetry/defensive instrumentation** (EDR-relevant). |
| `Ci` | Code Integrity (in `ci.dll`) | Driver/DSE signing, WDAC/HVCI policy. New `Ci*` = code-integrity policy/enforcement changes. |
| `Bcrypt`/`Ncrypt`/`Crypt` | CNG cryptography (`cng.sys`, `bcrypt.dll`) | New algorithms/providers, FIPS, key isolation. |
| `Vsl` / `Vbs` / `Skci` / `Securekernel` | Virtualization-Based Security / secure kernel | VBS, HVCI, Credential Guard, secure-kernel surface. High security value. |
| `Rtl` | Runtime Library (shared helpers) | Often supporting code for a feature whose policy lives elsewhere; trace callers. |
| `Exp`/`Psp`/`Mi`/`Obp`/`Sep`/`Cmp`/`Iop` | The `p`/`i` variants are the **internal** (private) implementations | Visible only as debug symbols, not exports; reveal the real logic behind a public stub. |
| `Etw`-style `Win32k`, `gre`, `Nt User`/`NtGdi` | win32k GUI/GDI syscalls | win32k is a huge LPE surface; new `NtUser*`/`NtGdi*` syscalls matter and tie into win32k lockdown. |

A name with no familiar prefix, or a brand-new prefix, can itself be the
signal — a new feature area. Note it and hypothesize from the rest of the name.

## 2. Suffixes and decorations

- `Ex` — extended version of an existing routine (new params/flags). Diff against
  the base routine to see what capability was added.
- `Worker`, `Callback`, `Notify`, `Routine` — registration/callback surface;
  new ones may be EDR/driver notification hooks.
- `Internal`, `Stub`, `Thunk` — wrappers; the interesting logic is elsewhere.
- Trailing digits / `2` — a v2 of an interface, usually because the struct or
  semantics changed; compare the associated types.
- `Mitigation`, `Cet`, `Cfg`, `Xfg`, `Shadow`, `Guard`, `Acg`, `Cig` in a name
  almost always indicate an exploit-mitigation feature (see §4).

## 3. Syscalls

`scripts/windiff_diff.py` reports syscalls as added / removed / renumbered.

- **Added syscalls** are the single highest-value finding: brand-new
  kernel-reachable surface. Interpret each from its `Nt`/`NtUser`/`NtGdi` name.
  Cross-reference whether a matching `Nt*` export and internal `Nt*`/`*p*`
  implementation also appeared.
- **Renumbered** (same name, different id) is normal across builds — the syscall
  table is regenerated. It matters only if you're hardcoding SSNs (e.g. for direct
  syscalls / EDR evasion research); call it out for that audience but don't treat
  it as a feature change.
- **Removed** syscalls are rare and noteworthy — a capability retired or merged.
- ntoskrnl carries the `Nt*` table; win32k binaries carry the `win32k` shadow
  table (`NtUser*`/`NtGdi*`). Diff both when GUI surface matters.

## 4. Security mitigation flags — the priority target

Mitigations are usually represented as **bitfields in a structure** or **enum
values**, not as standalone exports. Critically, the mitigation bitfields are
*anonymous* structs (`_EPROCESS::MitigationFlagsValues` is typed `_unnamed_0xNNNN`),
so a new bit hides inside an anonymous type whose id churns between builds. The
diff script handles this: check **`types.resolved_member_changes`** first — it
follows each anonymous member back to its named parent and reports new/removed bits
as `<parent>::<member>` (e.g. `_EPROCESS::MitigationFlags2Values` gaining
`RedirectionTrustPolicyEnabled : 1`). Then also look at these structures directly:

- **`_PS_MITIGATION_OPTIONS` / `_PS_MITIGATION_OPTIONS2` / `_PS_MITIGATION_AUDIT_OPTIONS`**
  — per-process mitigation policy bitmaps. New 4-bit nibble fields here = a new
  process mitigation (e.g. ACG, CIG, blocking non-MS binaries, redirection-guard,
  user-shadow-stack, pointer auth). This is the first place to check.
- **`_EPROCESS` / `_KPROCESS` flag bitfields** (e.g. `MitigationFlags`,
  `MitigationFlags2`, `MitigationFlags3`, `Flags`) — runtime mitigation state.
  New single-bit fields named `*Enabled`/`*Audit` are new mitigations or telemetry.
- **`_PS_PROTECTION`** — Protected Process Light (PPL) signer/type. New signer
  enum values = new protected-process classes.
- **CET / shadow stacks** — fields/types containing `Cet`, `ShadowStack`,
  `Ssp`, `UserCet`, `KernelCet`. Kernel CET (`kCET`) hardens ROP.
- **CFG / XFG** — Control Flow Guard / eXtended Flow Guard: `Guard`, `Cfg`, `Xfg`,
  `GuardFlags`. New bits tighten indirect-call protection.
- **Code Integrity (`ci.dll`)** — types/enums with `Ci`, `Policy`, `Wdac`, `Hvci`,
  `SiPolicy`, `Signing`. New policy fields = tightened driver/usermode signing.
- **Token/authz (`Se*`, `_SEP_TOKEN_*`, `_TOKEN`)** — new privilege bits,
  AppContainer/capability fields, trust labels.
- **VBS/secure kernel** — `Vsm`, `Vsl`, `Ium`, `Secure`, enclave fields.

When you see a new bitfield, state the structure, the field name, its width/offset
if shown, and the mitigation it most plausibly implements. If a new field merely
*reserves* bits (`SpareBits`, padding), say so — not every new bit is a feature.

## 5. Key structures to watch

`_EPROCESS`, `_KPROCESS`, `_ETHREAD`, `_KTHREAD` (process/thread state and flags);
`_PS_MITIGATION_OPTIONS*`, `_PS_PROTECTION`; `_TOKEN`, `_SEP_TOKEN_PRIVILEGES`;
`_OBJECT_HEADER`, object-type structs; `_HANDLE_TABLE*`; `_MMVAD*`, `_MMPTE`;
`_KPRCB`, `_KPCR` (per-CPU, where CET/speculation state hides); CI policy structs
in `ci.dll`; `_ALPC_*`. A new *field* in one of these is often more telling than a
whole new type, because it shows an existing object gaining a new capability.

## 6. Phrasing an interpretation

For each finding, aim for: **component (from prefix) → subsystem → hypothesis about
the feature/mitigation → security impact**. Example shape:

> `NtSetInformationProcess` gains a new info class `ProcessFooMitigationPolicy`
> (new enum value in `_PROCESSINFOCLASS`) alongside a new nibble in
> `_PS_MITIGATION_OPTIONS2`. This is consistent with a new opt-in process
> mitigation; the `Ps` ownership and the `Options2` overflow suggest the existing
> bitmap was full. Impact: defenders gain a new hardening toggle; researchers
> should reverse `PspSetMitigationPolicy` to learn the enforcement and whether it
> is audit-only first.

Be explicit about confidence. Use "likely / consistent with / appears to" for
inferences and reserve definite statements for what the data shows directly (a
name exists, a field was added). When unsure, name the concrete next step a
researcher would take (reverse the routine, check Microsoft docs / public PDB
symbols, diff the disassembly in IDA/Ghidra/BinDiff). Honest uncertainty beats a
confident wrong guess.

---

## 7. Security-relevant features & components beyond mitigations

Mitigation flags are the most obvious finding, but a version diff often reveals
other security-relevant work that matters just as much to three audiences:
**anti-malware / EDR developers**, **anti-cheat developers**, and **vulnerability
researchers**. Actively look for the categories below and, for each finding, say
which audience(s) should care and why. The same change can serve more than one.

### Detection & telemetry surface (primarily EDR, also anti-cheat)
EDR and kernel anti-cheat both live and die by the visibility the OS gives them.
New entries here are often the highest-value finding even though they aren't
"mitigations":

- **ETW providers / events** — symbols/types containing `Etw`, `Provider`,
  `EtwWrite`, GUID blobs, or `*EtwEvent*`. The crown jewel is the **Threat
  Intelligence** channel: `EtwTi*` routines (e.g. `EtwTiLogReadWriteVm`,
  `EtwTiLogAllocExecVm`, `EtwTiLogProtectExecVm`, `EtwTiLogSetContextThread`,
  `EtwTiLogDriverObjectLoad`, `EtwTiLogRedirectionTrustPolicy`). A new `EtwTiLog*`
  routine = a new kernel event EDR/anti-cheat can subscribe to (or that defenders
  must now account for). New non-Ti providers expand audit/forensic coverage.
- **Notification callbacks** — registration surface that drivers (EDR/AC) hook:
  `PsSetCreateProcessNotifyRoutine[Ex2]`, `PsSetCreateThreadNotifyRoutine[Ex]`,
  `PsSetLoadImageNotifyRoutine[Ex]`, `ObRegisterCallbacks` (handle-operation
  filtering), `CmRegisterCallback[Ex]` (registry). New `*NotifyRoutine*`,
  `*Callback*`, or callout-table entries change what products can observe or
  what attackers can tamper with. New `Ex`/`Ex2` variants usually add flags or
  context — diff the associated struct.
- **AMSI / script & content scanning** — `Amsi*`, scan interfaces, content
  inspection hooks: new places malware content gets surfaced to scanners.

### Code integrity, signing & boot trust (EDR + vuln research)
- **ELAM** (Early-Launch Anti-Malware) — `Elam*`, early-boot driver vetting.
- **Code Integrity / WDAC** (`ci.dll`, `Ci*`, `SiPolicy`, `Hvci`, `Wdac`) — driver
  blocklist, signing-level, and policy changes. New `Ci*` = tightened DSE / driver
  loading, directly relevant to BYOVD research and to EDR self-protection.
- **Protected Process Light** (`_PS_PROTECTION`, signer enums) — which signers can
  run protected. Central to anti-cheat (protect the game) and to EDR
  self-defense (protect the agent). New signer classes are notable.

### Process / object / handle hardening (anti-cheat + vuln research)
- New `Ob` object types, handle-table changes, `ObRegisterCallbacks` altitude or
  pre/post-op changes — anti-cheat uses these to block handle theft of game
  processes; researchers probe them for bypasses.
- Anti-tamper, integrity-check, or self-protection routines (`*Integrity*`,
  `*Tamper*`, `*SelfProtect*`).

### Virtualization-based security (all three)
`Vsm`/`Vsl`/`Ium`/`Secure*`/`Skci`/enclave surface — VBS/HVCI/Credential Guard.
New secure-kernel calls or trustlet surface matter for both hardening analysis and
secure-kernel vuln research.

### Brand-new drivers, modules, or components
A binary appearing in the diff that wasn't tracked before, or a large cluster of
new routines under a previously-absent prefix, can signal a **new feature/component**
(e.g. a new security driver, a new subsystem). Call it out as a unit, hypothesize
its role from the names, and flag it as something to pull the PE/PDB for and reverse.

### How to tag audiences (quick guide)
- New **telemetry / callbacks / ETW(Ti)** → **EDR** first (new visibility or a
  closed blind spot), often **anti-cheat** too.
- New **process/handle/object protection, PPL, anti-tamper, VBS** → **anti-cheat**
  and **EDR self-protection**.
- New **syscalls, IOCTLs, parsing surface, drivers, widened structs, low-priv
  callback registration** → **vulnerability researchers** (attack surface), and
  note if a mitigation simultaneously *removes* a known primitive.
- New **CI/WDAC/ELAM/signing** → **EDR** (self-protection, BYOVD defense) and
  **vuln research** (bypass surface).

Don't force a tag where it doesn't fit, and don't invent relevance — if a change is
purely functional with no security angle, say so briefly and move on.
