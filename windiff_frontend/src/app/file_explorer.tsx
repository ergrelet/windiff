"use client";

import { useState } from "react";
import useSWR from "swr";
import Editor from "@monaco-editor/react";
import pako from "pako";

import DarkTabs from "./tabs";
import DarkListbox from "./listbox";
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

export default function FileExplorer() {
  const tabs = [
    "Exported Symbols",
    "Debug Symbols",
    "Modules",
    "Types",
    "Reconstructed Types",
  ];

  const [currentTabId, setCurrentTabId] = useState(Tab.Exports);
  let [OSVersion, setOSVersion] = useState("");
  let [binary, setBinary] = useState("");
  let [selectedType, setSelectedType] = useState("");

  // Fetch index content
  const { data: indexData, error: indexError } = useSWR(
    "/index.json.gz",
    compressedJsonFetcher
  );

  let fileName;
  if (indexData) {
    if (!OSVersion) {
      OSVersion = osVersionToPathSuffix(indexData.oses[0]);
    }

    if (binary.length == 0) {
      binary = indexData.binaries[0];
    }

    fileName = `${binary}_${OSVersion}.json.gz`;
  }

  let { data: fileData, error: fileError } = useSWR(
    `/${fileName}`,
    compressedJsonFetcher
  );
  if (indexError) {
    return <div>Failed to load</div>;
  }

  if (!indexData) {
    return <div>Loading...</div>;
  }

  // Prepare the appropriate data
  let data;
  let editorLanguage = "plaintext";
  let typesCombobox;
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
      case Tab.Modules:
        data = fileData.modules.join("\n");
        break;
      case Tab.TypeList:
        data = Object.keys(fileData.types).join("\n");
        break;
      case Tab.Types:
        data = fileData.types[selectedType];
        editorLanguage = "cpp";
        typesCombobox = (
          <DarkCombobox
            selectedOption={selectedType}
            options={Object.keys(fileData.types)}
            onChange={(value) => setSelectedType(value)}
          />
        );
        break;
    }
  }

  return (
    <div className="flex flex-row justify-center items-center">
      <div className="max-w-4xl w-full space-y-2 py-2 pl-10 pr-10">
        <DarkTabs tabs={tabs} onChange={(value) => setCurrentTabId(value)} />
        <div className="grid grid-cols-3 gap-2">
          <DarkCombobox
            selectedOption={OSVersion}
            options={indexData.oses.map((osVersion: any) =>
              osVersionToPathSuffix(osVersion)
            )}
            onChange={(value) => setOSVersion(value)}
          />

          <DarkCombobox
            selectedOption={binary}
            options={indexData.binaries}
            onChange={(value) => setBinary(value)}
          />

          {typesCombobox}
        </div>
        <Editor
          height="70vh"
          theme="vs-dark"
          value={data}
          language={editorLanguage}
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
