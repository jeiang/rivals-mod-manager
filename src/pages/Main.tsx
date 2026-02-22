import { invoke } from "@tauri-apps/api/core";
import {
  createSolidTable,
  flexRender,
  getCoreRowModel,
  getExpandedRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  type ColumnDef,
  type ExpandedState,
  type SortingState,
} from "@tanstack/solid-table";
import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";
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
  nexus_mod_id: number | null;
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
  nexusModId: number | null;
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
  nexusModId: null;
  category: string;
  uniqueFileCount: number;
  last_modified: null;
  subRows: [];
};

type TableRow = ModRow | FileRow;

function isModRow(row: TableRow): row is ModRow {
  return row.rowType === "mod";
}

function isFileRow(row: TableRow): row is FileRow {
  return row.rowType === "file";
}

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
  let ref: HTMLInputElement | undefined;
  createEffect(() => {
    if (ref) {
      ref.indeterminate = !!props.indeterminate;
    }
  });

  return (
    <input
      ref={(element) => {
        ref = element;
      }}
      type="checkbox"
      checked={props.checked}
      onClick={(event) => event.stopPropagation()}
      onChange={(event) => props.onChange(event.currentTarget.checked)}
    />
  );
}

function Main() {
  const [categories, setCategories] = createSignal<string[]>([]);
  const [mods, setMods] = createSignal<ModEntry[]>([]);
  const [selectedCategory, setSelectedCategory] = createSignal<string>("All");
  const [isLoading, setIsLoading] = createSignal(true);
  const [isModsLoading, setIsModsLoading] = createSignal(true);
  const [isRefreshing, setIsRefreshing] = createSignal(false);
  const [isRefreshingMods, setIsRefreshingMods] = createSignal(false);
  const [isApplyingMods, setIsApplyingMods] = createSignal(false);
  const [isClearingMods, setIsClearingMods] = createSignal(false);
  const [expanded, setExpanded] = createSignal<ExpandedState>({});
  const [sorting, setSorting] = createSignal<SortingState>([]);
  const [globalFilter, setGlobalFilter] = createSignal("");
  const [selectedModIds, setSelectedModIds] = createSignal<Set<number>>(new Set());
  const [lastSelectedModId, setLastSelectedModId] = createSignal<number | null>(null);
  const [contextMenu, setContextMenu] = createSignal<{
    x: number;
    y: number;
    modId: number;
  } | null>(null);
  const [categoryModal, setCategoryModal] = createSignal<{
    modIds: number[];
    category: string;
  } | null>(null);
  const [renameModal, setRenameModal] = createSignal<{
    modId: number;
    name: string;
  } | null>(null);
  const [authorModal, setAuthorModal] = createSignal<{
    modIds: number[];
    author: string;
  } | null>(null);
  const [modIdModal, setModIdModal] = createSignal<{
    modId: number;
    nexusModId: string;
  } | null>(null);
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

  const applyMods = async () => {
    try {
      setIsApplyingMods(true);
      await invoke("apply_mods");
      showToast("All mods are added");
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      setIsApplyingMods(false);
    }
  };

  const clearModsOutput = async () => {
    try {
      setIsClearingMods(true);
      await invoke("clear_mods_output");
      showToast("Mods output folder is cleared");
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      setIsClearingMods(false);
    }
  };

  const setModsCategory = async (modIds: number[], category: string) => {
    try {
      await invoke("set_mods_category", { modIds, category });
      const selected = new Set(modIds);
      setMods((current) =>
        current.map((mod) => (selected.has(mod.id) ? { ...mod, category } : mod)),
      );
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    } finally {
      setCategoryModal(null);
    }
  };

  const resetModsCategoryToAuto = async (modIds: number[]) => {
    try {
      await invoke("reset_mods_category_to_auto", { modIds });
      await loadMods();
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    }
  };

  const renameMod = async (modId: number, name: string) => {
    const trimmed = name.trim();
    if (!trimmed) {
      showToast("Mod name cannot be empty");
      return;
    }

    try {
      await invoke("set_mod_name", { modId, name: trimmed });
      setMods((current) =>
        current.map((mod) => (mod.id === modId ? { ...mod, name: trimmed } : mod)),
      );
      setRenameModal(null);
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    }
  };

  const setModsAuthor = async (modIds: number[], author: string) => {
    const trimmed = author.trim();
    if (!trimmed) {
      showToast("Author cannot be empty");
      return;
    }

    try {
      await invoke("set_mods_author", { modIds, author: trimmed });
      const selected = new Set(modIds);
      setMods((current) =>
        current.map((mod) => (selected.has(mod.id) ? { ...mod, author: trimmed } : mod)),
      );
      setAuthorModal(null);
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    }
  };

  const setModNexusId = async (modId: number, rawValue: string) => {
    const trimmed = rawValue.trim();
    let nexus_mod_id: number | null = null;
    if (trimmed !== "") {
      const parsed = Number(trimmed);
      if (!Number.isInteger(parsed) || parsed < 0) {
        showToast("Mod ID must be a non-negative whole number");
        return;
      }
      nexus_mod_id = parsed;
    }

    try {
      await invoke("set_mod_nexus_id", { modId, nexusModId: nexus_mod_id });
      setMods((current) =>
        current.map((mod) => (mod.id === modId ? { ...mod, nexus_mod_id } : mod)),
      );
      setModIdModal(null);
    } catch (err) {
      showToast(err instanceof Error ? err.message : String(err));
    }
  };

  const selectModRow = (modId: number, isMultiSelect: boolean, isRangeSelect: boolean) => {
    const orderedModIds = table
      .getRowModel()
      .rows.map((row) => row.original)
      .filter(isModRow)
      .map((row) => row.mod.id);

    setSelectedModIds((current) => {
      if (isRangeSelect && lastSelectedModId() !== null) {
        const anchorId = lastSelectedModId()!;
        const startIndex = orderedModIds.indexOf(anchorId);
        const endIndex = orderedModIds.indexOf(modId);

        if (startIndex !== -1 && endIndex !== -1) {
          const [from, to] =
            startIndex <= endIndex ? [startIndex, endIndex] : [endIndex, startIndex];
          const rangeIds = orderedModIds.slice(from, to + 1);
          if (isMultiSelect) {
            const next = new Set(current);
            for (const id of rangeIds) next.add(id);
            return next;
          }
          return new Set(rangeIds);
        }
      }

      if (!isMultiSelect) {
        return new Set([modId]);
      }

      const next = new Set(current);
      if (next.has(modId)) {
        next.delete(modId);
      } else {
        next.add(modId);
      }
      return next;
    });
    setLastSelectedModId(modId);
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
    mods()
      .filter((mod) => selectedCategory() === "All" || mod.category === selectedCategory())
      .map((mod) => ({
        rowType: "mod",
        mod,
        id: `mod-${mod.id}`,
        name: mod.name,
        author: mod.author,
        nexusModId: mod.nexus_mod_id,
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
          nexusModId: null,
          category: file.has_signatures ? "Signed" : "Unsigned",
          uniqueFileCount: 1,
          last_modified: null,
          subRows: [],
        })),
      })),
  );

  const authorSuggestions = createMemo<string[]>(() => {
    const unique = new Set<string>();
    for (const mod of mods()) {
      const author = mod.author.trim();
      if (author) unique.add(author);
    }
    return Array.from(unique).sort((a, b) => a.localeCompare(b));
  });

  const columns: ColumnDef<TableRow>[] = [
    {
      id: "expander",
      header: "",
      enableSorting: false,
      cell: (info) =>
        isModRow(info.row.original) ? (
          <button
            type="button"
            class="inline-flex w-4 items-center justify-center text-zinc-600"
            onClick={(event) => {
              event.stopPropagation();
              info.row.toggleExpanded();
            }}
            aria-label={info.row.getIsExpanded() ? "Collapse files" : "Expand files"}
          >
            {info.row.getIsExpanded() ? "▾" : "▸"}
          </button>
        ) : (
          <span class="inline-block w-4" />
        ),
    },
    {
      id: "enabled",
      header: "",
      enableSorting: false,
      cell: (info) => {
        const row = info.row.original;
        if (isFileRow(row)) {
          return (
            <TriStateCheckbox
              checked={row.file.is_enabled}
              onChange={(checked) => setFileEnabled(row.file.id, checked)}
            />
          );
        }

        const fileRows = row.subRows;
        const enabledCount = fileRows.filter((fileRow) => fileRow.file.is_enabled).length;
        const allEnabled = fileRows.length > 0 && enabledCount === fileRows.length;
        const noneEnabled = enabledCount === 0;
        return (
          <TriStateCheckbox
            checked={allEnabled}
            indeterminate={!allEnabled && !noneEnabled}
            onChange={(checked) => setModEnabled(row.mod.id, checked)}
          />
        );
      },
    },
    {
      id: "name",
      header: "Name",
      accessorFn: (row) => row.name,
      cell: (info) => (
        <div class={info.row.original.rowType === "file" ? "pl-2 text-zinc-600" : ""}>
          {info.row.original.name}
        </div>
      ),
    },
    {
      id: "author",
      header: "Author",
      accessorFn: (row) => row.author,
      cell: (info) => info.row.original.author,
    },
    {
      id: "mod_id",
      header: "Mod ID",
      accessorFn: (row) => row.nexusModId ?? 0,
      cell: (info) =>
        info.row.original.rowType === "mod" && info.row.original.nexusModId
          ? info.row.original.nexusModId
          : "",
    },
    {
      id: "category",
      header: "Category",
      accessorFn: (row) => row.category,
      cell: (info) => info.row.original.category,
    },
    {
      id: "files",
      header: "Files",
      accessorFn: (row) => row.uniqueFileCount,
      cell: (info) =>
        info.row.original.rowType === "mod" ? info.row.original.uniqueFileCount : "",
    },
    {
      id: "last_modified",
      header: "Last Modified",
      accessorFn: (row) => row.last_modified ?? 0,
      cell: (info) =>
        info.row.original.rowType === "mod"
          ? formatLastModified(info.row.original.last_modified)
          : "",
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
      get sorting() {
        return sorting();
      },
      get globalFilter() {
        return globalFilter();
      },
    },
    onExpandedChange: setExpanded,
    onSortingChange: setSorting,
    onGlobalFilterChange: setGlobalFilter,
    globalFilterFn: (row, _columnId, filterValue) => {
      const query = String(filterValue).trim().toLowerCase();
      if (!query) return true;
      const item = row.original;
      return (
        item.name.toLowerCase().includes(query) ||
        item.author.toLowerCase().includes(query) ||
        item.category.toLowerCase().includes(query)
      );
    },
    filterFromLeafRows: true,
    getExpandedRowModel: getExpandedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  onMount(() => {
    const closeMenu = () => setContextMenu(null);
    const closeModal = () => setCategoryModal(null);
    const closeRenameModal = () => setRenameModal(null);
    const closeAuthorModal = () => setAuthorModal(null);
    const closeModIdModal = () => setModIdModal(null);
    const onKeydown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        closeMenu();
        closeModal();
        closeRenameModal();
        closeAuthorModal();
        closeModIdModal();
      }
    };
    window.addEventListener("click", closeMenu);
    window.addEventListener("scroll", closeMenu, true);
    window.addEventListener("keydown", onKeydown);
    onCleanup(() => {
      window.removeEventListener("click", closeMenu);
      window.removeEventListener("scroll", closeMenu, true);
      window.removeEventListener("keydown", onKeydown);
    });
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
      <Show when={contextMenu()}>
        {(menu) => (
          <div
            class="fixed z-50 min-w-48 rounded border border-zinc-300 bg-white py-1 shadow-lg"
            style={{ left: `${menu().x}px`, top: `${menu().y}px` }}
            onClick={(event) => event.stopPropagation()}
          >
            <button
              type="button"
              class="block w-full px-3 py-1.5 text-left text-sm text-zinc-800 hover:bg-zinc-100"
              onClick={() => {
                const selected = selectedModIds();
                const targetIds =
                  selected.size > 1 && selected.has(menu().modId)
                    ? Array.from(selected)
                    : [menu().modId];
                const currentCategory =
                  mods().find((mod) => mod.id === targetIds[0])?.category ?? "Uncategorized";
                setCategoryModal({ modIds: targetIds, category: currentCategory });
                setContextMenu(null);
              }}
            >
              Set category
            </button>
            <button
              type="button"
              class="block w-full px-3 py-1.5 text-left text-sm text-zinc-800 hover:bg-zinc-100"
              onClick={() => {
                const selected = selectedModIds();
                const targetIds =
                  selected.size > 1 && selected.has(menu().modId)
                    ? Array.from(selected)
                    : [menu().modId];
                void resetModsCategoryToAuto(targetIds);
                setContextMenu(null);
              }}
            >
              Reset to auto-category
            </button>
            <button
              type="button"
              class="block w-full px-3 py-1.5 text-left text-sm text-zinc-800 hover:bg-zinc-100"
              onClick={() => {
                const selected = selectedModIds();
                const targetIds =
                  selected.size > 1 && selected.has(menu().modId)
                    ? Array.from(selected)
                    : [menu().modId];
                const currentAuthor = mods().find((mod) => mod.id === targetIds[0])?.author ?? "";
                setAuthorModal({ modIds: targetIds, author: currentAuthor });
                setContextMenu(null);
              }}
            >
              Set author
            </button>
            <Show when={!(selectedModIds().size > 1 && selectedModIds().has(menu().modId))}>
              <button
                type="button"
                class="block w-full px-3 py-1.5 text-left text-sm text-zinc-800 hover:bg-zinc-100"
                onClick={() => {
                  const currentName = mods().find((mod) => mod.id === menu().modId)?.name ?? "";
                  setRenameModal({ modId: menu().modId, name: currentName });
                  setContextMenu(null);
                }}
              >
                Rename mod
              </button>
              <button
                type="button"
                class="block w-full px-3 py-1.5 text-left text-sm text-zinc-800 hover:bg-zinc-100"
                onClick={() => {
                  const currentId = mods().find((mod) => mod.id === menu().modId)?.nexus_mod_id;
                  setModIdModal({
                    modId: menu().modId,
                    nexusModId: currentId === null ? "" : String(currentId),
                  });
                  setContextMenu(null);
                }}
              >
                Set mod id
              </button>
            </Show>
          </div>
        )}
      </Show>
      <Show when={categoryModal()}>
        {(modal) => (
          <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/30">
            <div
              class="w-full max-w-sm rounded border border-zinc-300 bg-white p-4 shadow-lg"
              onClick={(event) => event.stopPropagation()}
            >
              <h3 class="mb-3 text-base font-semibold text-zinc-900">Set mod category</h3>
              <p class="mb-3 text-sm text-zinc-600">
                Applying to {modal().modIds.length} mod{modal().modIds.length === 1 ? "" : "s"}
              </p>
              <label class="mb-4 block">
                <span class="mb-1 block text-sm font-medium text-zinc-700">Category</span>
                <select
                  class="w-full rounded border border-zinc-300 px-3 py-2 text-sm text-zinc-900"
                  value={modal().category}
                  onChange={(event) =>
                    setCategoryModal((current) =>
                      current ? { ...current, category: event.currentTarget.value } : current,
                    )
                  }
                >
                  <For each={categories()}>
                    {(category) => <option value={category}>{category}</option>}
                  </For>
                </select>
              </label>
              <div class="flex justify-end gap-2">
                <Button
                  class="bg-zinc-200 text-zinc-900 hover:bg-zinc-300"
                  onClick={() => setCategoryModal(null)}
                >
                  Cancel
                </Button>
                <Button onClick={() => setModsCategory(modal().modIds, modal().category)}>
                  Save
                </Button>
              </div>
            </div>
          </div>
        )}
      </Show>
      <Show when={renameModal()}>
        {(modal) => (
          <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/30">
            <div
              class="w-full max-w-sm rounded border border-zinc-300 bg-white p-4 shadow-lg"
              onClick={(event) => event.stopPropagation()}
            >
              <h3 class="mb-3 text-base font-semibold text-zinc-900">Rename mod</h3>
              <label class="mb-4 block">
                <span class="mb-1 block text-sm font-medium text-zinc-700">Name</span>
                <input
                  type="text"
                  class="w-full rounded border border-zinc-300 px-3 py-2 text-sm text-zinc-900"
                  value={modal().name}
                  onInput={(event) =>
                    setRenameModal((current) =>
                      current ? { ...current, name: event.currentTarget.value } : current,
                    )
                  }
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      void renameMod(modal().modId, modal().name);
                    }
                  }}
                />
              </label>
              <div class="flex justify-end gap-2">
                <Button
                  class="bg-zinc-200 text-zinc-900 hover:bg-zinc-300"
                  onClick={() => setRenameModal(null)}
                >
                  Cancel
                </Button>
                <Button onClick={() => renameMod(modal().modId, modal().name)}>Save</Button>
              </div>
            </div>
          </div>
        )}
      </Show>
      <Show when={authorModal()}>
        {(modal) => (
          <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/30">
            <div
              class="w-full max-w-sm rounded border border-zinc-300 bg-white p-4 shadow-lg"
              onClick={(event) => event.stopPropagation()}
            >
              <h3 class="mb-3 text-base font-semibold text-zinc-900">Set author</h3>
              <p class="mb-3 text-sm text-zinc-600">
                Applying to {modal().modIds.length} mod{modal().modIds.length === 1 ? "" : "s"}
              </p>
              <label class="mb-4 block">
                <span class="mb-1 block text-sm font-medium text-zinc-700">Author</span>
                <input
                  type="text"
                  list="author-suggestions"
                  class="w-full rounded border border-zinc-300 px-3 py-2 text-sm text-zinc-900"
                  value={modal().author}
                  onInput={(event) =>
                    setAuthorModal((current) =>
                      current ? { ...current, author: event.currentTarget.value } : current,
                    )
                  }
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      void setModsAuthor(modal().modIds, modal().author);
                    }
                  }}
                />
                <datalist id="author-suggestions">
                  <For each={authorSuggestions()}>{(author) => <option value={author} />}</For>
                </datalist>
              </label>
              <div class="flex justify-end gap-2">
                <Button
                  class="bg-zinc-200 text-zinc-900 hover:bg-zinc-300"
                  onClick={() => setAuthorModal(null)}
                >
                  Cancel
                </Button>
                <Button onClick={() => setModsAuthor(modal().modIds, modal().author)}>Save</Button>
              </div>
            </div>
          </div>
        )}
      </Show>
      <Show when={modIdModal()}>
        {(modal) => (
          <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/30">
            <div
              class="w-full max-w-sm rounded border border-zinc-300 bg-white p-4 shadow-lg"
              onClick={(event) => event.stopPropagation()}
            >
              <h3 class="mb-3 text-base font-semibold text-zinc-900">Set mod id</h3>
              <label class="mb-4 block">
                <span class="mb-1 block text-sm font-medium text-zinc-700">Nexus Mod ID</span>
                <input
                  type="text"
                  inputMode="numeric"
                  class="w-full rounded border border-zinc-300 px-3 py-2 text-sm text-zinc-900"
                  value={modal().nexusModId}
                  onInput={(event) =>
                    setModIdModal((current) =>
                      current ? { ...current, nexusModId: event.currentTarget.value } : current,
                    )
                  }
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      void setModNexusId(modal().modId, modal().nexusModId);
                    }
                  }}
                />
              </label>
              <div class="flex justify-end gap-2">
                <Button
                  class="bg-zinc-200 text-zinc-900 hover:bg-zinc-300"
                  onClick={() => setModIdModal(null)}
                >
                  Cancel
                </Button>
                <Button onClick={() => setModNexusId(modal().modId, modal().nexusModId)}>
                  Save
                </Button>
              </div>
            </div>
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
                  <li>
                    <button
                      type="button"
                      class="w-full rounded px-2 py-1 text-left hover:bg-zinc-100"
                      classList={{ "bg-zinc-100 font-medium": selectedCategory() === "All" }}
                      onClick={() => setSelectedCategory("All")}
                    >
                      All
                    </button>
                  </li>
                  <For each={categories()}>
                    {(category) => (
                      <li>
                        <button
                          type="button"
                          class="w-full rounded px-2 py-1 text-left hover:bg-zinc-100"
                          classList={{ "bg-zinc-100 font-medium": selectedCategory() === category }}
                          onClick={() => setSelectedCategory(category)}
                        >
                          {category}
                        </button>
                      </li>
                    )}
                  </For>
                </ul>
              </Show>
            </div>
          }
          content={
            <div class="space-y-3">
              <div class="flex items-center justify-between gap-3">
                <h2 class="text-lg font-semibold">Mods</h2>
                <div class="flex items-center gap-2">
                  <Button
                    disabled={isApplyingMods() || isClearingMods()}
                    onClick={() => applyMods()}
                  >
                    {isApplyingMods() ? "Applying..." : "Apply mods"}
                  </Button>
                  <Button
                    class="bg-zinc-200 text-zinc-900 hover:bg-zinc-300"
                    disabled={isApplyingMods() || isClearingMods()}
                    onClick={() => clearModsOutput()}
                  >
                    {isClearingMods() ? "Clearing..." : "Clear mods"}
                  </Button>
                </div>
              </div>
              <div>
                <input
                  type="text"
                  placeholder="Filter mods..."
                  value={globalFilter()}
                  onInput={(event) => setGlobalFilter(event.currentTarget.value)}
                  class="w-full rounded border border-zinc-300 px-3 py-2 text-sm text-zinc-900 md:max-w-sm"
                />
              </div>
              <Show when={isModsLoading()}>
                <p class="text-sm text-zinc-500">Loading mods...</p>
              </Show>
              <Show when={!isModsLoading()}>
                <div class="overflow-x-auto overflow-y-auto rounded border border-zinc-200 md:overflow-x-visible">
                  <table class="w-full min-w-190 border-collapse text-left text-sm md:min-w-full">
                    <thead class="bg-zinc-50">
                      <For each={table.getHeaderGroups()}>
                        {(headerGroup) => (
                          <tr>
                            <For each={headerGroup.headers}>
                              {(header) => (
                                <th
                                  class="border-b border-zinc-200 px-3 py-2 font-semibold text-zinc-700"
                                  classList={{
                                    "w-full whitespace-normal break-words":
                                      header.column.id === "name",
                                    "whitespace-nowrap": header.column.id !== "name",
                                  }}
                                >
                                  {header.isPlaceholder ? null : (
                                    <button
                                      type="button"
                                      class="inline-flex items-center gap-1"
                                      onClick={header.column.getToggleSortingHandler()}
                                      disabled={!header.column.getCanSort()}
                                    >
                                      {flexRender(
                                        header.column.columnDef.header,
                                        header.getContext(),
                                      )}
                                      <Show when={header.column.getCanSort()}>
                                        <span class="text-xs text-zinc-500">
                                          {header.column.getIsSorted() === "asc"
                                            ? "▲"
                                            : header.column.getIsSorted() === "desc"
                                              ? "▼"
                                              : ""}
                                        </span>
                                      </Show>
                                    </button>
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
                          <tr
                            class="border-b border-zinc-100 last:border-b-0"
                            classList={{
                              "bg-zinc-100":
                                row.original.rowType === "mod" &&
                                selectedModIds().has(row.original.mod.id),
                            }}
                            onMouseDown={(event) => {
                              if (row.original.rowType !== "mod") return;
                              if (event.shiftKey) {
                                event.preventDefault();
                              }
                            }}
                            onClick={(event) => {
                              if (row.original.rowType !== "mod") return;
                              const isMulti = event.ctrlKey || event.metaKey;
                              const isRange = event.shiftKey;
                              selectModRow(row.original.mod.id, isMulti, isRange);
                            }}
                            onContextMenu={(event) => {
                              if (row.original.rowType !== "mod") return;
                              event.preventDefault();
                              if (!selectedModIds().has(row.original.mod.id)) {
                                setSelectedModIds(new Set([row.original.mod.id]));
                              }
                              setContextMenu({
                                x: event.clientX,
                                y: event.clientY,
                                modId: row.original.mod.id,
                              });
                            }}
                          >
                            <For each={row.getVisibleCells()}>
                              {(cell) => (
                                <td
                                  class="px-3 py-2 align-top text-zinc-800"
                                  classList={{
                                    "whitespace-normal break-words": cell.column.id === "name",
                                    "whitespace-nowrap": cell.column.id !== "name",
                                  }}
                                >
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
