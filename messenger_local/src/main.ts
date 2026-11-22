import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ModuleInfo {
  id: string;
  name: string;
  icon: string;
  priority: number;
}

interface IslandState {
  expanded: boolean;
  user_hidden: boolean;
  fullscreen_hidden: boolean;
}

const STATE_EVENT = "island://state";

function el<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) throw new Error(`brak elementu #${id}`);
  return node as T;
}

let activeModule: string | null = null;

function renderState(state: IslandState): void {
  document.body.classList.toggle("expanded", state.expanded);
  document.body.classList.toggle("collapsed", !state.expanded);
}

function renderApps(modules: ModuleInfo[]): void {
  const apps = el<HTMLElement>("apps");
  apps.replaceChildren(
    ...modules.map((m) => {
      const btn = document.createElement("button");
      btn.className = "app-btn";
      btn.dataset.id = m.id;
      btn.title = m.name;
      btn.setAttribute("aria-label", m.name);
      btn.textContent = m.icon;
      btn.addEventListener("click", () => {
        activeModule = m.id;
        markActive();
        void invoke("activate_module", { moduleId: m.id }).catch(console.error);
      });
      return btn;
    }),
  );
  markActive();
}

function markActive(): void {
  for (const btn of document.querySelectorAll<HTMLButtonElement>(".app-btn")) {
    btn.setAttribute("aria-pressed", String(btn.dataset.id === activeModule));
  }
}

function setExpanded(expanded: boolean): void {
  void invoke("set_expanded", { expanded }).catch(console.error);
}

async function boot(): Promise<void> {
  const modules = await invoke<ModuleInfo[]>("list_modules");
  activeModule = modules[0]?.id ?? null;
  renderApps(modules);

  renderState(await invoke<IslandState>("get_state"));
  await listen<IslandState>(STATE_EVENT, (event) => renderState(event.payload));

  const tab = el<HTMLElement>("tab");
  tab.addEventListener("click", () => setExpanded(true));
  tab.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") setExpanded(true);
  });
  el<HTMLButtonElement>("collapse").addEventListener("click", () => setExpanded(false));
}

void boot().catch(console.error);
