import { Tab } from "@headlessui/react";
import type { JSX } from "react";

export default function DarkTabs({
  tabs,
  selectedIndex,
  onChange,
}: {
  tabs: string[];
  selectedIndex?: number;
  onChange?(value: any): void;
}): JSX.Element {
  return (
    <div className="w-full">
      <Tab.Group selectedIndex={selectedIndex} onChange={onChange}>
        <Tab.List className="flex space-x-1 rounded-xl bg-blue-900/20 p-1">
          {tabs.map((tab, id) => (
            <Tab
              key={id}
              className={({ selected }) =>
                classNames(
                  "w-full rounded-lg py-2.5 text-sm font-medium leading-5 text-blue-700",
                  "ring-white/60 ring-offset-2 ring-offset-blue-400 focus:outline-none focus:ring-2",
                  selected
                    ? "bg-white shadow"
                    : "text-blue-100 hover:bg-white/[0.12] hover:text-white"
                )
              }
            >
              {tab}
            </Tab>
          ))}
        </Tab.List>
      </Tab.Group>
    </div>
  );
}

function classNames(...classes: any[]): string {
  return classes.filter(Boolean).join(" ");
}
