import { Dialog, Transition } from "@headlessui/react";
import { Fragment } from "react";
import { ArrowTopRightOnSquareIcon } from "@heroicons/react/20/solid";

export default function AboutModal({
  open,
  onClose,
}: {
  open?: boolean;
  onClose(value: boolean): void;
}) {
  const title = "About";
  const content = (
    <div className="mt-2">
      <p className="text-sm text-gray-500">
        WinDiff is an open-source web-based tool which allows browsing and
        comparing symbol and type information of Microsoft Windows binaries
        across different versions of the OS.
        <br />
        <br />
        It was inspired by{" "}
        <Hyperlink
          name="ntdiff"
          target="https://github.com/ntdiff/ntdiff"
        />{" "}
        and made possible with the help of{" "}
        <Hyperlink
          name="Winbindex"
          target="https://github.com/m417z/winbindex"
        />
        .<br />
        <br />
        Github repository:{" "}
        <Hyperlink
          name="https://github.com/ergrelet/windiff"
          target="https://github.com/ergrelet/windiff"
        />
      </p>
    </div>
  );

  return (
    <>
      <Transition appear show={open} as={Fragment}>
        <Dialog as="div" className="relative z-20" onClose={onClose}>
          <Transition.Child
            as={Fragment}
            enter="ease-out duration-300"
            enterFrom="opacity-0"
            enterTo="opacity-100"
            leave="ease-in duration-200"
            leaveFrom="opacity-100"
            leaveTo="opacity-0"
          >
            <div className="fixed inset-0 bg-black bg-opacity-25" />
          </Transition.Child>

          <div className="fixed inset-0 overflow-y-auto">
            <div className="flex min-h-full items-center justify-center p-4 text-center">
              <Transition.Child
                as={Fragment}
                enter="ease-out duration-300"
                enterFrom="opacity-0 scale-95"
                enterTo="opacity-100 scale-100"
                leave="ease-in duration-200"
                leaveFrom="opacity-100 scale-100"
                leaveTo="opacity-0 scale-95"
              >
                <Dialog.Panel className="w-full max-w-md transform overflow-hidden rounded-2xl bg-white p-6 text-left align-middle shadow-xl transition-all">
                  <Dialog.Title
                    as="h3"
                    className="text-lg font-medium leading-6 text-gray-900"
                  >
                    {title}
                  </Dialog.Title>
                  {content}
                </Dialog.Panel>
              </Transition.Child>
            </div>
          </div>
        </Dialog>
      </Transition>
    </>
  );
}

function Hyperlink({ name, target }: { name: string; target: string }) {
  return (
    <div className="inline-flex items-baseline">
      <a href={target} target="_blank" className="text-blue-900">
        {name}
      </a>
      <ArrowTopRightOnSquareIcon className="h-3 w-3" aria-hidden="true" />
    </div>
  );
}
