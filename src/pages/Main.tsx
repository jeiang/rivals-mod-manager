import { invoke } from "@tauri-apps/api/core";
import {
  createSolidTable,
  flexRender,
  getCoreRowModel,
  type ColumnDef,
} from "@tanstack/solid-table";
import { For, Show, createMemo, createSignal, onMount } from "solid-js";
import Button from "../components/Button";
import ResizableSidebar from "../components/ResizableSidebar";
import TopNav from "../components/TopNav";

type ModEntry = {
  id: number;
  name: string;
  path: string;
  category: string;
  files: string[];
  last_modified: number | null;
};

type ModRow = ModEntry & {
  uniqueFileCount: number;
};

function getFileStem(filePath: string): string {
  const filename = filePath.split(/[\\/]/).pop() ?? "";
  const dotIndex = filename.lastIndexOf(".");
  return dotIndex > 0 ? filename.slice(0, dotIndex) : filename;
}

function getUniqueFileCount(files: string[]): number {
  const stems = new Set<string>();
  for (const file of files) {
    const stem = getFileStem(file);
    if (stem) stems.add(stem.toLowerCase());
  }
  return stems.size;
}

function formatLastModified(unixSeconds: number | null): string {
  if (unixSeconds === null) return "Unknown";
  return new Date(unixSeconds * 1000).toLocaleString();
}

function Main() {
  const [categories, setCategories] = createSignal<string[]>([]);
  const [mods, setMods] = createSignal<ModEntry[]>([]);
  const [selectedModIds, setSelectedModIds] = createSignal<Set<number>>(new Set());
  const [isLoading, setIsLoading] = createSignal(true);
  const [isModsLoading, setIsModsLoading] = createSignal(true);
  const [isRefreshing, setIsRefreshing] = createSignal(false);
  const [isRefreshingMods, setIsRefreshingMods] = createSignal(false);
  const [categoryError, setCategoryError] = createSignal<string | null>(null);
  const [toastMessage, setToastMessage] = createSignal<string | null>(null);

  const showToast = (message: string) => {
    setToastMessage(message);
    window.setTimeout(() => {
      setToastMessage(null);
    }, 6000);
  };

  const loadCategories = async () => {
    try {
      setCategoryError(null);
      const results = await invoke<string[]>("get_categories");
      setCategories(results);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setCategoryError(message);
    }
  };

  const loadMods = async () => {
    try {
      const results = await invoke<ModEntry[]>("get_mods");
      setMods(results);
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    }
  };

  onMount(async () => {
    try {
      await Promise.all([loadCategories(), loadMods()]);
    } finally {
      setIsLoading(false);
      setIsModsLoading(false);
    }
  });

  const refreshCategories = async () => {
    try {
      setIsRefreshing(true);
      setCategoryError(null);
      await invoke<string[]>("refresh_categories");
      await loadCategories();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setCategoryError(message);
      showToast(
        "MarvelRivalsAPI token must be set in Settings. A token can be gotten from https://marvelrivalsapi.com/",
      );
    } finally {
      setIsRefreshing(false);
    }
  };

  const refreshMods = async () => {
    try {
      setIsRefreshingMods(true);
      await invoke("refresh_mods");
      await loadMods();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      setIsRefreshingMods(false);
    }
  };

  const toggleMod = (modId: number, checked: boolean) => {
    setSelectedModIds((current) => {
      const next = new Set(current);
      if (checked) next.add(modId);
      else next.delete(modId);
      return next;
    });
  };

  const modRows = createMemo<ModRow[]>(() =>
    mods().map((mod) => ({
      ...mod,
      uniqueFileCount: getUniqueFileCount(mod.files),
    })),
  );

  const columns: ColumnDef<ModRow>[] = [
    {
      id: "selected",
      header: "",
      cell: (info) => (
        <input
          type="checkbox"
          checked={selectedModIds().has(info.row.original.id)}
          onChange={(event) => toggleMod(info.row.original.id, event.currentTarget.checked)}
        />
      ),
    },
    {
      accessorKey: "name",
      header: "Name",
    },
    {
      accessorKey: "category",
      header: "Category",
    },
    {
      accessorKey: "uniqueFileCount",
      header: "Files",
    },
    {
      accessorKey: "last_modified",
      header: "Last Modified",
      cell: (info) => formatLastModified(info.row.original.last_modified),
    },
  ];

  const table = createSolidTable({
    get data() {
      return modRows();
    },
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  return (
    <div class="flex h-full min-h-0 flex-col">
      <Show when={toastMessage()}>
        {(message) => (
          <div class="fixed right-4 top-4 z-50 max-w-md rounded border border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-900 shadow">
            {message()}
          </div>
        )}
      </Show>
      <TopNav
        rightContent={
          <div class="flex items-center gap-2">
            <Button disabled={isRefreshingMods()} onClick={() => refreshMods()}>
              {isRefreshingMods() ? "Refreshing mods..." : "Refresh mods"}
            </Button>
            <Button disabled={isRefreshing()} onClick={() => refreshCategories()}>
              {isRefreshing() ? "Refreshing..." : "Refresh categories"}
            </Button>
          </div>
        }
      />
      <div class="min-h-0 flex-1">
        <ResizableSidebar
          sidebar={
            <div class="space-y-3">
              <h2 class="text-lg font-semibold">Categories</h2>
              <Show when={isLoading()}>
                <p class="text-sm text-zinc-500">Loading categories...</p>
              </Show>
              <Show when={categoryError()}>
                {(message) => <p class="text-sm text-red-500">{message()}</p>}
              </Show>
              <Show when={!isLoading() && !categoryError()}>
                <ul class="space-y-1 text-sm">
                  <li class="rounded px-2 py-1 hover:bg-zinc-100">All</li>
                  <For each={categories()}>
                    {(category) => <li class="rounded px-2 py-1 hover:bg-zinc-100">{category}</li>}
                  </For>
                </ul>
              </Show>
            </div>
          }
          content={
            <div class="space-y-3">
              <h2 class="text-lg font-semibold">Mods</h2>
              <Show when={isModsLoading()}>
                <p class="text-sm text-zinc-500">Loading mods...</p>
              </Show>
              <Show when={!isModsLoading()}>
                <div class="overflow-auto rounded border border-zinc-200">
                  <table class="min-w-full border-collapse text-left text-sm">
                    <thead class="bg-zinc-50">
                      <For each={table.getHeaderGroups()}>
                        {(headerGroup) => (
                          <tr>
                            <For each={headerGroup.headers}>
                              {(header) => (
                                <th class="border-b border-zinc-200 px-3 py-2 font-semibold text-zinc-700">
                                  {header.isPlaceholder
                                    ? null
                                    : flexRender(
                                        header.column.columnDef.header,
                                        header.getContext(),
                                      )}
                                </th>
                              )}
                            </For>
                          </tr>
                        )}
                      </For>
                    </thead>
                    <tbody>
                      <For each={table.getRowModel().rows}>
                        {(row) => (
                          <tr class="border-b border-zinc-100 last:border-b-0">
                            <For each={row.getVisibleCells()}>
                              {(cell) => (
                                <td class="px-3 py-2 align-top text-zinc-800">
                                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                                </td>
                              )}
                            </For>
                          </tr>
                        )}
                      </For>
                    </tbody>
                  </table>
                </div>
              </Show>
            </div>
          }
        />
      </div>
    </div>
  );
}

export default Main;
