// Index data
export type WinDiffIndexData = {
  oses: WinDiffIndexOS[];
  binaries: WinDiffIndexBinary[];
  // Each map associates an OS path suffix ("version_update_architecture") to the
  // binaries that have a non-empty data set of the corresponding kind for that
  // OS version. Used to filter the binary dropdown on the Debug Symbols, Modules
  // and (Reconstructed) Types tabs. Optional for backward-compat with older
  // index files.
  binaries_with_symbols?: { [osPathSuffix: string]: WinDiffIndexBinary[] };
  binaries_with_modules?: { [osPathSuffix: string]: WinDiffIndexBinary[] };
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
