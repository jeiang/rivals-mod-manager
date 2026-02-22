import { invoke } from "@tauri-apps/api/core";
import { For, Index, Show, createSignal, onMount } from "solid-js";
import Button from "../components/Button";
import TopNav from "../components/TopNav";

type CategoryMatcher = {
  id: number;
  pattern: string;
  matcher_type: "string" | "regex";
  case_sensitive: boolean;
};

type CategoryWithMatchers = {
  category: string;
  matchers: CategoryMatcher[];
};

type CategoryManagementData = {
  api_categories: CategoryWithMatchers[];
  custom_categories: CategoryWithMatchers[];
};

type MatcherDraft = {
  pattern: string;
  matcher_type: "string" | "regex";
  case_sensitive: boolean;
};

function Categories() {
  const [data, setData] = createSignal<CategoryManagementData>({
    api_categories: [],
    custom_categories: [],
  });
  const [isLoading, setIsLoading] = createSignal(true);
  const [isSaving, setIsSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [customName, setCustomName] = createSignal("");
  const [activeCategory, setActiveCategory] = createSignal<string | null>(null);
  const [draftMatchers, setDraftMatchers] = createSignal<MatcherDraft[]>([]);

  const load = async () => {
    try {
      setError(null);
      const result = await invoke<CategoryManagementData>("get_category_management");
      setData(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  onMount(() => {
    void load();
  });

  const openMatcherModal = (category: CategoryWithMatchers) => {
    setActiveCategory(category.category);
    setDraftMatchers(
      category.matchers.map((matcher) => ({
        pattern: matcher.pattern,
        matcher_type: matcher.matcher_type,
        case_sensitive: matcher.case_sensitive,
      })),
    );
  };

  const saveMatchers = async () => {
    const category = activeCategory();
    if (!category) return;

    try {
      setIsSaving(true);
      setError(null);
      await invoke("set_category_matchers", {
        category,
        matchers: draftMatchers(),
      });
      setActiveCategory(null);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSaving(false);
    }
  };

  const createCategory = async () => {
    const category = customName().trim();
    if (!category) return;

    try {
      setError(null);
      await invoke("create_custom_category", { category });
      setCustomName("");
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div class="flex h-full min-h-0 flex-col">
      <TopNav />
      <section class="mx-auto w-full max-w-5xl space-y-6 p-4">
        <h2 class="text-lg font-semibold">Category Management</h2>
        <Show when={error()}>{(message) => <p class="text-sm text-red-500">{message()}</p>}</Show>
        <Show when={isLoading()}>
          <p class="text-sm text-zinc-500">Loading categories...</p>
        </Show>

        <Show when={!isLoading()}>
          <div class="space-y-2 rounded border border-zinc-200 bg-zinc-50 p-3">
            <h3 class="text-sm font-semibold text-zinc-900">Add custom category</h3>
            <div class="flex flex-wrap items-center gap-2">
              <input
                type="text"
                class="w-full rounded border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-900 md:w-80"
                placeholder="Category name"
                value={customName()}
                onInput={(event) => setCustomName(event.currentTarget.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    void createCategory();
                  }
                }}
              />
              <Button onClick={() => createCategory()}>Add</Button>
            </div>
          </div>

          <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
            <section class="space-y-2">
              <h3 class="text-base font-semibold text-zinc-900">API Categories</h3>
              <p class="text-xs text-zinc-500">
                Default matchers include hero name and real name (case-insensitive).
              </p>
              <div class="space-y-2">
                <For each={data().api_categories}>
                  {(category) => (
                    <div class="flex items-center justify-between rounded border border-zinc-200 bg-white px-3 py-2">
                      <div>
                        <p class="text-sm font-medium text-zinc-900">{category.category}</p>
                        <p class="text-xs text-zinc-500">{category.matchers.length} matcher(s)</p>
                      </div>
                      <Button
                        class="bg-zinc-200 text-zinc-900 hover:bg-zinc-300"
                        onClick={() => openMatcherModal(category)}
                      >
                        Configure matchers
                      </Button>
                    </div>
                  )}
                </For>
              </div>
            </section>

            <section class="space-y-2">
              <h3 class="text-base font-semibold text-zinc-900">Custom Categories</h3>
              <div class="space-y-2">
                <For each={data().custom_categories}>
                  {(category) => (
                    <div class="flex items-center justify-between rounded border border-zinc-200 bg-white px-3 py-2">
                      <div>
                        <p class="text-sm font-medium text-zinc-900">{category.category}</p>
                        <p class="text-xs text-zinc-500">{category.matchers.length} matcher(s)</p>
                      </div>
                      <Button
                        class="bg-zinc-200 text-zinc-900 hover:bg-zinc-300"
                        onClick={() => openMatcherModal(category)}
                      >
                        Configure matchers
                      </Button>
                    </div>
                  )}
                </For>
              </div>
            </section>
          </div>
        </Show>
      </section>

      <Show when={activeCategory()}>
        {(category) => (
          <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/30">
            <div
              class="w-full max-w-3xl rounded border border-zinc-300 bg-white p-4 shadow-lg"
              onClick={(event) => event.stopPropagation()}
            >
              <h3 class="mb-3 text-base font-semibold text-zinc-900">
                Configure matchers: {category()}
              </h3>
              <div class="space-y-2">
                <Index each={draftMatchers()}>
                  {(matcher, index) => (
                    <div class="grid grid-cols-1 gap-2 rounded border border-zinc-200 bg-zinc-50 p-2 md:grid-cols-[1fr_auto_auto_auto]">
                      <input
                        type="text"
                        class="rounded border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-900"
                        placeholder="Matcher text or regex"
                        value={matcher().pattern}
                        onInput={(event) =>
                          setDraftMatchers((current) =>
                            current.map((item, idx) =>
                              idx === index
                                ? { ...item, pattern: event.currentTarget.value }
                                : item,
                            ),
                          )
                        }
                      />
                      <select
                        class="rounded border border-zinc-300 bg-white px-2 py-2 text-sm text-zinc-900"
                        value={matcher().matcher_type}
                        onChange={(event) =>
                          setDraftMatchers((current) =>
                            current.map((item, idx) =>
                              idx === index
                                ? {
                                    ...item,
                                    matcher_type: event.currentTarget.value as "string" | "regex",
                                  }
                                : item,
                            ),
                          )
                        }
                      >
                        <option value="string">String</option>
                        <option value="regex">Regex</option>
                      </select>
                      <label class="inline-flex items-center gap-2 rounded border border-zinc-300 bg-white px-2 py-2 text-sm text-zinc-900">
                        <input
                          type="checkbox"
                          checked={matcher().case_sensitive}
                          onChange={(event) =>
                            setDraftMatchers((current) =>
                              current.map((item, idx) =>
                                idx === index
                                  ? {
                                      ...item,
                                      case_sensitive: event.currentTarget.checked,
                                    }
                                  : item,
                              ),
                            )
                          }
                        />
                        Case sensitive
                      </label>
                      <Button
                        class="bg-zinc-200 text-zinc-900 hover:bg-zinc-300"
                        onClick={() =>
                          setDraftMatchers((current) =>
                            current.filter((_, idx) => idx !== index),
                          )
                        }
                      >
                        Remove
                      </Button>
                    </div>
                  )}
                </Index>
              </div>
              <div class="mt-3 flex flex-wrap items-center justify-between gap-2">
                <Button
                  class="bg-zinc-200 text-zinc-900 hover:bg-zinc-300"
                  onClick={() =>
                    setDraftMatchers((current) => [
                      ...current,
                      { pattern: "", matcher_type: "string", case_sensitive: false },
                    ])
                  }
                >
                  Add matcher
                </Button>
                <div class="flex items-center gap-2">
                  <Button
                    class="bg-zinc-200 text-zinc-900 hover:bg-zinc-300"
                    onClick={() => setActiveCategory(null)}
                  >
                    Cancel
                  </Button>
                  <Button disabled={isSaving()} onClick={() => saveMatchers()}>
                    {isSaving() ? "Saving..." : "Save"}
                  </Button>
                </div>
              </div>
            </div>
          </div>
        )}
      </Show>
    </div>
  );
}

export default Categories;
