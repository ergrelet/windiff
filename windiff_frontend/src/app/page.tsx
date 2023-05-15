"use client";

import { useState } from "react";
import useSWR from "swr";
import Editor, { DiffEditor } from "@monaco-editor/react";

export default function Home() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-between p-24">
      <Index />
    </main>
  );
}

const jsonFetcher = (url: string) => fetch(url).then((res) => res.json());

function Index() {
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
    leftFileData = { exports: [] };
  }
  if (!rightFileData) {
    rightFileData = { exports: [] };
  }

  return (
    <div className="w-full">
      <h1 className="ml-3">Available Files</h1>
      <div className="grid grid-cols-2 gap-2">
        <div>
          <select
            onChange={(e) => setLeftOSVersion(e.target.value)}
            className="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
          >
            {indexData.os_versions.map((item: string, index: number) => {
              return <option key={index}>{item}</option>;
            })}
          </select>
        </div>
        <div>
          <select
            onChange={(e) => setRightOSVersion(e.target.value)}
            className="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
          >
            {indexData.os_versions.map((item: string, index: number) => {
              return <option key={index}>{item}</option>;
            })}
          </select>
        </div>

        <div>
          <select
            onChange={(e) => setLeftBinary(e.target.value)}
            className="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
          >
            {indexData.binaries.map((item: string, index: number) => {
              return <option key={index}>{item}</option>;
            })}
          </select>
        </div>
        <div>
          <select
            onChange={(e) => setRightBinary(e.target.value)}
            className="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
          >
            {indexData.binaries.map((item: string, index: number) => {
              return <option key={index}>{item}</option>;
            })}
          </select>
        </div>
      </div>
      <DiffView
        oldRevision={leftFileData.exports.join("\n")}
        newRevision={rightFileData.exports.join("\n")}
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
      height="60vh"
      theme="vs-dark"
      originalLanguage={language}
      modifiedLanguage={language}
      original={oldRevision}
      modified={newRevision}
    />
  );
}
