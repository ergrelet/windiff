import { Fragment } from "react";

import { Menu, Transition } from "@headlessui/react";
import { ChevronDownIcon } from "@heroicons/react/20/solid";

import DarkSwitch from "./switch";

export default function OptionsMenu({ options }: { options: any[] }) {
  return (
    <div className="text-right">
      <Menu as="div" className="z-40 relative inline-block text-left">
        <div>
          <Menu.Button className="inline-flex w-full justify-center rounded-md bg-blue-700 text-gray-50 bg-opacity-20 px-4 py-2 text-sm font-medium text-white hover:bg-opacity-30 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75">
            Options
            <ChevronDownIcon
              className="ml-2 -mr-1 h-5 w-5 text-blue-200 hover:text-blue-100"
              aria-hidden="true"
            />
          </Menu.Button>
        </div>
        <Transition
          as={Fragment}
          enter="transition ease-out duration-100"
          enterFrom="transform opacity-0 scale-95"
          enterTo="transform opacity-100 scale-100"
          leave="transition ease-in duration-75"
          leaveFrom="transform opacity-100 scale-100"
          leaveTo="transform opacity-0 scale-95"
        >
          <Menu.Items className="absolute right-0 mt-2 w-64 origin-top-right divide-y divide-gray-100 rounded-md bg-gray-800 text-gray-50 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none">
            <div className="px-3 py-2">
              {options.map((item) => (
                <Menu.Item key={item.name}>
                  <div className="group flex w-full justify-between items-center">
                    {item.name}
                    <DarkSwitch
                      checked={item.checked}
                      onChange={item.updateState}
                    />
                  </div>
                </Menu.Item>
              ))}
            </div>
          </Menu.Items>
        </Transition>
      </Menu>
    </div>
  );
}
