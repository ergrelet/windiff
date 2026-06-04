"use client";

import { useState, useEffect, useRef, type JSX } from "react";
import useSWR from "swr";
import { Editor, DiffEditor } from "@monaco-editor/react";
import pako from "pako";

import DarkTabs from "./tabs";
import DarkCombobox from "./combobox";
import {
  WinDiffFileData,
  WinDiffIndexData,
  WinDiffIndexOS,
} from "./windiff_types";
import OptionsMenu from "./options_menu";
import {
  readParam,
  writeParams,
  TAB_KEYS,
  PARAM_TAB,
  PARAM_LHS,
  PARAM_RHS,
  PARAM_BIN,
  PARAM_TYPE,
  PARAM_SC_SORT,
  PARAM_SC_IDS,
  PARAM_SC_NAMES,
} from "./permalink";

const compressedJsonFetcher = async (url: string) => {
  const response = await fetch(url);
  let binaryData = await response.arrayBuffer();
  let uintArray = new Uint8Array(binaryData);
  const jsonString = pako.inflate(uintArray, { to: "string" });
  return JSON.parse(jsonString);
};
const compareStrings = (a: string, b: string) => (a > b ? 1 : b > a ? -1 : 0);

export enum ExplorerMode {
  Browse = 0,
  Diff = 1,
}

export enum Tab {
  Exports = 0,
  Symbols = 1,
  Modules = 2,
  TypeList = 3,
  Types = 4,
  Sycalls = 5,
}

const indexFilePath: string = "/index.json.gz";
const tabNames: string[] = [
  "Exported Symbols",
  "Debug Symbols",
  "Modules",
  "Types",
  "Reconstructed Types",
  "Syscalls",
];

// List of binaries we support syscall extraction for
const supportedBinariesForSyscalls: string[] = [
  "ntdll.dll",
  "win32u.dll",
  "ntoskrnl.exe",
  "win32k.sys",
];

