// Index data
export type WinDiffIndexData = {
  oses: WinDiffIndexOS[];
  binaries: WinDiffIndexBinary[];
};
type WinDiffIndexOS = { version: string; update: string; architecture: string };
type WinDiffIndexBinary = string;

// File data
export type WinDiffFileData = {
  exports: string[];
  symbols: string[];
  modules: string[];
  types: { [typeName: string]: string };
};
