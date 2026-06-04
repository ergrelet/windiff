export const PARAM_MODE = "mode";
export const PARAM_TAB = "tab";
export const PARAM_LHS = "lhs";
export const PARAM_RHS = "rhs";
export const PARAM_BIN = "bin";
export const PARAM_TYPE = "type";
export const PARAM_SC_SORT = "scsort";
export const PARAM_SC_IDS = "scids";
export const PARAM_SC_NAMES = "scnames";

export const MODE_BROWSE = "browse";
export const MODE_DIFF = "diff";

// Stable URL keys for each Tab enum value (index-aligned with Tab enum in data_explorer.tsx)
export const TAB_KEYS = [
  "exports",  // Tab.Exports = 0
  "symbols",  // Tab.Symbols = 1
  "modules",  // Tab.Modules = 2
  "typelist", // Tab.TypeList = 3
  "types",    // Tab.Types = 4
  "syscalls", // Tab.Sycalls = 5
] as const;

export function readParam(key: string): string | null {
  if (typeof window === "undefined") return null;
  return new URLSearchParams(window.location.search).get(key);
}

// Merges updates into the current URL query string without triggering navigation.
// Pass null for a key to remove it.
export function writeParams(updates: Record<string, string | null>): void {
  if (typeof window === "undefined") return;
  const params = new URLSearchParams(window.location.search);
  for (const [key, value] of Object.entries(updates)) {
    if (value === null) {
      params.delete(key);
    } else {
      params.set(key, value);
    }
  }
  const search = params.toString();
  window.history.replaceState(
    null,
    "",
    search ? `?${search}` : window.location.pathname
  );
}
