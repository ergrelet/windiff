"use client";

import { useState } from "react";
import useSWR from "swr";
import Editor from "@monaco-editor/react";

import DarkTabs from "./tabs";
import DarkListbox from "./listbox";

const jsonFetcher = (url: string) => fetch(url).then((res) => res.json());

enum Tab {
  Exports = 0,
  Symbols = 1,
  Types = 2,
}

export default function FileExplorer() {
  const tabs = ["Exported Symbols", "Debug Symbols", "Debug Types"];

  const [currentTabId, setCurrentTabId] = useState(Tab.Exports);
  let [OSVersion, setOSVersion] = useState("");
  let [binary, setBinary] = useState("");

  // Fetch index content
  const { data: indexData, error: indexError } = useSWR(
    "/index.json",
    jsonFetcher
  );

  let fileName;
  if (indexData) {
    if (!OSVersion) {
      OSVersion = osVersionToPathSuffix(indexData.oses[0]);
    }

    if (binary.length == 0) {
      binary = indexData.binaries[0];
    }

    fileName = `${binary}_${OSVersion}.json`;
  }

  let { data: fileData, error: fileError } = useSWR(
    `/${fileName}`,
    jsonFetcher
  );
  if (indexError) {
    return <div>Failed to load</div>;
  }

  if (!indexData) {
    return <div>Loading...</div>;
  }

  // Prepare the appropriate data
  let data;
  if (!fileData) {
    data = fileError ? "" : "Loading...";
  } else {
    switch (currentTabId) {
      default:
      case Tab.Exports:
        data = fileData.exports.join("\n");
        break;
      case Tab.Symbols:
        data = fileData.symbols.join("\n");
        break;
      case Tab.Types:
        data = fileData.types.join("\n");
        break;
    }
  }

  return (
    <div className="flex flex-row justify-center items-center">
      <div className="max-w-4xl w-full space-y-2 py-2 pl-10 pr-10">
        <DarkTabs tabs={tabs} onChange={(value) => setCurrentTabId(value)} />
        <div className="grid grid-cols-2 gap-2">
          <DarkListbox
            value={OSVersion}
            options={indexData.oses.map((osVersion: any) =>
              osVersionToPathSuffix(osVersion)
            )}
            onChange={(value) => setOSVersion(value)}
          />

          <DarkListbox
            value={binary}
            options={indexData.binaries}
            onChange={(value) => setBinary(value)}
          />
        </div>
        <Editor height="70vh" theme="vs-dark" value={data} language="text" />
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
