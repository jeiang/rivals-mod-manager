import { invoke } from "@tauri-apps/api/core";
import {
  createSolidTable,
  flexRender,
  getCoreRowModel,
  getExpandedRowModel,
  type ColumnDef,
  type ExpandedState,
} from "@tanstack/solid-table";
import { For, Show, createEffect, createMemo, createSignal, onMount } from "solid-js";
import Button from "../components/Button";
import ResizableSidebar from "../components/ResizableSidebar";
import TopNav from "../components/TopNav";

type ModFileEntry = {
  id: number;
  filename: string;
  has_signatures: boolean;
  is_enabled: boolean;
};

type ModEntry = {
  id: number;
  name: string;
  author: string;
  path: string;
  category: string;
  files: ModFileEntry[];
  last_modified: number | null;
};

type ModRow = {
  rowType: "mod";
  mod: ModEntry;
  id: string;
  name: string;
  author: string;
  category: string;
  uniqueFileCount: number;
  last_modified: number | null;
  subRows: FileRow[];
};

type FileRow = {
  rowType: "file";
  modId: number;
  file: ModFileEntry;
  id: string;
  name: string;
  author: string;
  category: string;
  uniqueFileCount: number;
  last_modified: null;
  subRows: [];
};

type TableRow = ModRow | FileRow;

function getFileStem(filename: string): string {
  const dotIndex = filename.lastIndexOf(".");
  return dotIndex > 0 ? filename.slice(0, dotIndex) : filename;
}

function getUniqueFileCount(files: ModFileEntry[]): number {
  const stems = new Set<string>();
  for (const file of files) {
    const stem = getFileStem(file.filename);
    if (stem) stems.add(stem.toLowerCase());
  }
  return stems.size;
}

function formatLastModified(unixSeconds: number | null): string {
  if (unixSeconds === null) return "Unknown";
  return new Date(unixSeconds * 1000).toLocaleString();
}

function TriStateCheckbox(props: {
  checked: boolean;
  indeterminate?: boolean;
  onChange: (checked: boolean) => void;
}) {
  let ref!: HTMLInputElement;
  createEffect(() => {
    if (ref) {
      ref.indeterminate = !!props.indeterminate;
    }
  });

  return (
    <input
      ref={ref}
      type="checkbox"
      checked={props.checked}
      onChange={(event) => props.onChange(event.currentTarget.checked)}
    />
  );
}

function Main() {
  const [categories, setCategories] = createSignal<string[]>([]);
  const [mods, setMods] = createSignal<ModEntry[]>([]);
  const [isLoading, setIsLoading] = createSignal(true);
  const [isModsLoading, setIsModsLoading] = createSignal(true);
  const [isRefreshing, setIsRefreshing] = createSignal(false);
  const [isRefreshingMods, setIsRefreshingMods] = createSignal(false);
  const [expanded, setExpanded] = createSignal<ExpandedState>({});
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

  const setFileEnabled = async (fileId: number, isEnabled: boolean) => {
    try {
      await invoke("set_mod_file_enabled", { fileId, isEnabled });
      setMods((current) =>
        current.map((mod) => ({
          ...mod,
          files: mod.files.map((file) =>
            file.id === fileId ? { ...file, is_enabled: isEnabled } : file,
          ),
        })),
      );
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    }
  };

  const setModEnabled = async (modId: number, isEnabled: boolean) => {
    try {
      await invoke("set_mod_enabled", { modId, isEnabled });
      setMods((current) =>
        current.map((mod) =>
          mod.id === modId
            ? {
                ...mod,
                files: mod.files.map((file) => ({ ...file, is_enabled: isEnabled })),
              }
            : mod,
        ),
      );
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    }
  };

  const modRows = createMemo<ModRow[]>(() =>
    mods().map((mod) => ({
      rowType: "mod",
      mod,
      id: `mod-${mod.id}`,
      name: mod.name,
      author: mod.author,
      category: mod.category,
      uniqueFileCount: getUniqueFileCount(mod.files),
      last_modified: mod.last_modified,
      subRows: mod.files.map((file) => ({
        rowType: "file",
        modId: mod.id,
        file,
        id: `file-${file.id}`,
        name: file.filename,
        author: "",
        category: file.has_signatures ? "Signed" : "Unsigned",
        uniqueFileCount: 1,
        last_modified: null,
        subRows: [],
      })),
    })),
  );

  const columns: ColumnDef<TableRow>[] = [
    {
      id: "expander",
      header: "",
      cell: (info) => (
        <Show when={info.row.original.rowType === "mod"} fallback={<span class="inline-block w-4" />}>
          <button
            type="button"
            class="inline-flex w-4 items-center justify-center text-zinc-600"
            onClick={info.row.getToggleExpandedHandler()}
            aria-label={info.row.getIsExpanded() ? "Collapse files" : "Expand files"}
          >
            {info.row.getIsExpanded() ? "▾" : "▸"}
          </button>
        </Show>
      ),
    },
    {
      id: "enabled",
      header: "",
      cell: (info) => (
        <Show
          when={info.row.original.rowType === "mod"}
          fallback={
            <TriStateCheckbox
              checked={info.row.original.file.is_enabled}
              onChange={(checked) => setFileEnabled(info.row.original.file.id, checked)}
            />
          }
        >
          {() => {
            const fileRows = info.row.original.subRows;
            const enabledCount = fileRows.filter((fileRow) => fileRow.file.is_enabled).length;
            const allEnabled = fileRows.length > 0 && enabledCount === fileRows.length;
            const noneEnabled = enabledCount === 0;
            return (
              <TriStateCheckbox
                checked={allEnabled}
                indeterminate={!allEnabled && !noneEnabled}
                onChange={(checked) => setModEnabled(info.row.original.mod.id, checked)}
              />
            );
          }}
        </Show>
      ),
    },
    {
      id: "name",
      header: "Name",
      cell: (info) => (
        <div class={info.row.original.rowType === "file" ? "pl-2 text-zinc-600" : ""}>
          {info.row.original.name}
        </div>
      ),
    },
    {
      id: "author",
      header: "Author",
      cell: (info) => info.row.original.author,
    },
    {
      id: "category",
      header: "Category",
      cell: (info) => info.row.original.category,
    },
    {
      id: "files",
      header: "Files",
      cell: (info) => (info.row.original.rowType === "mod" ? info.row.original.uniqueFileCount : ""),
    },
    {
      id: "last_modified",
      header: "Last Modified",
      cell: (info) =>
        info.row.original.rowType === "mod" ? formatLastModified(info.row.original.last_modified) : "",
    },
  ];

  const table = createSolidTable({
    get data() {
      return modRows() as TableRow[];
    },
    columns,
    getCoreRowModel: getCoreRowModel(),
    getSubRows: (row) => row.subRows,
    state: {
      get expanded() {
        return expanded();
      },
    },
    onExpandedChange: setExpanded,
    getExpandedRowModel: getExpandedRowModel(),
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
                <div class="overflow-x-auto overflow-y-auto rounded border border-zinc-200">
                  <table class="w-max min-w-full border-collapse text-left text-sm">
                    <thead class="bg-zinc-50">
                      <For each={table.getHeaderGroups()}>
                        {(headerGroup) => (
                          <tr>
                            <For each={headerGroup.headers}>
                              {(header) => (
                                <th class="whitespace-nowrap border-b border-zinc-200 px-3 py-2 font-semibold text-zinc-700">
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
                                <td class="whitespace-nowrap px-3 py-2 align-top text-zinc-800">
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
