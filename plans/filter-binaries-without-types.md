# Filter binaries without type information from the type tabs

## Issue

> In Syscalls tab, you only list modules which have syscalls. Do the same for
> types, as most are just empty.

Filter out DLL/EXE files that have no type information (empty type list) so they
don't appear in the binary dropdown on the **Types** and **Reconstructed Types**
tabs.

Additional requirement: the dropdown must be filtered **per OS version**.
- **Browse mode:** show only binaries that have a non-empty type map for the
  selected OS version.
- **Diff mode:** show a binary if it has types in **at least one** of the two
  selected versions (union). This keeps binaries where types were added/removed
  between versions — exactly what's interesting to diff. ("filter out binaries
  with no types for either version" = drop only those empty in *both*.)

## Background / key constraint

The binary dropdown is populated **only** from the index file
(`index.json.gz`), which lists `oses` and a flat `binaries` set — long before any
per-binary type data is fetched. The Syscalls tab filters using a *hardcoded*
list (`supportedBinariesForSyscalls` in `data_explorer.tsx`).

Types can't be hardcoded: even though all configured binaries have the `TYPES`
flag, most produce an **empty type map** (no usable type info in the PDB). So the
filter must be **data-driven**: the CLI records which binaries actually produced
non-empty types, per OS version, and writes that into the index; the frontend
then filters the dropdown on the two type tabs (`Tab.TypeList` = "Types",
`Tab.Types` = "Reconstructed Types").

The combobox is controlled by `selectedOption` (display value) and `idOnChange`
decides whether `onChange` returns an array index or the value. Because
`selectedBinaryId` is currently an **index** into a list that now changes when
you change OS, changing OS would silently re-point the index to a *different*
binary. The fix is to track the selected binary by **name**.

---

## Part 1 — CLI: emit per-OS-version type presence

### `windiff_cli/src/database.rs`

