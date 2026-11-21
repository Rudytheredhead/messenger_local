import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ModuleInfo {
  id: string;
  name: string;
  icon: string;
  priority: number;
}

interface StatusDto {
  id: string;
  icon: string;
  badge: number;
}

interface IslandState {
  expanded: boolean;
  user_hidden: boolean;
  fullscreen_hidden: boolean;
  status: StatusDto | null;
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

  const icon = el<HTMLElement>("pill-icon");
  const badge = el<HTMLElement>("pill-badge");
  if (state.status) {
    icon.textContent = state.status.icon;
    badge.textContent = state.status.badge > 99 ? "99+" : String(state.status.badge);
    badge.hidden = state.status.badge === 0;
  } else {
    icon.textContent = "◎";
    badge.hidden = true;
  }
}

function renderTabs(modules: ModuleInfo[]): void {
  const tabs = el<HTMLElement>("tabs");
  tabs.replaceChildren(
    ...modules.map((m) => {
      const btn = document.createElement("button");
      btn.className = "tab";
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
  for (const btn of document.querySelectorAll<HTMLButtonElement>(".tab")) {
    btn.setAttribute("aria-pressed", String(btn.dataset.id === activeModule));
  }
}

function setExpanded(expanded: boolean): void {
  void invoke("set_expanded", { expanded }).catch(console.error);
}

async function boot(): Promise<void> {
  const modules = await invoke<ModuleInfo[]>("list_modules");
  activeModule = modules[0]?.id ?? null;
  renderTabs(modules);

  renderState(await invoke<IslandState>("get_state"));
  await listen<IslandState>(STATE_EVENT, (event) => renderState(event.payload));

  const pill = el<HTMLElement>("pill");
  pill.addEventListener("click", () => setExpanded(true));
  pill.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") setExpanded(true);
  });
  el<HTMLButtonElement>("collapse").addEventListener("click", () => setExpanded(false));
}

void boot().catch(console.error);