export default function DataExplorer({ mode }: { mode: ExplorerMode }) {
  // Tab selection
  const [currentTabId, setCurrentTabId] = useState(Tab.Exports);
  // OS and binary selection
  let [selectedLeftOSVersionId, setSelectedLeftOSVersionId] = useState(0);
  let [selectedRightOSVersionId, setSelectedRightOSVersionId] = useState(0);
  let [selectedBinaryId, setSelectedBinaryId] = useState(0);
  // Type selection
  let [selectedType, setSelectedType] = useState("");
  // Syscall options
  const [orderSyscallsByName, setOrderSyscallsByName] = useState(true);
  const [displaySyscallIds, setDisplaySyscallIds] = useState(false);
  let [displaySyscallNames, setDisplaySyscallNames] = useState(true);
  // Force displaying at least syscall names
  if (!displaySyscallIds && !displaySyscallNames) {
    setDisplaySyscallNames(true);
  }

  // Snapshot of initial URL params — read once at mount, stable for the session
  const initialLhs = useRef<string | null>(null);
  const initialRhs = useRef<string | null>(null);
  const initialBin = useRef<string | null>(null);
  const initialType = useRef<string | null>(null);

  // Hydration flags so we only apply URL → state once per data load
  const indexHydrated = useRef(false);
  const fileHydrated = useRef(false);

  // Read all URL params at mount and hydrate fields available without data
  useEffect(() => {
    initialLhs.current = readParam(PARAM_LHS);
    initialRhs.current = readParam(PARAM_RHS);
    initialBin.current = readParam(PARAM_BIN);
    initialType.current = readParam(PARAM_TYPE);

    // Intentional: hydrate state from URL params once after mount to avoid SSR
    // hydration mismatches. Disabling set-state-in-effect for this block.
    /* eslint-disable react-hooks/set-state-in-effect */
    const tabParam = readParam(PARAM_TAB);
    const tabIndex = TAB_KEYS.indexOf(tabParam as (typeof TAB_KEYS)[number]);
    if (tabIndex !== -1) {
      setCurrentTabId(tabIndex);
    }

    const scSort = readParam(PARAM_SC_SORT);
    if (scSort !== null) {
      setOrderSyscallsByName(scSort !== "id");
    }

    const scIds = readParam(PARAM_SC_IDS);
    if (scIds !== null) {
      setDisplaySyscallIds(scIds === "1");
    }

    const scNames = readParam(PARAM_SC_NAMES);
    if (scNames !== null) {
      setDisplaySyscallNames(scNames === "1");
    }
    /* eslint-enable react-hooks/set-state-in-effect */
  }, []);

  // Fetch index content
  const { data: indexData, error: indexError } = useSWR<WinDiffIndexData>(
    indexFilePath,
    compressedJsonFetcher
  );

  let sortedOSNames: string[] = [];
  let sortedOSPathSuffixes: string[] = [];
  let sortedBinaryNames: string[] = [];
  let leftFileName: string = "";
  let rightFileName: string = "";
  let leftOSVersion: string = "";
  let rightOSVersion: string = "";
  let selectedBinaryName: string = "";
  if (indexData) {
    // Prepare sorted lists for OS names and path suffixes used to fetch the
    // corresponding binary versions
    // Use a Collator for natural ordering
    let collator = new Intl.Collator(undefined, {
      numeric: true,
      sensitivity: "base",
    });
    [sortedOSNames, sortedOSPathSuffixes] = indexData.oses
      .map((osVersion: WinDiffIndexOS) => [
        osVersionToHumanString(osVersion),
        osVersionToPathSuffix(osVersion),
      ])
      .sort((a: string[], b: string[]) => collator.compare(a[0], b[0]))
      .reduce(
        (accumulator: string[][], current: string[]) => {
          accumulator[0].push(current[0]);
          accumulator[1].push(current[1]);
          return accumulator;
        },
        [[], []]
      );

    // Sort binary names
    sortedBinaryNames = indexData.binaries.sort(compareStrings);
    // Filter binary list if needed
    if (currentTabId == Tab.Sycalls) {
      sortedBinaryNames = sortedBinaryNames.filter((binary: string) => {
        return supportedBinariesForSyscalls.indexOf(binary) > -1;
      });
    }

    leftOSVersion = sortedOSNames[selectedLeftOSVersionId];
    rightOSVersion = sortedOSNames[selectedRightOSVersionId];
    selectedBinaryName = sortedBinaryNames[selectedBinaryId];

    const binaryVersion = sortedOSPathSuffixes[selectedLeftOSVersionId];
    leftFileName = `${selectedBinaryName}_${binaryVersion}.json.gz`;
    if (mode == ExplorerMode.Diff) {
      const binaryVersion = sortedOSPathSuffixes[selectedRightOSVersionId];
      rightFileName = `${selectedBinaryName}_${binaryVersion}.json.gz`;
    }
  }

  // Hydrate OS and binary selections from URL once the index is loaded
  useEffect(() => {
    if (!indexData || indexHydrated.current) return;
    indexHydrated.current = true;

    let collator = new Intl.Collator(undefined, {
      numeric: true,
      sensitivity: "base",
    });
    const [names, suffixes] = indexData.oses
      .map((os: WinDiffIndexOS) => [
        osVersionToHumanString(os),
        osVersionToPathSuffix(os),
      ])
      .sort((a: string[], b: string[]) => collator.compare(a[0], b[0]))
      .reduce(
        (acc: string[][], cur: string[]) => {
          acc[0].push(cur[0]);
          acc[1].push(cur[1]);
          return acc;
        },
        [[], []]
      );

    if (initialLhs.current !== null) {
      const idx = suffixes.indexOf(initialLhs.current);
      if (idx !== -1) setSelectedLeftOSVersionId(idx);
    }
    if (initialRhs.current !== null) {
      const idx = suffixes.indexOf(initialRhs.current);
      if (idx !== -1) setSelectedRightOSVersionId(idx);
    }

    // The binary dropdown is filtered to syscall-capable binaries on the
    // Syscalls tab, so resolve the index against the same filtered list the
    // render uses (otherwise the index points past the end of the filtered
    // list and the field shows up empty). Derive the tab from the URL rather
    // than `currentTabId` state to stay correct regardless of effect timing.
    let bins = [...indexData.binaries].sort(compareStrings);
    if (readParam(PARAM_TAB) === TAB_KEYS[Tab.Sycalls]) {
      bins = bins.filter(
        (binary) => supportedBinariesForSyscalls.indexOf(binary) > -1
      );
    }
    if (initialBin.current !== null) {
      const idx = bins.indexOf(initialBin.current);
      if (idx !== -1) setSelectedBinaryId(idx);
    }

    // suppress unused variable warning
    void names;
  }, [indexData]);

  // `keepPreviousData` keeps the editor mounted with the previously loaded data
  // while a new file is fetched, so we never feed Monaco a transient "Loading..."
  // placeholder once it has real content (see the editor render below).
  let { data: leftFileData, error: leftFileError } = useSWR<WinDiffFileData>(
    `/${leftFileName}`,
    compressedJsonFetcher,
    { keepPreviousData: true }
  );
  let { data: rightFileData, error: rightFileError } = useSWR<WinDiffFileData>(
    `/${rightFileName}`,
    compressedJsonFetcher,
    { keepPreviousData: true }
  );

  // Hydrate selected type from URL once the left file is loaded
  useEffect(() => {
    if (!leftFileData || fileHydrated.current) return;
    fileHydrated.current = true;

    if (initialType.current !== null && initialType.current in leftFileData.types) {
      setSelectedType(initialType.current);
    }
  }, [leftFileData]);

  // Keep URL in sync with all selections whenever anything changes.
  // sortedOSPathSuffixes and selectedBinaryName are derived from indexData and
  // selected*Id state which are already listed; omitting them avoids infinite
  // re-renders from new array refs on every render.
  useEffect(() => {
    if (!indexData || sortedOSPathSuffixes.length === 0) return;

    writeParams({
      [PARAM_TAB]: TAB_KEYS[currentTabId],
      [PARAM_LHS]: sortedOSPathSuffixes[selectedLeftOSVersionId] ?? null,
      [PARAM_RHS]:
        mode === ExplorerMode.Diff
          ? sortedOSPathSuffixes[selectedRightOSVersionId] ?? null
          : null,
      [PARAM_BIN]: selectedBinaryName || null,
      [PARAM_TYPE]: currentTabId === Tab.Types ? selectedType || null : null,
      [PARAM_SC_SORT]:
        currentTabId === Tab.Sycalls
          ? orderSyscallsByName
            ? "name"
            : "id"
          : null,
      [PARAM_SC_IDS]:
        currentTabId === Tab.Sycalls ? (displaySyscallIds ? "1" : "0") : null,
      [PARAM_SC_NAMES]:
        currentTabId === Tab.Sycalls ? (displaySyscallNames ? "1" : "0") : null,
    });
  }, [ // eslint-disable-line react-hooks/exhaustive-deps
    currentTabId,
    selectedLeftOSVersionId,
    selectedRightOSVersionId,
    selectedBinaryId,
    selectedType,
    orderSyscallsByName,
    displaySyscallIds,
    displaySyscallNames,
    mode,
    indexData,
  ]);

  if (indexError) {
    return <div>Failed to load</div>;
  }

  if (!indexData) {
    return <div>Loading...</div>;
  }

  // Setup the combobox used to select types if needed
  const typesCombobox: JSX.Element = (() => {
    if (leftFileData) {
      let typeList: Set<string> | string[];
      if (rightFileData) {
        typeList = new Set(
          Object.keys(leftFileData.types).concat(
            Object.keys(rightFileData.types)
          )
        );
      } else {
        typeList = Object.keys(leftFileData.types);
      }
      if (selectedType.length == 0) {
        // Select the first element of the list by default for this render
        // eslint-disable-next-line react-hooks/immutability -- intentional: local render-time default, not a state mutation
        selectedType = typeList.values().next().value ?? "";
      }
      if (currentTabId == Tab.Types) {
        return (
          <DarkCombobox
            selectedOption={selectedType}
            options={[...typeList]}
            idOnChange={false}
            onChange={(value) => setSelectedType(value)}
          />
        );
      }
    }
    return <></>;
  })();

  const syscallOptionsMenu: JSX.Element = (() => {
    if (currentTabId == Tab.Sycalls) {
      const syscallOptions = [
        {
          name: "Order syscalls by name",
          checked: orderSyscallsByName,
          updateState: (checked: boolean) => setOrderSyscallsByName(checked),
        },
        {
          name: "Display syscall IDs",
          checked: displaySyscallIds,
          updateState: (checked: boolean) => setDisplaySyscallIds(checked),
        },
        {
          name: "Display syscall names",
          checked: displaySyscallNames,
          updateState: (checked: boolean) => setDisplaySyscallNames(checked),
        },
      ];
      return (
        <div>
          <OptionsMenu options={syscallOptions} />
        </div>
      );
    }
    return <></>;
  })();

  // Data displayed on the left (in diff mode) or in the center (in browse mode)
  const leftData: string = (() => {
    if (!leftFileData) {
      return leftFileError ? "" : "Loading...";
    } else {
      return getEditorDataFromFileData(
        leftFileData,
        currentTabId,
        selectedType,
        orderSyscallsByName,
        displaySyscallIds,
        displaySyscallNames
      );
    }
  })();
  // Data displayed on the right (in diff mode)
  const rightData: string = (() => {
    if (!rightFileData) {
      return rightFileError ? "" : "Loading...";
    } else {
      return getEditorDataFromFileData(
        rightFileData,
        currentTabId,
        selectedType,
        orderSyscallsByName,
        displaySyscallIds,
        displaySyscallNames
      );
    }
  })();

  // Setup the a second combobox to select the OS version displayed on the right
  // if needed
  const rightOSCombobox: JSX.Element = (() => {
    if (mode == ExplorerMode.Diff) {
      return (
        <DarkCombobox
          selectedOption={rightOSVersion}
          options={sortedOSNames}
          idOnChange={true}
          onChange={(value) => setSelectedRightOSVersionId(value)}
        />
      );
    }
    return <></>;
  })();

  // Setup the combobox grid with 3 columns in browsing mode and 2 columns in
  // diffing mode
  const comboboxGridClass =
    mode == ExplorerMode.Browse
      ? "grid grid-cols-3 gap-2"
      : "grid grid-cols-2 gap-2";
  const editorLanguage = currentTabId == Tab.Types ? "cpp" : "plaintext";

  // Only mount the editor once the required data is available (or has errored),
  // so Monaco's models are never created from a transient "Loading..." string.
  // Doing so previously caused the modified (right) pane to stay stuck on
  // "Loading..." on some browsers (Firefox), because @monaco-editor/react drops
  // value updates that arrive while the diff editor is still initializing.
  const leftReady = leftFileData !== undefined || leftFileError !== undefined;
  const rightReady =
    mode == ExplorerMode.Browse ||
    rightFileData !== undefined ||
    rightFileError !== undefined;
  const editorReady = leftReady && rightReady;

  return (
    <div className="flex flex-row justify-center items-center">
      <div className="max-w-6xl w-full space-y-2 py-2 pl-10 pr-10">
        {/* Tabs used to select the displayed data */}
        <DarkTabs
          tabs={tabNames}
          selectedIndex={currentTabId}
          onChange={(value) => setCurrentTabId(value)}
        />
        {/* Comboboxes used to select the binary versions */}
        <div className={comboboxGridClass}>
          <DarkCombobox
            selectedOption={leftOSVersion}
            options={sortedOSNames}
            idOnChange={true}
            onChange={(value) => setSelectedLeftOSVersionId(value)}
          />

          {rightOSCombobox}

          <DarkCombobox
            selectedOption={selectedBinaryName}
            options={sortedBinaryNames}
            idOnChange={true}
            onChange={(value) => setSelectedBinaryId(value)}
          />

          {typesCombobox}

          {syscallOptionsMenu}
        </div>

        {/* Text editor */}
        {editorReady ? (
          <WinDiffEditor
            mode={mode}
            language={editorLanguage}
            leftData={leftData}
            rightData={rightData}
          />
        ) : (
          <div className="h-[70vh] flex items-center justify-center text-gray-400">
            Loading...
          </div>
        )}
      </div>
    </div>
  );
}