1. **Index struct** (`DatabaseIndex`) — add a map keyed by the OS path-suffix (the
   same `version_update_architecture` string used for db filenames and by the
   frontend's `osVersionToPathSuffix`):

   ```rust
   #[derive(Serialize, Debug, Default)]
   pub struct DatabaseIndex {
       pub oses: BTreeSet<OSVersion>,
       pub binaries: BTreeSet<String>,
       /// os_suffix ("version_update_architecture") -> binaries that produced
       /// a non-empty `types` map for that OS version
       pub binaries_with_types: BTreeMap<String, BTreeSet<String>>,
   }
   ```

2. **Report non-empty types per generation.** Change `generate_database_for_pe`
   and `generate_database_for_pe_version` to return `Result<bool>` (true when
   `!database.types.is_empty()`), computed after the types-extraction block.

3. **Collect in `generate_databases`.** The stream currently discards results.
   Change each item's closure to return `Result<Option<(String, String)>>` =
   `Some((os_suffix, binary_name))` when types were non-empty. `os_suffix` is
   built from the `DownloadedPEVersion` fields:
   `format!("{}_{}_{}", pe_version.os_version, pe_version.os_update, pe_version.architecture.to_str())`
   (identical to the filename suffix). Fold the collected pairs into a
   `BTreeMap<String, BTreeSet<String>>`. Change the function to **return
   `Result<BTreeMap<String, BTreeSet<String>>>`**, and when `generate_index` is
   set, pass that map to `generate_database_index`.

4. **`generate_database_index`** gains a
   `binaries_with_types: &BTreeMap<String, BTreeSet<String>>` parameter and writes
   it into the index.

### `windiff_cli/src/main.rs`

5. **normal_mode**: index is generated inside `generate_databases`; ignore the
   returned map.

6. **low_storage_mode**: accumulate each per-binary call's returned map into a
   global `BTreeMap<String, BTreeSet<String>>` (merge the per-suffix sets), and
   pass it to the final `generate_database_index` call.

---

## Part 2 — Frontend: OS-dependent dropdown + name-based binary selection

### `windiff_frontend/src/app/windiff_types.ts`

7. ```ts
   export type WinDiffIndexData = {
     oses: WinDiffIndexOS[];
     binaries: WinDiffIndexBinary[];
     binaries_with_types?: { [osPathSuffix: string]: WinDiffIndexBinary[] }; // optional → backward-compat
   };
   ```

### `windiff_frontend/src/app/data_explorer.tsx`

8. **Switch binary selection from index to name** (core change that makes
   OS-dependent filtering correct).
   - Replace the `selectedBinaryId` state with
     `const [selectedBinaryName, setSelectedBinaryName] = useState("")`.
   - The binary combobox becomes `idOnChange={false}` with
     `onChange={(value) => setSelectedBinaryName(value)}` and
     `selectedOption={resolvedBinaryName}`.
   - Selection now survives list changes (changing OS keeps the same binary if
     still present), and this fixes the same latent index-overflow bug that
     exists today for the Syscalls tab.

9. **Filter helper** (next to `supportedBinariesForSyscalls`):

   ```ts
   function filterBinariesForTab(
     binaries: string[],
     tab: Tab,
     binariesWithTypes: { [k: string]: string[] } | undefined,
     leftSuffix: string,
     rightSuffix: string | null, // null in browse mode
   ): string[] {
     if (tab === Tab.Sycalls) {
       return binaries.filter((b) => supportedBinariesForSyscalls.includes(b));
     }
     if ((tab === Tab.TypeList || tab === Tab.Types) && binariesWithTypes) {
       const allowed = new Set(binariesWithTypes[leftSuffix] ?? []);
       if (rightSuffix !== null) {
         (binariesWithTypes[rightSuffix] ?? []).forEach((b) => allowed.add(b)); // union
       }
       return binaries.filter((b) => allowed.has(b));
     }
     return binaries; // missing field or non-type/syscall tab → no filtering
   }
   ```

10. **Apply it in the render path** (`sortedOSPathSuffixes` is already computed
    just above, so suffixes are available here):

    ```ts
    sortedBinaryNames = indexData.binaries.sort(compareStrings);
    const leftSuffix = sortedOSPathSuffixes[selectedLeftOSVersionId];
    const rightSuffix =
      mode === ExplorerMode.Diff ? sortedOSPathSuffixes[selectedRightOSVersionId] : null;
    sortedBinaryNames = filterBinariesForTab(
      sortedBinaryNames, currentTabId, indexData.binaries_with_types, leftSuffix, rightSuffix
    );

    // Resolve the selected binary by name against the (possibly filtered) list,
    // falling back to the first entry when the current one isn't available for
    // these OS version(s). Render-time default — same pattern as selectedType.
    const resolvedBinaryName =
      selectedBinaryName && sortedBinaryNames.includes(selectedBinaryName)
        ? selectedBinaryName
        : (sortedBinaryNames[0] ?? "");
    ```

    Use `resolvedBinaryName` for `leftFileName`/`rightFileName` and as the
    combobox's `selectedOption`.

11. **Simplify URL hydration.** Because `PARAM_BIN` already stores the binary
    **name**, the index→index resolution block (and its syscalls special-case)
    collapses to:

    ```ts
    if (initialBin.current !== null) setSelectedBinaryName(initialBin.current);
    ```

12. **Permalink effect**: write `resolvedBinaryName` for `PARAM_BIN`, and replace
    the `selectedBinaryId` dependency with `selectedBinaryName`.

Net effect: on the two type tabs the binary dropdown lists only binaries with
type data for the current OS selection; switching OS re-filters live and
preserves your binary when possible; other tabs are unchanged. Syscalls keeps its
static filter.

---

## Part 3 — Regenerate data & validate

13. Regenerate so the index carries the new map:

    ```bash
    cd windiff_cli
    cargo run --release -- ../ci/db_configuration_mini.json ../windiff_frontend/public/
    gzip -dc ../windiff_frontend/public/index.json.gz | python3 -m json.tool | head -40
    ```

14. `cargo fmt --check && cargo clippy && cargo test` (CLI);
    `npm run lint && npm run build` (frontend). In `npm run dev`: confirm the type
    tabs' dropdown shrinks to binaries with types, updates when changing OS in
    browse mode, shows the union of both versions in diff mode, and that a
    type-tab permalink restores the right binary.

---

## Notes / decisions

- **Index size:** the map duplicates binary names per OS version, but it's small
  (tens of binaries × tens of versions of short strings) and gzipped —
  negligible.
- **Backward compatibility:** missing `binaries_with_types` ⇒
  `filterBinariesForTab` returns the full list, so a stale index never yields an
  empty dropdown.
- **Name-based selection** is a deliberate, contained refactor (binary only — OS
  lists don't change with selection, so those stay index-based). It's required
  for correctness once the list depends on OS, and removes the fragile hydration
  code as a bonus.
