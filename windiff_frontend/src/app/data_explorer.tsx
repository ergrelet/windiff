"use client";

import { useState } from "react";
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

enum Tab {
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
  let [leftOSVersion, setLeftOSVersion] = useState("");
  let [rightOSVersion, setRightOSVersion] = useState("");
  let [binary, setBinary] = useState("");
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

  // Fetch index content
  const { data: indexData, error: indexError } = useSWR<WinDiffIndexData>(
    indexFilePath,
    compressedJsonFetcher
  );

  let leftFileName: string = "";
  let rightFileName: string = "";
  if (indexData) {
    if (leftOSVersion.length == 0) {
      leftOSVersion = osVersionToHumanString(indexData.oses[0]);
    }
    if (rightOSVersion.length == 0) {
      rightOSVersion = osVersionToHumanString(indexData.oses[0]);
    }
    if (binary.length == 0) {
      binary = indexData.binaries[0];
    }

    const binaryVersion = humanOsVersionToPathSuffix(leftOSVersion);
    leftFileName = `${binary}_${binaryVersion}.json.gz`;
    if (mode == ExplorerMode.Diff) {
      const binaryVersion = humanOsVersionToPathSuffix(rightOSVersion);
      rightFileName = `${binary}_${binaryVersion}.json.gz`;
    }
  }

  let { data: leftFileData, error: leftFileError } = useSWR<WinDiffFileData>(
    `/${leftFileName}`,
    compressedJsonFetcher
  );
  let { data: rightFileData, error: rightFileError } = useSWR<WinDiffFileData>(
    `/${rightFileName}`,
    compressedJsonFetcher
  );

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
        // Select the first element of the list by default
        selectedType = typeList.values().next().value;
      }
      if (currentTabId == Tab.Types) {
        return (
          <DarkCombobox
            selectedOption={selectedType}
            options={[...typeList]}
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

  // Prepare the appropriate data
  const sortedOSes: string[] = indexData.oses
    .map((osVersion: any) => osVersionToHumanString(osVersion))
    .sort(compareStrings);
  let sortedBinaries: string[] = indexData.binaries.sort(compareStrings);
  // Filter binary list if needed
  if (currentTabId == Tab.Sycalls) {
    sortedBinaries = sortedBinaries.filter((binary: string) => {
      return supportedBinariesForSyscalls.indexOf(binary) > -1;
    });
  }

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
          options={sortedOSes}
          onChange={(value) => setRightOSVersion(value)}
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
  return (
    <div className="flex flex-row justify-center items-center">
      <div className="max-w-6xl w-full space-y-2 py-2 pl-10 pr-10">
        {/* Tabs used to select the displayed data */}
        <DarkTabs
          tabs={tabNames}
          onChange={(value) => setCurrentTabId(value)}
        />
        {/* Comboboxes used to select the binary versions */}
        <div className={comboboxGridClass}>
          <DarkCombobox
            selectedOption={leftOSVersion}
            options={sortedOSes}
            onChange={(value) => setLeftOSVersion(value)}
          />

          {rightOSCombobox}

          <DarkCombobox
            selectedOption={binary}
            options={sortedBinaries}
            onChange={(value) => setBinary(value)}
          />

          {typesCombobox}

          {syscallOptionsMenu}
        </div>

        {/* Text editor */}
        <WinDiffEditor
          mode={mode}
          language={editorLanguage}
          leftData={leftData}
          rightData={rightData}
        />
      </div>
    </div>
  );
}

function osVersionToHumanString(osVersion: WinDiffIndexOS): string {
  // Normalize version names between Windows 10 and 11
  let versionPrefix = "";
  if (!osVersion.version.startsWith("11")) {
    // Windows 10
    versionPrefix = "10-";
  }

  return `Windows ${versionPrefix}${osVersion.version} ${osVersion.architecture} (${osVersion.update})`;
}

// Convert "human" versions of version strings to the corresponding file path suffixes
function humanOsVersionToPathSuffix(osVersionName: string): string {
  const versionParts = osVersionName.split(" ");
  let osVersion = versionParts[1];
  if (osVersion.startsWith("10-")) {
    // Remove added prefix
    osVersion = osVersion.substring(3);
  }

  const osArchitecture = versionParts[2];
  const osUpdateWithParentheses = versionParts[3];
  // Remove parentheses
  const osUpdate = osUpdateWithParentheses.substring(
    1,
    osUpdateWithParentheses.length - 1
  );

  return `${osVersion}_${osUpdate}_${osArchitecture}`;
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
