import { A } from "@solidjs/router";
import { JSXElement } from "solid-js";

interface TopNavProps {
  rightContent?: JSXElement;
}

function TopNav({ rightContent }: TopNavProps) {
  return (
    <header class="flex items-center justify-between border-b border-zinc-200 bg-white px-4 py-2">
      <nav class="flex items-center gap-2">
        <A
          href="/"
          end
          class="rounded px-3 py-1.5 text-sm font-medium"
          inactiveClass="text-zinc-700 hover:bg-zinc-100"
          activeClass="bg-zinc-900 text-white hover:bg-zinc-900"
        >
          Mods
        </A>
        <A
          href="/settings"
          class="rounded px-3 py-1.5 text-sm font-medium"
          inactiveClass="text-zinc-700 hover:bg-zinc-100"
          activeClass="bg-zinc-900 text-white hover:bg-zinc-900"
        >
          Settings
        </A>
        <A
          href="/categories"
          class="rounded px-3 py-1.5 text-sm font-medium"
          inactiveClass="text-zinc-700 hover:bg-zinc-100"
          activeClass="bg-zinc-900 text-white hover:bg-zinc-900"
        >
          Categories
        </A>
      </nav>
      {rightContent}
    </header>
  );
}

export default TopNav;
