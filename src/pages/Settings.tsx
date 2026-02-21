import { invoke } from "@tauri-apps/api/core";
import { For, Show, createSignal, onMount } from "solid-js";
import Button from "../components/Button";
import TopNav from "../components/TopNav";

type SettingField = {
  key: string;
  label: string;
  placeholder: string;
  sensitive?: boolean;
};

const SETTING_FIELDS: SettingField[] = [
  {
    key: "tokens.nexusmods",
    label: "NexusMods API key",
    placeholder: "Enter NexusMods API key",
    sensitive: true,
  },
  {
    key: "tokens.marvelrivalsapi",
    label: "MarvelRivalsAPI API key",
    placeholder: "Enter MarvelRivalsAPI API key",
    sensitive: true,
  },
  {
    key: "paths.game",
    label: "Marvel Rivals game folder",
    placeholder: "<steam directory>/steamapps/common/MarvelRivals/",
  },
  {
    key: "paths.mods",
    label: "Input mods folder",
    placeholder: "Folder where mods are stored",
  },
  {
    key: "paths.downloads",
    label: "Downloads folder",
    placeholder: "Folder where compressed mods are stored",
  },
];

function Settings() {
  const [values, setValues] = createSignal<Record<string, string>>({});
  const [isLoading, setIsLoading] = createSignal(true);
  const [isSaving, setIsSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [saveMessage, setSaveMessage] = createSignal<string | null>(null);

  const loadSettings = async () => {
    try {
      setError(null);
      const entries = await Promise.all(
        SETTING_FIELDS.map(async (field) => {
          const value = await invoke<string | null>("get_setting", { name: field.key });
          return [field.key, value ?? ""] as const;
        }),
      );
      setValues(Object.fromEntries(entries));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  onMount(() => {
    void loadSettings();
  });

  const setFieldValue = (key: string, nextValue: string) => {
    setValues((current) => ({ ...current, [key]: nextValue }));
  };

  const saveSettings = async () => {
    try {
      setIsSaving(true);
      setError(null);
      setSaveMessage(null);

      await Promise.all(
        SETTING_FIELDS.map((field) =>
          invoke("set_setting", {
            name: field.key,
            value: values()[field.key].trim() === "" ? null : values()[field.key],
          }),
        ),
      );

      setSaveMessage("Settings saved.");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div class="flex h-full min-h-0 flex-col">
      <TopNav
        rightContent={
          <Button disabled={isSaving() || isLoading()} onClick={() => saveSettings()}>
            {isSaving() ? "Saving..." : "Save settings"}
          </Button>
        }
      />
      <section class="mx-auto w-full max-w-4xl space-y-4 p-4">
        <h2 class="text-lg font-semibold">Settings</h2>
        <Show when={isLoading()}>
          <p class="text-sm text-zinc-500">Loading settings...</p>
        </Show>
        <Show when={error()}>
          {(message) => <p class="text-sm text-red-500">{message()}</p>}
        </Show>
        <Show when={saveMessage()}>
          {(message) => <p class="text-sm text-emerald-600">{message()}</p>}
        </Show>
        <Show when={!isLoading()}>
          <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
            <For each={SETTING_FIELDS}>
              {(field) => (
                <label class="space-y-1">
                  <span class="text-sm font-medium text-zinc-700">{field.label}</span>
                  <input
                    type={field.sensitive ? "password" : "text"}
                    class="w-full rounded border border-zinc-300 px-3 py-2 text-sm text-zinc-900 outline-none focus:border-zinc-500"
                    placeholder={field.placeholder}
                    value={values()[field.key] ?? ""}
                    onInput={(event) => setFieldValue(field.key, event.currentTarget.value)}
                  />
                  <p class="text-xs text-zinc-500">{field.key}</p>
                </label>
              )}
            </For>
          </div>
        </Show>
      </section>
    </div>
  );
}

export default Settings;
