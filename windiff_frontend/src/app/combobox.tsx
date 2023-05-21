"use client";

import { Fragment, useState } from "react";
import { Transition, Combobox } from "@headlessui/react";
import { ChevronUpDownIcon } from "@heroicons/react/20/solid";

export default function DarkCombobox({
  selectedOption,
  options,
  onChange,
}: {
  selectedOption: string;
  options: string[];
  onChange?(value: any): void;
}) {
  const [query, setQuery] = useState("");

  const filteredOptions =
    query === ""
      ? options
      : options.filter((option) => {
          return option.toLowerCase().includes(query.toLowerCase());
        });

  return (
    <Combobox value={selectedOption} onChange={onChange}>
      <div className="relative mt-1">
        <Combobox.Input
          onChange={(event) => setQuery(event.target.value)}
          className="relative w-full cursor-default rounded-lg bg-blue-900 text-gray-50 py-2 pl-3 pr-10 text-left shadow-md focus:outline-none focus-visible:border-indigo-500 focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75 focus-visible:ring-offset-2 focus-visible:ring-offset-orange-300 sm:text-sm"
        />
        <Combobox.Button className="absolute inset-y-0 right-0 flex items-center pr-2">
          <ChevronUpDownIcon
            className="h-5 w-5 text-gray-400"
            aria-hidden="true"
          />
        </Combobox.Button>
        <Transition
          as={Fragment}
          enter="transition-opacity duration-50"
          enterFrom="opacity-0"
          enterTo="opacity-100"
          leave="transition ease-in duration-100"
          leaveFrom="opacity-100"
          leaveTo="opacity-0"
        >
          <Combobox.Options className="z-20 absolute mt-1 max-h-60 w-full overflow-auto rounded-md bg-gray-800 text-gray-50 py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm">
            {filteredOptions.map((option, id) => (
              <Combobox.Option
                key={id}
                className={({ active }) =>
                  `relative cursor-default select-none py-2 pl-4 pr-4 ${
                    active ? "bg-gray-900 text-gray-300" : "text-gray-50"
                  }`
                }
                value={option}
              >
                {option}
              </Combobox.Option>
            ))}
          </Combobox.Options>
        </Transition>
      </div>
    </Combobox>
  );
}
