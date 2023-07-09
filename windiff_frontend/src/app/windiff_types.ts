// Index data
export type WinDiffIndexData = {
  oses: WinDiffIndexOS[];
  binaries: WinDiffIndexBinary[];
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