function osVersionToHumanString(osVersion: WinDiffIndexOS): string {
  const splitOSVersion = osVersion.version.split("-");

  let windowsVersion: number = 0;
  let featureUpdate: string = "";
  // Windows 10
  if (splitOSVersion.length == 1) {
    windowsVersion = 10;
    featureUpdate = splitOSVersion[0];
  } else if (splitOSVersion.length == 2 && splitOSVersion[0] == "11") {
    windowsVersion = 11;
    featureUpdate = splitOSVersion[1];
  }

  let buildName = "";
  if (osVersion.build_number.length == 0) {
    // Note(ergrelet): happens for "BASE" versions
    buildName = osVersion.update;
  } else {
    buildName = `Build ${osVersion.build_number}`;
  }

  return `Windows ${windowsVersion} ${featureUpdate} ${osVersion.architecture} (${buildName})`;
}

function osVersionToPathSuffix(osVersion: WinDiffIndexOS): string {
  return `${osVersion.version}_${osVersion.update}_${osVersion.architecture}`;
}

function getEditorDataFromFileData(
  fileData: WinDiffFileData,
  tab: Tab,
  selectedType: string | undefined,
  orderSyscallsByName: boolean,
  displaySyscallIds: boolean,
  displaySyscallNames: boolean
): string {
  switch (tab) {
    default:
    case Tab.Exports:
      return fileData.exports.join("\n");
    case Tab.Symbols:
      return fileData.symbols.join("\n");
    case Tab.Modules:
      return fileData.modules.join("\n");
    case Tab.TypeList:
      return Object.keys(fileData.types).join("\n");
    case Tab.Types:
      return selectedType ? fileData.types[selectedType] : "";
    case Tab.Sycalls:
      let syscalls = Object.entries(fileData.syscalls);
      if (orderSyscallsByName) {
        syscalls.sort((a, b) => compareStrings(a[1], b[1]));
      }
      return syscalls
        .map((value) => {
          if (displaySyscallIds && displaySyscallNames) {
            return `0x${parseInt(value[0], 10)
              .toString(16)
              .padStart(4, "0")}: ${value[1]}`;
          }
          if (displaySyscallIds) {
            return `0x${parseInt(value[0], 10).toString(16).padStart(4, "0")}`;
          }
          if (displaySyscallNames) {
            return value[1];
          }
        })
        .join("\n");
  }
}

function WinDiffEditor({
  mode,
  language,
  leftData,
  rightData,
}: {
  mode: ExplorerMode;
  language: string;
  leftData: string;
  rightData: string;
}): JSX.Element {
  switch (mode) {
    default:
    case ExplorerMode.Browse:
      return (
        <Editor
          height="70vh"
          theme="vs-dark"
          value={leftData}
          language={language}
        />
      );
    case ExplorerMode.Diff:
      return (
        <DiffView
          oldRevision={leftData}
          newRevision={rightData}
          language={language}
        />
      );
  }
}

function DiffView({
  oldRevision,
  newRevision,
  language,
}: {
  oldRevision: string;
  newRevision: string;
  language: string;
}): JSX.Element {
  return (
    <DiffEditor
      height="63vh"
      theme="vs-dark"
      originalLanguage={language}
      modifiedLanguage={language}
      original={oldRevision}
      modified={newRevision}
    />
  );
}
