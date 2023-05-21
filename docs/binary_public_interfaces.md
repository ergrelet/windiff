# Binary Public Interfaces

The idea is to extract data about public (and internal) interfaces of different
Windows binaries.

## Database Implementation

### Single-file Idea

Example:

```json
{
  "metadata": {
    "name": "kernel32.dll", // Binary file's name
    "version": "10.0.19041.928 (WinBuild.160101.0800)", // File version
    "architecture": "amd64" // Target architecture
  },
  // Exported symbols
  "exports": ["MyApi1", "MyApi2"],
  // Debug symbols (functions have parentheses after the identifier)
  "symbols": ["MyApi1()", "MyVar2"],
  // Modules (extracted from PDB)
  "modules": ["d:\\my\\module1.obj", "d:\\my\\module2.obj"],
  // Types (extracted from PDB)
  "types": { "MyType1": "struct MyType1 {};", "MyType2": "struct MyType2 {};" }
}
```

### Multi-file Idea

- Metadata
- Public interface
- Internal interface/feature set
  - Question: identifiers only or identifiers + types?

Note: this idea hasn't been implemented
