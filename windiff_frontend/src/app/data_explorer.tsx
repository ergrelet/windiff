"use client";

import { useState } from "react";
import useSWR from "swr";
import { Editor, DiffEditor } from "@monaco-editor/react";
import pako from "pako";

import DarkTabs from "./tabs";
import DarkCombobox from "./combobox";

const compressedJsonFetcher = async (url: string) => {
  const response = await fetch(url);
  let binaryData = await response.arrayBuffer();
  let uintArray = new Uint8Array(binaryData);
  const jsonString = pako.inflate(uintArray, { to: "string" });
  return JSON.parse(jsonString);
};

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
}

const indexFilePath = "/index.json.gz";
const tabNames = [
  "Exported Symbols",
  "Debug Symbols",
  "Modules",
  "Types",
  "Reconstructed Types",
];

export default function DataExplorer({ mode }: { mode: ExplorerMode }) {
  const [currentTabId, setCurrentTabId] = useState(Tab.Exports);
  const [selectedType, setSelectedType] = useState("");
  let [leftOSVersion, setLeftOSVersion] = useState("");
  let [rightOSVersion, setRightOSVersion] = useState("");
  let [binary, setBinary] = useState("");

  // Fetch index content
  const { data: indexData, error: indexError } = useSWR(
    indexFilePath,
    compressedJsonFetcher
  );

  let leftFileName = "";
  let rightFileName = "";
  if (indexData) {
    if (leftOSVersion.length == 0) {
      leftOSVersion = osVersionToPathSuffix(indexData.oses[0]);
    }
    if (rightOSVersion.length == 0) {
      rightOSVersion = osVersionToPathSuffix(indexData.oses[0]);
    }
    if (binary.length == 0) {
      binary = indexData.binaries[0];
    }

    leftFileName = `${binary}_${leftOSVersion}.json.gz`;
    if (mode == ExplorerMode.Diff) {
      rightFileName = `${binary}_${rightOSVersion}.json.gz`;
    }
  }

  let { data: leftFileData, error: leftFileError } = useSWR(
    `/${leftFileName}`,
    compressedJsonFetcher
  );
  let { data: rightFileData, error: rightFileError } = useSWR(
    `/${rightFileName}`,
    compressedJsonFetcher
  );

  if (indexError) {
    return <div>Failed to load</div>;
  }

  if (!indexData) {
    return <div>Loading...</div>;
  }

  // Prepare the appropriate data
  const compareStrings = (a: string, b: string) => (a > b ? 1 : b > a ? -1 : 0);
  const sortedOSes: string[] = indexData.oses
    .map((osVersion: any) => osVersionToPathSuffix(osVersion))
    .sort(compareStrings);
  const sortedBinaries: string[] = indexData.binaries.sort(compareStrings);
  // Data displayed on the left (in diff mode) or in the center (in browse mode)
  const leftData = (() => {
    if (!leftFileData) {
      return leftFileError ? "" : "Loading...";
    } else {
      return getEditorDataFromFileData(
        leftFileData,
        currentTabId,
        selectedType
      );
    }
  })();
  // Data displayed on the right (in diff mode)
  const rightData = (() => {
    if (!rightFileData) {
      return rightFileError ? "" : "Loading...";
    } else {
      return getEditorDataFromFileData(
        rightFileData,
        currentTabId,
        selectedType
      );
    }
  })();

  // Setup the a second combobox to select the OS version displayed on the right
  // if needed
  const rightOSCombobox = (() => {
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

  // Setup the combobox used to select types if needed
  const typesCombobox = (() => {
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
        <div className="grid grid-cols-4 gap-2">
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

function osVersionToHumanString(osVersion: any): string {
  return `${osVersion.version} ${osVersion.architecture} (${osVersion.update})`;
}

function osVersionToPathSuffix(osVersion: any): string {
  return `${osVersion.version}_${osVersion.update}_${osVersion.architecture}`;
}

function getEditorDataFromFileData(
  fileData: any,
  tab: Tab,
  selectedType: string | undefined
) {
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
}) {
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
}) {
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
