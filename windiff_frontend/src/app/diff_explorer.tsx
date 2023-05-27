"use client";

import { useState } from "react";
import useSWR from "swr";
import { DiffEditor } from "@monaco-editor/react";
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

enum Tab {
  Exports = 0,
  Symbols = 1,
  Modules = 2,
  TypeList = 3,
  Types = 4,
}

export default function DiffExplorer() {
  const tabs = [
    "Exported Symbols",
    "Debug Symbols",
    "Modules",
    "Types",
    "Reconstructed Types",
  ];

  const [currentTabId, setCurrentTabId] = useState(Tab.Exports);
  let [leftOSVersion, setLeftOSVersion] = useState("");
  let [rightOSVersion, setRightOSVersion] = useState("");
  let [binary, setBinary] = useState("");
  const [selectedType, setSelectedType] = useState("");

  // Fetch index content
  const { data: indexData, error: indexError } = useSWR(
    "/index.json.gz",
    compressedJsonFetcher
  );

  let leftFileName;
  let rightFileName;
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
    rightFileName = `${binary}_${rightOSVersion}.json.gz`;
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
  let leftData;
  let rightData;
  if (!leftFileData) {
    leftData = leftFileError ? "" : "Loading...";
  } else {
    switch (currentTabId) {
      default:
      case Tab.Exports:
        leftData = leftFileData.exports.join("\n");
        break;
      case Tab.Symbols:
        leftData = leftFileData.symbols.join("\n");
        break;
      case Tab.Modules:
        leftData = leftFileData.modules.join("\n");
        break;
      case Tab.TypeList:
        leftData = Object.keys(leftFileData.types).join("\n");
        break;
      case Tab.Types:
        leftData = leftFileData.types[selectedType];
        break;
    }
  }

  if (!rightFileData) {
    rightData = rightFileError ? "" : "Loading...";
  } else {
    switch (currentTabId) {
      default:
      case Tab.Exports:
        rightData = rightFileData.exports.join("\n");
        break;
      case Tab.Symbols:
        rightData = rightFileData.symbols.join("\n");
        break;
      case Tab.Modules:
        rightData = rightFileData.modules.join("\n");
        break;
      case Tab.TypeList:
        rightData = Object.keys(rightFileData.types).join("\n");
        break;
      case Tab.Types:
        rightData = rightFileData.types[selectedType];
        break;
    }
  }

  let editorLanguage = "plaintext";
  let typesCombobox;
  if (leftFileData && rightFileData) {
    const typeList = new Set(
      Object.keys(leftFileData.types).concat(Object.keys(rightFileData.types))
    );
    if (currentTabId == Tab.Types) {
      editorLanguage = "cpp";
      typesCombobox = (
        <DarkCombobox
          selectedOption={selectedType}
          options={[...typeList]}
          onChange={(value) => setSelectedType(value)}
        />
      );
    }
  }

  return (
    <div className="flex flex-row justify-center items-center">
      <div className="max-w-6xl w-full space-y-2 py-2 pl-10 pr-10">
        <DarkTabs tabs={tabs} onChange={(value) => setCurrentTabId(value)} />
        <div className="grid grid-cols-4 gap-2">
          <DarkCombobox
            selectedOption={leftOSVersion}
            options={sortedOSes}
            onChange={(value) => setLeftOSVersion(value)}
          />

          <DarkCombobox
            selectedOption={rightOSVersion}
            options={sortedOSes}
            onChange={(value) => setRightOSVersion(value)}
          />

          <DarkCombobox
            selectedOption={binary}
            options={sortedBinaries}
            onChange={(value) => setBinary(value)}
          />

          {typesCombobox}
        </div>
        <DiffView
          oldRevision={leftData}
          newRevision={rightData}
          language={editorLanguage}
        />
      </div>
    </div>
  );
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

function osVersionToHumanString(osVersion: any): string {
  return `${osVersion.version} ${osVersion.architecture} (${osVersion.update})`;
}

function osVersionToPathSuffix(osVersion: any): string {
  return `${osVersion.version}_${osVersion.update}_${osVersion.architecture}`;
}
