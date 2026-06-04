// Index data
export type WinDiffIndexData = {
  oses: WinDiffIndexOS[];
  binaries: WinDiffIndexBinary[];
  // Maps an OS path suffix ("version_update_architecture") to the binaries that
  // have a non-empty type map for that OS version. Optional for backward-compat
  // with older index files.
  binaries_with_types?: { [osPathSuffix: string]: WinDiffIndexBinary[] };
};
export type WinDiffIndexOS = {
  version: string;
  update: string;
  build_number: string;
  architecture: string;
};
export type WinDiffIndexBinary = string;

// File data
export type WinDiffFileData = {
  exports: string[];
  symbols: string[];
  modules: string[];
  types: { [typeName: string]: string };
  syscalls: { [syscallId: string]: string };
};
