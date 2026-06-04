"use client";

import { useState, useEffect } from "react";

import TopNavBar from "./navbar";
import DataExplorer, { ExplorerMode } from "./data_explorer";
import {
  readParam,
  writeParams,
  PARAM_MODE,
  MODE_BROWSE,
  MODE_DIFF,
} from "./permalink";

enum NavigationButton {
  Browsing = 0,
  Diffing = 1,
}

export default function Home() {
  const [currentNavigationButton, setCurrentNavigationButton] = useState(
    NavigationButton.Browsing
  );

  // Hydrate mode from URL after mount (avoids SSR hydration mismatch)
  useEffect(() => {
    if (readParam(PARAM_MODE) === MODE_DIFF) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- intentional: defers URL-derived state to post-mount to avoid hydration mismatch
      setCurrentNavigationButton(NavigationButton.Diffing);
    }
  }, []);

  const setMode = (button: NavigationButton) => {
    setCurrentNavigationButton(button);
    writeParams({
      [PARAM_MODE]:
        button === NavigationButton.Diffing ? MODE_DIFF : MODE_BROWSE,
    });
  };

  const navigationButtons = [
    {
      name: "Browse Files",
      onClick: () => setMode(NavigationButton.Browsing),
      current: currentNavigationButton == NavigationButton.Browsing,
    },
    {
      name: "Diff Files",
      onClick: () => setMode(NavigationButton.Diffing),
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
