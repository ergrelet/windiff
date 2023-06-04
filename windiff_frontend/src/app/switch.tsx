import { Switch } from "@headlessui/react";

export default function DarkSwitch({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange?(checked: boolean): void;
}) {
  return (
    <Switch.Group>
      <div className="flex items-center">
        <Switch
          checked={checked}
          onChange={onChange}
          className={`${checked ? "bg-blue-900" : "bg-gray-700"}
              relative inline-flex h-[20px] w-[37px] shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2  focus-visible:ring-white focus-visible:ring-opacity-75`}
        >
          <span
            aria-hidden="true"
            className={`${
              checked
                ? "translate-x-[18px] translate-y-[0.5px]"
                : "translate-x-0 translate-y-[0.5px]"
            }
                pointer-events-none inline-block h-[14px] w-[14px] transform rounded-full bg-white shadow-lg ring-0 transition duration-200 ease-in-out`}
          />
        </Switch>
      </div>
    </Switch.Group>
  );
}
