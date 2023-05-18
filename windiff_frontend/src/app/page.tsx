"use client";

import { useState } from "react";

import TopNavBar from "./navbar";
import DiffExplorer from "./diff_explorer";

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

  // Select main component to display
  let mainComponent;
  switch (currentNavigationButton) {
    default:
    case NavigationButton.Browsing:
      // TODO
      break;
    case NavigationButton.Diffing:
      mainComponent = <DiffExplorer />;
      break;
  }

  return (
    <main>
      <TopNavBar buttons={navigationButtons} />
      {mainComponent}
    </main>
  );
}
