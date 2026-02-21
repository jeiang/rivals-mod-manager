import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

function Main() {
  const [name, setName] = createSignal("");
  const [inputValue, setInputValue] = createSignal("");
  const [greetMsg, setGreetMsg] = createSignal("");

  const handleSubmit = async (event: SubmitEvent) => {
    event.preventDefault();
    setName(inputValue());
    setGreetMsg(await invoke("greet", { name: name() }));
  };

  return (
    <main class="w-full h-full flex items-center justify-center flex-col">
      <h1>Hello, world</h1>
      <p>Sample app i guess</p>
      <form class="mt-4 flex gap-2" onSubmit={handleSubmit}>
        <input
          type="text"
          value={inputValue()}
          onInput={(event) => setInputValue(event.currentTarget.value)}
          placeholder="Enter name"
          class="border rounded px-2 py-1"
        />
        <button type="submit" class="border rounded px-3 py-1">
          Set name
        </button>
      </form>
      <p class="mt-2">{greetMsg()}</p>
    </main>
  );
}

export default Main;
