# Kernel-User Interface

## Idea

Being able to extract "syscall interfaces" from one or both sides (user and
kernel) would be nice.

Example:

```json
{
  "syscalls": {
    // Syscall ID -> Syscall metadata
    "1337": {
      // Can be extracted from ntdll.dll's and win32u.dll's exports
      "user": {
        "location": "ntdll.dll",
        "identifier": "NtCreateFile"
      },
      // Can be extracted from ntoskrnl.exe's and win32k.sys's exports
      "kernel": {
        "location": "ntoskrnl.exe",
        "identifier": "ZwCreateFile"
      }
    }
  }
}
```

Note: this idea hasn't been implemented yet
