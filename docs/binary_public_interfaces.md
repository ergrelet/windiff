# Binary Public Interfaces

The idea is to extract data about public (and internal) interfaces of different
Windows binaries.

## Database Implementation

### Multi-file Idea

- Metadata
- Public interface
- Internal interface/feature set
  - Question: identifiers only or identifiers + types?

### Single-file Idea

```json
{
  "metadata": {
    "name": "kernel32.dll", // Binary file's name
    "mode": "User", // Can be "User" or "Kernel"
    "version": "1.0.0", // File version
    "os_name": "Windows 11 22H2", // Host OS name
    "architecture": "amd64" // Target architecture
  },
  "interfaces": {
    // Extracted from exported symbols
    "public": {
      "procedures": ["MyApi1", "MyApi2"], // API identifiers or ordinal
      "data": ["MyExportedVariable"] // Data identifiers
    },
    // Extracted from PDB files
    "internal": {
      "procedures": ["MyInternalProcedure1", "MyInternalProcedure2"],
      "types": ["MyInternalStruct"]
    }
  }
}
```

## Binaries to Target

- `ntoskrnl.exe`
- `win32k.sys`
- `ntdll.dll`
- `win32u.dll`
- `kernel32.dll`
- `user32.dll`
