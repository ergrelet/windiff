# Binary Download

The idea is to leverage [Winbindex](https://github.com/m417z/winbindex) from
@m417z, to obtain download links to the required binary files.

## Downloading PEs

We can simply use the following URL: `https://winbindex.m417z.com/data/by_filename_compressed/<filename>.json.gz`.
For example, with `kernel32.dll`, this looks like: `https://winbindex.m417z.com/data/by_filename_compressed/kernel32.dll.json.gz`.

These files contain information on all `kernel32.dll` PEs indexed by Winbindex.

From this file's content we can find the version we're looking for and deduce
the file's MSDL download URL (with `fileInfo.timestamp` and `fileInfo.virtualSize`):
`https://msdl.microsoft.com/download/symbols/<pe_name>/<timestamp><image_size>/<pe_name>`.

## Downloading PDBs

To download PDBs for given PEs, we must first download the target PEs and
generate the PDB's MSDN URLs from the debug information present in those PEs.

## Configuration

### Ideas

- It's easier to configure a database's creation once in a configuration file,
  rather than passing countless command-line arguments.
- This also allows to easily rebuild a database given the original configuration
  file that produced it.
- We want to be able to fetch one or multiple binaries, in one or multiple versions.

Example:

```json
{
  "oses": [{ "version": "21H1", "update": "BASE", "architecture": "amd64" }],
  "binaries": {
    "kernel32.dll": { "extracted_information": ["EXPORTS"] },
    "user32.dll": { "extracted_information": ["EXPORTS"] }
  }
}
```
