import { invoke } from "@tauri-apps/api/core";
import { For, Show, createSignal, onMount } from "solid-js";
import ResizableSidebar from "../components/ResizableSidebar";

function Main() {
  const [categories, setCategories] = createSignal<string[]>([]);
  const [isLoading, setIsLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      const results = await invoke<string[]>("get_categories");
      setCategories(results);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setIsLoading(false);
    }
  });

  return (
    <ResizableSidebar
      sidebar={
        <div class="space-y-3">
          <h2 class="text-lg font-semibold">categories</h2>
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
      content={<h2 class="text-lg font-semibold">mods</h2>}
    />
  );
}

export default Main;
