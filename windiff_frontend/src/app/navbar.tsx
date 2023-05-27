"use client";

import { useState } from "react";
import { Disclosure } from "@headlessui/react";
import {
  XMarkIcon,
  Bars3Icon,
  QuestionMarkCircleIcon,
} from "@heroicons/react/24/solid";
import Image from "next/image";

import AboutModal from "./about";
import Link from "next/link";

export default function TopNavBar({
  buttons,
}: {
  buttons: any[];
}): JSX.Element {
  const [isAboutModalOpen, setIsAboutModalOpen] = useState(false);

  return (
    <>
      <AboutModal
        open={isAboutModalOpen}
        onClose={() => setIsAboutModalOpen(false)}
      />
      <Disclosure as="nav" className="bg-gray-800">
        {({ open }) => (
          <>
            <div className="mx-auto max-w-7xl px-2 sm:px-6 lg:px-8">
              <div className="relative flex h-16 items-center justify-between">
                <div className="absolute inset-y-0 left-0 flex items-center sm:hidden">
                  {/* Mobile menu button*/}
                  <Disclosure.Button className="inline-flex items-center justify-center rounded-md p-2 text-gray-400 hover:bg-gray-700 hover:text-white focus:outline-none focus:ring-2 focus:ring-inset focus:ring-white">
                    <span className="sr-only">Open main menu</span>
                    {open ? (
                      <XMarkIcon className="block h-6 w-6" aria-hidden="true" />
                    ) : (
                      <Bars3Icon className="block h-6 w-6" aria-hidden="true" />
                    )}
                  </Disclosure.Button>
                </div>
                <div className="flex flex-1 items-center justify-center sm:items-stretch sm:justify-start">
                  <div className="flex flex-shrink-0 items-center">
                    <Link href="/">
                      <Image
                        className="block h-8 w-auto lg:hidden"
                        src="/WinDiffLogoWhite.png"
                        width={200}
                        height={100}
                        alt="WinDiff Logo"
                      />
                      <Image
                        className="hidden h-8 w-auto lg:block"
                        src="/WinDiffLogoWhite.png"
                        width={200}
                        height={100}
                        alt="WinDiff Logo"
                      />
                    </Link>
                  </div>
                  <div className="hidden sm:ml-6 sm:block">
                    <div className="flex space-x-4">
                      {buttons.map((item) => (
                        <a
                          key={item.name}
                          href="#"
                          onClick={item.onClick}
                          className={classNames(
                            item.current
                              ? "bg-gray-900 text-white"
                              : "text-gray-300 hover:bg-gray-700 hover:text-white",
                            "rounded-md px-3 py-2 text-sm font-medium"
                          )}
                          aria-current={item.current ? "page" : undefined}
                        >
                          {item.name}
                        </a>
                      ))}
                    </div>
                  </div>
                </div>
                <div className="absolute inset-y-0 right-0 flex items-center pr-2 sm:static sm:inset-auto sm:ml-6 sm:pr-0">
                  <button
                    type="button"
                    className="rounded-full bg-gray-800 p-1 text-gray-400 hover:text-white focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-gray-800"
                    onClick={() => setIsAboutModalOpen(true)}
                  >
                    <span className="sr-only">About</span>
                    <QuestionMarkCircleIcon
                      className="h-6 w-6"
                      aria-hidden="true"
                    />
                  </button>
                </div>
              </div>
            </div>

            <Disclosure.Panel className="sm:hidden">
              <div className="space-y-1 px-2 pb-3 pt-2">
                {buttons.map((item) => (
                  <Disclosure.Button
                    key={item.name}
                    as="a"
                    href={item.href}
                    className={classNames(
                      item.current
                        ? "bg-gray-900 text-white"
                        : "text-gray-300 hover:bg-gray-700 hover:text-white",
                      "block rounded-md px-3 py-2 text-base font-medium"
                    )}
                    aria-current={item.current ? "page" : undefined}
                  >
                    {item.name}
                  </Disclosure.Button>
                ))}
              </div>
            </Disclosure.Panel>
          </>
        )}
      </Disclosure>
    </>
  );
}

function classNames(...classes: any[]): string {
  return classes.filter(Boolean).join(" ");
}
