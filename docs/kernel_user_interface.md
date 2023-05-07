# Kernel-User Interface

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
