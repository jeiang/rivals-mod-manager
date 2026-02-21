import { JSX, createSignal, onCleanup } from "solid-js";

type ResizableSidebarProps = {
  sidebar: JSX.Element;
  content: JSX.Element;
  maxRatio?: number;
  minRatio?: number;
  initialRatio?: number;
};

function ResizableSidebar({
  sidebar,
  content,
  maxRatio = 1 / 3,
  minRatio = 0.1,
  initialRatio = 0.25,
}: ResizableSidebarProps) {
  // oxlint-disable-next-line no-unassigned-vars
  let containerRef: HTMLDivElement | undefined;
  const [sidebarRatio, setSidebarRatio] = createSignal(initialRatio);

  const clampRatio = (nextRatio: number) => Math.max(minRatio, Math.min(maxRatio, nextRatio));

  const startResize = (event: PointerEvent) => {
    if (!containerRef) return;

    event.preventDefault();

    const handleMove = (moveEvent: PointerEvent) => {
      if (!containerRef) return;
      const bounds = containerRef.getBoundingClientRect();
      const nextRatio = (moveEvent.clientX - bounds.left) / bounds.width;
      setSidebarRatio(clampRatio(nextRatio));
    };

    const stopResize = () => {
      document.body.style.cursor = "";
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", stopResize);
    };

    document.body.style.cursor = "col-resize";
    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", stopResize);
  };

  onCleanup(() => {
    document.body.style.cursor = "";
  });

  return (
    <div ref={containerRef} class="flex h-full w-full overflow-hidden">
      <aside
        class="h-full overflow-auto p-4"
        style={{ "flex-basis": `${sidebarRatio() * 100}%`, "flex-shrink": "0" }}
      >
        {sidebar}
      </aside>

      <div
        class="h-full w-1 cursor-col-resize bg-zinc-300 transition-colors hover:bg-zinc-200"
        onPointerDown={startResize}
        role="separator"
        aria-label="Resize sidebar"
        aria-orientation="vertical"
      />

      <section class="min-w-0 flex-1 overflow-auto p-4">{content}</section>
    </div>
  );
}

export default ResizableSidebar;
