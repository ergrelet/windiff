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
  Modules = 2,
  Types = 3,
}

export default function DiffExplorer() {
  const tabs = ["Exported Symbols", "Debug Symbols", "Modules", "Debug Types"];

  const [currentTabId, setCurrentTabId] = useState(Tab.Exports);
  let [leftOSVersion, setLeftOSVersion] = useState("");
  let [rightOSVersion, setRightOSVersion] = useState("");
  let [binary, setBinary] = useState("");

  // Fetch index content
  const { data: indexData, error: indexError } = useSWR(
    "/index.json",
    jsonFetcher
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

    leftFileName = `${binary}_${leftOSVersion}.json`;
    rightFileName = `${binary}_${rightOSVersion}.json`;
  }

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

  // Prepare the appropriate data
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
      case Tab.Types:
        leftData = leftFileData.types.join("\n");
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
      case Tab.Types:
        rightData = rightFileData.types.join("\n");
        break;
    }
  }

  return (
    <div className="flex flex-row justify-center items-center">
      <div className="max-w-6xl w-full space-y-2 py-2 pl-10 pr-10">
        <DarkTabs tabs={tabs} onChange={(value) => setCurrentTabId(value)} />
        <div className="grid grid-cols-3 gap-2">
          <DarkListbox
            value={leftOSVersion}
            options={indexData.oses.map((osVersion: any) =>
              osVersionToPathSuffix(osVersion)
            )}
            onChange={(value) => setLeftOSVersion(value)}
          />

          <DarkListbox
            value={rightOSVersion}
            options={indexData.oses.map((osVersion: any) =>
              osVersionToPathSuffix(osVersion)
            )}
            onChange={(value) => setRightOSVersion(value)}
          />

          <DarkListbox
            value={binary}
            options={indexData.binaries}
            onChange={(value) => setBinary(value)}
          />
        </div>
        <DiffView
          oldRevision={leftData}
          newRevision={rightData}
          language="text"
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
