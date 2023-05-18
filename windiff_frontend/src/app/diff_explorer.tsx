"use client";

import { useState } from "react";
import useSWR from "swr";
import { DiffEditor } from "@monaco-editor/react";

import DarkTabs from "./tabs";
import DarkListbox from "./listbox";

const jsonFetcher = (url: string) => fetch(url).then((res) => res.json());

enum Tab {
  Exports = 0,
  Symbols = 1,
  Types = 2,
}

export default function DiffExplorer() {
  const tabs = ["Exported Symbols", "Debug Symbols", "Debug Types"];

  const [currentTabId, setCurrentTabId] = useState(Tab.Exports);
  let [leftOSVersion, setLeftOSVersion] = useState("");
  let [rightOSVersion, setRightOSVersion] = useState("");
  let [leftBinary, setLeftBinary] = useState("");
  let [rightBinary, setRightBinary] = useState("");

  // Fetch index content
  const { data: indexData, error: indexError } = useSWR(
    "/index.json",
    jsonFetcher
  );

  const architecture = "amd64";
  if (indexData) {
    if (leftOSVersion.length == 0) {
      leftOSVersion = indexData.os_versions[0];
    }
    if (leftBinary.length == 0) {
      leftBinary = indexData.binaries[0];
    }

    if (rightOSVersion.length == 0) {
      rightOSVersion = indexData.os_versions[0];
    }
    if (rightBinary.length == 0) {
      rightBinary = indexData.binaries[0];
    }
  }
  const leftFileName = `${leftBinary}_${leftOSVersion}_${architecture}.json`;
  const rightFileName = `${rightBinary}_${rightOSVersion}_${architecture}.json`;

  let { data: leftFileData, error: leftFileError } = useSWR(
    `/${leftFileName}`,
    jsonFetcher
  );
  let { data: rightFileData, error: rightFileError } = useSWR(
    `/${rightFileName}`,
    jsonFetcher
  );

  if (indexError) {
    return <div>Failed to load</div>;
  }

  if (!indexData) {
    return <div>Loading...</div>;
  }

  if (!leftFileData) {
    leftFileData = { exports: [], symbols: [], types: [] };
  }
  if (!rightFileData) {
    rightFileData = { exports: [], symbols: [], types: [] };
  }

  // Prepare the appropriate data
  let leftData;
  let rightData;
  switch (currentTabId) {
    case Tab.Exports:
      leftData = leftFileData.exports.join("\n");
      rightData = rightFileData.exports.join("\n");
      break;
    case Tab.Symbols:
      leftData = leftFileData.symbols.join("\n");
      rightData = rightFileData.symbols.join("\n");
      break;
    // Types
    case Tab.Types:
      leftData = leftFileData.types.join("\n");
      rightData = rightFileData.types.join("\n");
      break;
    default:
      break;
  }

  return (
    <div className="w-full space-y-2 py-2 pl-10 pr-10">
      <DarkTabs tabs={tabs} onChange={(value) => setCurrentTabId(value)} />
      <div className="grid grid-cols-2 gap-2">
        <DarkListbox
          value={leftOSVersion}
          options={indexData.os_versions}
          onChange={(value) => setLeftOSVersion(value)}
        />

        <DarkListbox
          value={rightOSVersion}
          options={indexData.os_versions}
          onChange={(value) => setRightOSVersion(value)}
        />

        <DarkListbox
          value={leftBinary}
          options={indexData.binaries}
          onChange={(value) => setLeftBinary(value)}
        />

        <DarkListbox
          value={rightBinary}
          options={indexData.binaries}
          onChange={(value) => setRightBinary(value)}
        />
      </div>
      <DiffView
        oldRevision={leftData}
        newRevision={rightData}
        language="text"
      />
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
      height="70vh"
      theme="vs-dark"
      originalLanguage={language}
      modifiedLanguage={language}
      original={oldRevision}
      modified={newRevision}
    />
  );
}
