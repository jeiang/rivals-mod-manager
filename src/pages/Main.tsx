import { invoke } from "@tauri-apps/api/core";
import { For, Show, createSignal, onMount } from "solid-js";
import ResizableSidebar from "../components/ResizableSidebar";
import TopNav from "../components/TopNav";
import Button from "../components/Button";

function Main() {
  const [categories, setCategories] = createSignal<string[]>([]);
  const [isLoading, setIsLoading] = createSignal(true);
  const [isRefreshing, setIsRefreshing] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const loadCategories = async () => {
    try {
      setError(null);
      const results = await invoke<string[]>("get_categories");
      setCategories(results);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    }
  };

  onMount(async () => {
    try {
      await loadCategories();
    } finally {
      setIsLoading(false);
    }
  });

  const refreshCategories = async () => {
    try {
      setIsRefreshing(true);
      setError(null);
      await invoke<string[]>("refresh_categories");
      await loadCategories();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setIsRefreshing(false);
    }
  };

  return (
    <div class="flex h-full min-h-0 flex-col">
      <TopNav
        rightContent={
          <Button onClick={() => refreshCategories()}>
            {isRefreshing() ? "Refreshing..." : "Refresh categories"}
          </Button>
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
              <Show when={error()}>
                {(message) => <p class="text-sm text-red-500">{message()}</p>}
              </Show>
              <Show when={!isLoading() && !error()}>
                <ul class="space-y-1 text-sm">
                  <li class="rounded px-2 py-1 hover:bg-zinc-100">All</li>
                  <For each={categories()}>
                    {(category) => <li class="rounded px-2 py-1 hover:bg-zinc-100">{category}</li>}
                  </For>
                </ul>
              </Show>
            </div>
          }
          content={<h2 class="text-lg font-semibold">Mods</h2>}
        />
      </div>
    </div>
  );
}

export default Main;
