# Roles of the tracked Windows binaries

Knowing what each binary is responsible for lets you predict where a given kind of
change should appear and judge whether a finding is significant. These are the
binaries WinDiff commonly tracks (see `ci/db_configuration.json` for the live set).

| Binary | Role | What new changes here usually mean |
|--------|------|-------------------------------------|
| **ntoskrnl.exe** | The NT kernel: process/thread/memory/object/security managers, the `Nt*` syscall table, most mitigation logic | The center of gravity. New syscalls, new `Ps/Ke/Mm/Ob/Se` routines, and new mitigation bitfields land here. Always diff it. |
| **ntdll.dll** | User-mode native API layer: syscall stubs (`Nt*`/`Zw*`), loader (`Ldr*`), heap, RTL helpers, CSR/`Csr*`, ETW user stubs | New `Nt*` stubs mirror new kernel syscalls. Loader/heap changes can indicate new user-mode mitigations (e.g. CFG/XFG metadata, heap hardening). |
| **win32k.sys** | Kernel GUI/window-manager syscall surface (`NtUser*`), the win32k shadow syscall table | A major LPE attack surface. New `NtUser*` syscalls and win32k lockdown/filter changes matter for sandbox-escape research. |
| **win32kbase.sys** | Base win32k services split out of win32k.sys | New shared GUI primitives; pairs with win32kfull. |
| **win32kfull.sys** | Full win32k window/messaging/GDI implementation | The bulk of GUI logic; new internal routines here often back new `NtUser*` surface. |
| **ci.dll** | Code Integrity: kernel-mode driver signature enforcement (DSE), WDAC/Device Guard policy, HVCI integration | New `Ci*` routines and policy structs = tightened code-signing / driver-blocking / WDAC enforcement. Key for DSE-bypass and supply-chain research. |
| **cng.sys** | Kernel Cryptography Next Generation provider | New algorithms/providers, FIPS, key isolation. Relevant to crypto-downgrade and key-protection research. |
| **hal.dll** | Hardware Abstraction Layer | Low-level platform/CPU changes; new speculation/CPU-feature handling occasionally appears. |
| **secure kernel / `securekernel.exe`, `skci.dll`** (if tracked) | VBS secure kernel and its code-integrity module | VBS/HVCI/Credential Guard surface — high security value when present. |

## Pairing changes across binaries

The strongest findings show a coherent story across binaries. Examples:

- A new `Nt*` **syscall in ntoskrnl** + a matching **stub in ntdll** = a fully new
  user-reachable kernel service. Confirm both before claiming "new syscall".
- A new **mitigation field** in `_PS_MITIGATION_OPTIONS*` (ntoskrnl) + a new
  enum value in `_PROCESSINFOCLASS`/`_PROCESS_MITIGATION_POLICY` = a new opt-in
  process mitigation surfaced through `NtSetInformationProcess`.
- New **`Ci*` policy fields** (ci.dll) + new mitigation toggles (ntoskrnl) can
  indicate tighter driver/usermode signing tied to a process policy.
- New **`NtUser*`** (win32k) + win32k filter/lockdown changes = adjusted GUI
  attack surface for sandboxed processes.

When you spot one half of such a pair, look for the other half in the companion
binary's diff and report them together.
