"use client";

import { useState } from "react";

import TopNavBar from "./navbar";
import DataExplorer, { ExplorerMode } from "./data_explorer";

enum NavigationButton {
  Browsing = 0,
  Diffing = 1,
}

export default function Home() {
  const [currentNavigationButton, setCurrentNavigationButton] = useState(
    NavigationButton.Browsing
  );
  const navigationButtons = [
    {
      name: "Browse Files",
      onClick: () => setCurrentNavigationButton(NavigationButton.Browsing),
      current: currentNavigationButton == NavigationButton.Browsing,
    },
    {
      name: "Diff Files",
      onClick: () => setCurrentNavigationButton(NavigationButton.Diffing),
      current: currentNavigationButton == NavigationButton.Diffing,
    },
  ];

  // Select data explorer mode
  let explorerMode;
  switch (currentNavigationButton) {
    default:
    case NavigationButton.Browsing:
      explorerMode = ExplorerMode.Browse;
      break;
    case NavigationButton.Diffing:
      explorerMode = ExplorerMode.Diff;
      break;
  }

  return (
    <main>
      <TopNavBar buttons={navigationButtons} />
      <DataExplorer mode={explorerMode} />
    </main>
  );
}
