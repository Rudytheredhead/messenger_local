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
  large: boolean;
  user_hidden: boolean;
  fullscreen_hidden: boolean;
  messenger_running: boolean;
  spotify_running: boolean;
  discord_running: boolean;
  gemini_running: boolean;
  antigravity_running: boolean;
  active_module: string;
}

interface ChatMessage {
  role: "user" | "assistant" | "error";
  text: string;
  paths?: string[];
}

interface AgyChatThread {
  id: string;
  conversationId: string | null;
  title: string;
  updatedAt: number;
  messages: ChatMessage[];
}

interface AntigravityReply {
  reply: string;
  conversation_id: string | null;
}

type EffortLevel = "low" | "medium" | "high" | "xhigh";

interface BaseModelOption {
  id: string;
  label: string;
  subtitle?: string;
  isNew?: boolean;
  efforts: readonly EffortLevel[];
  fixedCliId?: string;
}

const EFFORT_LABELS: Record<EffortLevel, string> = {
  low: "Low",
  medium: "Medium",
  high: "High",
  xhigh: "XHigh",
};

const STATE_EVENT = "island://state";

const AGY_BASE_MODELS: BaseModelOption[] = [
  { id: "", label: "Domyślny", efforts: ["low", "medium", "high", "xhigh"] },
  { id: "gemini-4-neon", label: "Gemini 4 Neon", isNew: true, efforts: ["low", "medium", "high", "xhigh"] },
  { id: "gemini-4-argon", label: "Gemini 4 Argon", isNew: true, efforts: ["low", "medium", "high"] },
  { id: "gemini-4-hydrogen", label: "Gemini 4 Hydrogen", isNew: true, efforts: ["low", "medium", "high", "xhigh"] },
  {
    id: "orchestrator-argon",
    label: "Orchestrator",
    subtitle: "Gemini 4 Argon Medium",
    isNew: true,
    efforts: [],
    fixedCliId: "gemini-pro-orchestrator-g1-pro",
  },
  {
    id: "orchestrator-neon",
    label: "Orchestrator (Neon)",
    subtitle: "Neon",
    isNew: true,
    efforts: [],
    fixedCliId: "gemini-pro-orchestrator-neon",
  },
  { id: "gemini-3.8-flash", label: "Gemini 3.8 Flash", efforts: ["low", "medium", "high"] },
  { id: "claude-opus-5-5", label: "Claude Opus 5.5", isNew: true, efforts: ["low", "medium", "high"] },
  { id: "claude-sonnet-5-5", label: "Claude Sonnet 5.5", isNew: true, efforts: ["low", "medium", "high"] },
  {
    id: "gpt-oss-120b-medium",
    label: "GPT-OSS 120B (Medium)",
    efforts: [],
    fixedCliId: "gpt-oss-120b-medium",
  },
];

function findBaseModel(id: string): BaseModelOption {
  return AGY_BASE_MODELS.find((m) => m.id === id) ?? AGY_BASE_MODELS[0];
}

function clampEffortForModel(model: BaseModelOption, effort: EffortLevel): EffortLevel {
  if (model.efforts.length === 0) return "medium";
  if (model.efforts.includes(effort)) return effort;
  return model.efforts.includes("high") ? "high" : model.efforts[model.efforts.length - 1];
}

function resolveCliModelAndEffort(
  baseId: string,
  effort: EffortLevel
): { model: string | null; effort: EffortLevel | null } {
  const spec = findBaseModel(baseId);
  if (spec.fixedCliId) {
    return { model: spec.fixedCliId, effort: null };
  }
  const safeEffort = clampEffortForModel(spec, effort);
  if (!spec.id) {
    return { model: null, effort: safeEffort };
  }
  return { model: `${spec.id}-${safeEffort}`, effort: safeEffort };
}

const STORAGE_KEY_MODEL = "agy_selected_base_model";
const STORAGE_KEY_EFFORT = "agy_selected_effort";
const STORAGE_KEY_HISTORY = "agy_chat_history_v1";
const STORAGE_KEY_ACTIVE_CHAT = "agy_active_chat_id";
const MAX_HISTORY_THREADS = 50;

function loadSavedHistory(): AgyChatThread[] {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY_HISTORY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(
        (item): item is AgyChatThread =>
          Boolean(item) &&
          typeof item.id === "string" &&
          typeof item.title === "string" &&
          Array.isArray(item.messages)
      )
      .slice(0, MAX_HISTORY_THREADS);
  } catch {
    return [];
  }
}

function loadSavedActiveChatId(threads: AgyChatThread[]): string | null {
  try {
    const saved = window.localStorage.getItem(STORAGE_KEY_ACTIVE_CHAT);
    if (saved && threads.some((t) => t.id === saved)) {
      return saved;
    }
  } catch {
    // ignoruj błędy dostępu do localStorage
  }
  return threads[0]?.id ?? null;
}

function persistAgyHistory(): void {
  try {
    window.localStorage.setItem(
      STORAGE_KEY_HISTORY,
      JSON.stringify(agyThreads.slice(0, MAX_HISTORY_THREADS))
    );
    if (agyActiveChatId) {
      window.localStorage.setItem(STORAGE_KEY_ACTIVE_CHAT, agyActiveChatId);
    } else {
      window.localStorage.removeItem(STORAGE_KEY_ACTIVE_CHAT);
    }
  } catch {
    // ignoruj błędy zapisu do localStorage
  }
}

function buildThreadTitle(prompt: string): string {
  const clean = prompt.replace(/\s+/g, " ").trim();
  return clean.length > 42 ? `${clean.slice(0, 41)}…` : clean || "Nowy czat";
}

function getActiveChatThread(): AgyChatThread | null {
  if (!agyActiveChatId) return null;
  return agyThreads.find((t) => t.id === agyActiveChatId) ?? null;
}

function loadSavedBaseModel(): string {
  try {
    const saved = window.localStorage.getItem(STORAGE_KEY_MODEL);
    if (saved !== null && AGY_BASE_MODELS.some((m) => m.id === saved)) {
      return saved;
    }
  } catch {
    // ignoruj błędy dostępu do localStorage
  }
  return "";
}

function loadSavedEffort(): EffortLevel {
  try {
    const saved = window.localStorage.getItem(STORAGE_KEY_EFFORT);
    if (saved === "low" || saved === "medium" || saved === "high" || saved === "xhigh") {
      return saved;
    }
  } catch {
    // ignoruj błędy dostępu do localStorage
  }
  return "medium";
}

function persistAgyModelSelection(): void {
  try {
    window.localStorage.setItem(STORAGE_KEY_MODEL, agySelectedBaseModel);
    window.localStorage.setItem(STORAGE_KEY_EFFORT, agySelectedEffort);
  } catch {
    // ignoruj błędy zapisu do localStorage
  }
}

const BRAND_ICONS: Record<string, string> = {
  messenger: `
    <svg class="brand-svg" viewBox="0 0 24 24" aria-hidden="true">
      <defs>
        <linearGradient id="msg-grad" x1="15%" y1="90%" x2="85%" y2="10%">
          <stop offset="0%" stop-color="#0099FF"/>
          <stop offset="55%" stop-color="#006AFF"/>
          <stop offset="100%" stop-color="#A033FF"/>
        </linearGradient>
      </defs>
      <path fill="url(#msg-grad)" d="M12 2C6.36 2 2 6.13 2 11.7c0 2.91 1.19 5.44 3.14 7.17.16.14.26.35.27.57l.05 1.78c.02.63.67 1.04 1.24.79l1.99-.88c.17-.08.36-.1.54-.05.87.24 1.8.37 2.77.37 5.64 0 10-4.13 10-9.7S17.64 2 12 2Z"/>
      <path fill="#fff" d="m17.95 9.15-2.94 4.66a1.5 1.5 0 0 1-2.17.4l-2.34-1.75a.6.6 0 0 0-.72 0l-3.16 2.4c-.42.32-.97-.18-.69-.63l2.94-4.66a1.5 1.5 0 0 1 2.17-.4l2.34 1.75a.6.6 0 0 0 .72 0l3.16-2.4c.42-.32.97.18.69.63Z"/>
    </svg>
  `,
  discord: `
    <svg class="brand-svg" viewBox="0 0 24 24" aria-hidden="true">
      <path fill="#5865F2" d="M20.32 4.37a19.8 19.8 0 0 0-4.89-1.52.07.07 0 0 0-.08.04c-.21.38-.45.87-.61 1.25a18.27 18.27 0 0 0-5.48 0 12.6 12.6 0 0 0-.62-1.25.08.08 0 0 0-.08-.04A19.74 19.74 0 0 0 3.67 4.37a.07.07 0 0 0-.03.03C.53 9.05-.32 13.58.1 18.06a.08.08 0 0 0 .03.05 19.9 19.9 0 0 0 5.99 3.03.08.08 0 0 0 .08-.03c.46-.63.87-1.3 1.22-2a.08.08 0 0 0-.04-.11 13.1 13.1 0 0 1-1.87-.89.08.08 0 0 1-.01-.13c.13-.09.25-.19.37-.29a.07.07 0 0 1 .08-.01c3.93 1.79 8.18 1.79 12.06 0a.07.07 0 0 1 .08.01c.12.1.24.2.37.29a.08.08 0 0 1-.01.13c-.6.35-1.22.65-1.87.89a.08.08 0 0 0-.04.11c.36.7.77 1.37 1.22 2a.08.08 0 0 0 .08.03 19.83 19.83 0 0 0 6-3.03.08.08 0 0 0 .03-.05c.5-5.18-.84-9.67-3.55-13.66a.06.06 0 0 0-.03-.03ZM8.02 15.33c-1.18 0-2.16-1.08-2.16-2.42 0-1.33.96-2.42 2.16-2.42 1.21 0 2.18 1.1 2.16 2.42 0 1.34-.95 2.42-2.16 2.42Zm7.97 0c-1.18 0-2.16-1.08-2.16-2.42 0-1.33.96-2.42 2.16-2.42 1.21 0 2.18 1.1 2.16 2.42 0 1.34-.95 2.42-2.16 2.42Z"/>
    </svg>
  `,
  spotify: `
    <svg class="brand-svg" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="12" r="10" fill="#1DB954"/>
      <path fill="#121417" d="M16.58 16.05a.62.62 0 0 1-.85.2c-2.34-1.43-5.29-1.75-8.76-.96a.62.62 0 1 1-.28-1.22c3.8-.87 7.06-.5 9.69 1.11.29.18.38.57.2.87Zm1.22-2.72a.78.78 0 0 1-1.07.26c-2.68-1.65-6.77-2.13-9.94-1.16a.78.78 0 0 1-.46-1.49c3.62-1.1 8.12-.57 11.21 1.33.37.23.48.71.26 1.06Zm.11-2.83c-3.21-1.91-8.52-2.08-11.59-1.15a.94.94 0 1 1-.55-1.79c3.52-1.07 9.38-.86 13.09 1.34a.94.94 0 0 1-.95 1.6Z"/>
    </svg>
  `,
  gemini: `
    <svg class="brand-svg" viewBox="0 0 24 24" aria-hidden="true">
      <defs>
        <linearGradient id="gem-grad" x1="15%" y1="85%" x2="85%" y2="15%">
          <stop offset="0%" stop-color="#3B82F6"/>
          <stop offset="50%" stop-color="#8B5CF6"/>
          <stop offset="100%" stop-color="#EC4899"/>
        </linearGradient>
      </defs>
      <circle cx="12" cy="12" r="10" fill="#171B2E"/>
      <path fill="url(#gem-grad)" d="M12 4.2c.38 4.1 3.7 7.42 7.8 7.8-4.1.38-7.42 3.7-7.8 7.8-.38-4.1-3.7-7.42-7.8-7.8 4.1-.38 7.42-3.7 7.8-7.8Z"/>
    </svg>
  `,
  antigravity: `
    <svg class="brand-svg" viewBox="0 0 24 24" aria-hidden="true">
      <defs>
        <linearGradient id="agy-grad" x1="10%" y1="90%" x2="90%" y2="10%">
          <stop offset="0%" stop-color="#8B5CF6"/>
          <stop offset="55%" stop-color="#6366F1"/>
          <stop offset="100%" stop-color="#38BDF8"/>
        </linearGradient>
      </defs>
      <circle cx="12" cy="12" r="10" fill="url(#agy-grad)"/>
      <path fill="#fff" d="M12 5.2 13.75 10.25 18.8 12l-5.05 1.75L12 18.8l-1.75-5.05L5.2 12l5.05-1.75L12 5.2Z"/>
    </svg>
  `,
};

const EXPAND_ICON = `
  <svg viewBox="0 0 16 16" aria-hidden="true">
    <path d="M10 2h4v4M14 2l-4.5 4.5M6 14H2v-4M2 14l4.5-4.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
  </svg>
`;

const SHRINK_ICON = `
  <svg viewBox="0 0 16 16" aria-hidden="true">
    <path d="M14 6h-4V2M10 6l4.5-4.5M2 10h4v4M6 10l-4.5 4.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
  </svg>
`;

function el<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) throw new Error(`brak elementu #${id}`);
  return node as T;
}

let cachedModules: ModuleInfo[] = [];
let activeModule: string | null = null;
let currentState: IslandState = {
  expanded: false,
  large: true,
  user_hidden: false,
  fullscreen_hidden: false,
  messenger_running: false,
  spotify_running: false,
  discord_running: false,
  gemini_running: false,
  antigravity_running: false,
  active_module: "messenger",
};

let agyThreads: AgyChatThread[] = loadSavedHistory();
let agyActiveChatId: string | null = loadSavedActiveChatId(agyThreads);
let agyMessages: ChatMessage[] = [...(getActiveChatThread()?.messages ?? [])];
let agyHistoryOpen = false;
let agyPending = false;
let agyDraft = "";
let agySelectedBaseModel = loadSavedBaseModel();
let agySelectedEffort: EffortLevel = loadSavedEffort();
let agyModelMenuOpen = false;
let agySubmenuModelId: string | null = null;
let agyMenuListenersBound = false;
let agyAttachedPaths: string[] = [];
let agyPathInputVisible = false;

function isModuleRunning(state: IslandState, id: string): boolean {
  switch (id) {
    case "messenger":
      return state.messenger_running;
    case "spotify":
      return state.spotify_running;
    case "discord":
      return state.discord_running;
    case "gemini":
      return state.gemini_running;
    case "antigravity":
      return state.antigravity_running;
    default:
      return false;
  }
}

function renderBrandIcon(target: HTMLElement, id: string, fallback: string): void {
  const svg = BRAND_ICONS[id];
  if (svg) {
    target.innerHTML = svg;
  } else {
    target.textContent = fallback;
  }
}

function formatPathShort(raw: string): string {
  const normalized = raw.replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]/);
  return parts[parts.length - 1] || raw;
}

function addAttachedPaths(newPaths: string[]): void {
  let added = false;
  for (const raw of newPaths) {
    const clean = raw.trim().replace(/^"(.*)"$/, "$1").trim();
    if (clean && !agyAttachedPaths.includes(clean)) {
      agyAttachedPaths.push(clean);
      added = true;
    }
  }
  if (added) {
    renderAttachedPaths();
  }
}

function renderAttachedPaths(): void {
  const bar = document.getElementById("agy-paths-bar");
  if (!bar) return;

  bar.replaceChildren();
  bar.hidden = agyAttachedPaths.length === 0;

  for (const path of agyAttachedPaths) {
    const chip = document.createElement("div");
    chip.className = "agy-path-chip";
    chip.title = path;

    const label = document.createElement("span");
    label.className = "agy-path-label";
    label.textContent = formatPathShort(path);

    const removeBtn = document.createElement("button");
    removeBtn.type = "button";
    removeBtn.className = "agy-path-remove";
    removeBtn.title = "Usuń ścieżkę";
    removeBtn.setAttribute("aria-label", `Usuń ${path}`);
    removeBtn.textContent = "×";
    removeBtn.addEventListener("click", () => {
      agyAttachedPaths = agyAttachedPaths.filter((p) => p !== path);
      renderAttachedPaths();
    });

    chip.append(label, removeBtn);
    bar.appendChild(chip);
  }
}

function appendFormattedText(container: HTMLElement, text: string): void {
  const parts = text.split(/```([\s\S]*?)```/g);
  parts.forEach((part, index) => {
    if (index % 2 === 1) {
      const pre = document.createElement("pre");
      pre.className = "agy-code-block";
      const code = document.createElement("code");
      const trimmed = part.replace(/^[a-zA-Z0-9_-]+\n/, "");
      code.textContent = trimmed.trim();
      pre.appendChild(code);
      container.appendChild(pre);
    } else if (part.trim().length > 0) {
      const lines = part.split(/`([^`]+)`/g);
      const p = document.createElement("div");
      p.className = "agy-text";
      lines.forEach((segment, i) => {
        if (i % 2 === 1) {
          const inlineCode = document.createElement("code");
          inlineCode.className = "agy-inline-code";
          inlineCode.textContent = segment;
          p.appendChild(inlineCode);
        } else if (segment.length > 0) {
          p.appendChild(document.createTextNode(segment));
        }
      });
      container.appendChild(p);
    }
  });
}

function renderAntigravityMessages(): void {
  const list = document.getElementById("agy-messages");
  if (!list) return;

  list.replaceChildren();

  if (agyMessages.length === 0 && !agyPending) {
    const empty = document.createElement("div");
    empty.className = "agy-empty";

    const badge = document.createElement("div");
    badge.className = "agy-empty-badge";
    renderBrandIcon(badge, "antigravity", "✨");

    const title = document.createElement("div");
    title.className = "agy-empty-title";
    title.textContent = "Antigravity gotowy do pracy";

    const sub = document.createElement("div");
    sub.className = "agy-empty-sub";
    sub.textContent =
      "Wybierz model u góry, dodaj pliki lub foldery (+ Plik / + Folder / przeciągnij i upuść) i napisz polecenie.";

    empty.append(badge, title, sub);
    list.appendChild(empty);
  } else {
    for (const msg of agyMessages) {
      const bubble = document.createElement("div");
      bubble.className = `agy-msg ${msg.role}`;
      if (msg.role === "user") {
        if (msg.paths && msg.paths.length > 0) {
          const pathsWrap = document.createElement("div");
          pathsWrap.className = "agy-msg-paths";
          for (const p of msg.paths) {
            const tag = document.createElement("span");
            tag.className = "agy-msg-path-tag";
            tag.title = p;
            tag.textContent = formatPathShort(p);
            pathsWrap.appendChild(tag);
          }
          bubble.appendChild(pathsWrap);
        }
        const textNode = document.createElement("div");
        textNode.textContent = msg.text;
        bubble.appendChild(textNode);
      } else {
        appendFormattedText(bubble, msg.text);
      }
      list.appendChild(bubble);
    }

    if (agyPending) {
      const loading = document.createElement("div");
      loading.className = "agy-msg assistant loading";
      loading.textContent = "Antigravity myśli…";
      list.appendChild(loading);
    }
  }

  list.scrollTop = list.scrollHeight;

  const sendBtn = document.getElementById("agy-send-btn") as HTMLButtonElement | null;
  if (sendBtn) {
    sendBtn.disabled = agyPending;
  }
}

function formatThreadTime(ts: number): string {
  try {
    return new Date(ts).toLocaleTimeString("pl-PL", { hour: "2-digit", minute: "2-digit" });
  } catch {
    return "";
  }
}

function renderAntigravityHistory(): void {
  const btn = document.getElementById("agy-history-btn");
  if (btn) {
    btn.classList.toggle("is-open", agyHistoryOpen);
    btn.setAttribute("aria-expanded", String(agyHistoryOpen));
    btn.textContent = agyThreads.length > 0 ? `Historia (${agyThreads.length})` : "Historia";
  }

  const panel = document.getElementById("agy-history-panel");
  if (!panel) return;
  panel.hidden = !agyHistoryOpen;
  if (!agyHistoryOpen) return;

  const list = document.getElementById("agy-history-list");
  if (!list) return;
  list.replaceChildren();

  if (agyThreads.length === 0) {
    const empty = document.createElement("div");
    empty.className = "agy-history-empty";
    empty.textContent = "Brak zapisanych rozmów.";
    list.appendChild(empty);
    return;
  }

  for (const thread of agyThreads) {
    const row = document.createElement("div");
    row.className = "agy-history-item";
    if (thread.id === agyActiveChatId) {
      row.classList.add("is-active");
    }

    const openBtn = document.createElement("button");
    openBtn.type = "button";
    openBtn.className = "agy-history-open";
    openBtn.title = thread.title;

    const titleEl = document.createElement("span");
    titleEl.className = "agy-history-item-title";
    titleEl.textContent = thread.title;

    const metaEl = document.createElement("span");
    metaEl.className = "agy-history-item-meta";
    const count = thread.messages.length;
    metaEl.textContent = `${formatThreadTime(thread.updatedAt)} · ${count} wiad.`;

    openBtn.append(titleEl, metaEl);
    openBtn.addEventListener("click", () => selectAgyChat(thread.id));

    const delBtn = document.createElement("button");
    delBtn.type = "button";
    delBtn.className = "agy-history-delete";
    delBtn.title = "Usuń rozmowę";
    delBtn.setAttribute("aria-label", `Usuń ${thread.title}`);
    delBtn.textContent = "×";
    delBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      deleteAgyChat(thread.id);
    });

    row.append(openBtn, delBtn);
    list.appendChild(row);
  }
}

function createNewAgyChat(): void {
  if (agyPending) return;
  agyActiveChatId = null;
  agyMessages = [];
  agyAttachedPaths = [];
  persistAgyHistory();
  renderAttachedPaths();
  renderAntigravityHistory();
  renderAntigravityMessages();
  void invoke("reset_antigravity").catch(console.error);
  const field = document.getElementById("agy-input") as HTMLTextAreaElement | null;
  field?.focus();
}

function selectAgyChat(id: string): void {
  if (agyPending) return;
  const thread = agyThreads.find((t) => t.id === id);
  if (!thread) return;
  agyActiveChatId = thread.id;
  agyMessages = [...thread.messages];
  if (!currentState.large) {
    agyHistoryOpen = false;
  }
  persistAgyHistory();
  renderAntigravityHistory();
  renderAntigravityMessages();
}

function deleteAgyChat(id: string): void {
  if (agyPending) return;
  agyThreads = agyThreads.filter((t) => t.id !== id);
  if (agyActiveChatId === id) {
    agyActiveChatId = agyThreads[0]?.id ?? null;
    agyMessages = [...(getActiveChatThread()?.messages ?? [])];
  }
  persistAgyHistory();
  renderAntigravityHistory();
  renderAntigravityMessages();
}

async function sendAntigravityPrompt(): Promise<void> {
  if (agyPending) return;
  const input = document.getElementById("agy-input") as HTMLTextAreaElement | null;
  const text = (input ? input.value : agyDraft).trim();
  if (!text) return;

  const usedPaths = [...agyAttachedPaths];
  let thread = getActiveChatThread();
  if (!thread) {
    thread = {
      id: `chat_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
      conversationId: null,
      title: buildThreadTitle(text),
      updatedAt: Date.now(),
      messages: [],
    };
    agyThreads.unshift(thread);
    agyActiveChatId = thread.id;
  } else {
    agyThreads = [thread, ...agyThreads.filter((t) => t.id !== thread!.id)].slice(
      0,
      MAX_HISTORY_THREADS
    );
  }

  thread.messages.push({
    role: "user",
    text,
    paths: usedPaths.length > 0 ? usedPaths : undefined,
  });
  thread.updatedAt = Date.now();
  agyMessages = thread.messages;
  persistAgyHistory();

  agyDraft = "";
  if (input) input.value = "";
  agyAttachedPaths = [];
  renderAttachedPaths();
  agyPending = true;
  renderAntigravityHistory();
  renderAntigravityMessages();

  try {
    const resolved = resolveCliModelAndEffort(agySelectedBaseModel, agySelectedEffort);
    const res = await invoke<AntigravityReply>("ask_antigravity", {
      prompt: text,
      model: resolved.model,
      effort: resolved.effort,
      conversationId: thread.conversationId,
      paths: usedPaths,
    });
    if (res.conversation_id) {
      thread.conversationId = res.conversation_id;
    }
    thread.messages.push({ role: "assistant", text: res.reply });
    thread.updatedAt = Date.now();
    persistAgyHistory();
  } catch (err) {
    thread.messages.push({
      role: "error",
      text: typeof err === "string" ? err : String(err),
    });
    thread.updatedAt = Date.now();
    persistAgyHistory();
  } finally {
    agyPending = false;
    renderAntigravityHistory();
    renderAntigravityMessages();
    const field = document.getElementById("agy-input") as HTMLTextAreaElement | null;
    field?.focus();
  }
}

function syncAgyModelControls(): void {
  const spec = findBaseModel(agySelectedBaseModel);
  if (spec.efforts.length > 0) {
    agySelectedEffort = clampEffortForModel(spec, agySelectedEffort);
  }

  const trigger = document.getElementById("agy-model-trigger") as HTMLButtonElement | null;
  if (trigger) {
    const labelSpan = trigger.querySelector(".agy-model-trigger-label");
    if (labelSpan) labelSpan.textContent = spec.label;
    trigger.setAttribute("aria-expanded", String(agyModelMenuOpen));
  }

  const effortSelect = document.getElementById("agy-effort-select") as HTMLSelectElement | null;
  if (effortSelect) {
    if (spec.efforts.length === 0) {
      effortSelect.hidden = true;
      effortSelect.disabled = true;
    } else {
      effortSelect.hidden = false;
      effortSelect.disabled = false;
      const existingValues = Array.from(effortSelect.options).map((o) => o.value).join(",");
      const neededValues = spec.efforts.join(",");
      if (existingValues !== neededValues) {
        effortSelect.innerHTML = "";
        for (const eff of spec.efforts) {
          const opt = document.createElement("option");
          opt.value = eff;
          opt.textContent = EFFORT_LABELS[eff];
          effortSelect.appendChild(opt);
        }
      }
      effortSelect.value = agySelectedEffort;
    }
  }
  persistAgyModelSelection();
}

function closeAgyModelMenu(): void {
  agyModelMenuOpen = false;
  agySubmenuModelId = null;
  const menu = document.getElementById("agy-model-menu");
  const submenu = document.getElementById("agy-effort-submenu");
  const trigger = document.getElementById("agy-model-trigger");
  if (menu) menu.hidden = true;
  if (submenu) submenu.hidden = true;
  if (trigger) trigger.setAttribute("aria-expanded", "false");
}

function renderAgyEffortSubmenu(modelId: string, anchorTop: number): void {
  const submenu = document.getElementById("agy-effort-submenu");
  if (!submenu) return;

  const model = findBaseModel(modelId);
  if (model.efforts.length === 0) {
    submenu.hidden = true;
    return;
  }

  submenu.innerHTML = "";
  const title = document.createElement("div");
  title.className = "agy-menu-title";
  title.textContent = "Thinking effort";
  submenu.appendChild(title);

  const activeEffort = clampEffortForModel(model, agySelectedEffort);
  for (const eff of model.efforts) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "agy-effort-item";

    const name = document.createElement("span");
    name.textContent = EFFORT_LABELS[eff];

    const check = document.createElement("span");
    check.className = "agy-check";
    check.textContent = eff === activeEffort ? "✓" : "";

    btn.append(name, check);
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      agySelectedBaseModel = model.id;
      agySelectedEffort = eff;
      syncAgyModelControls();
      closeAgyModelMenu();
    });
    submenu.appendChild(btn);
  }

  submenu.style.top = `${Math.max(4, Math.min(anchorTop, 175))}px`;
  submenu.hidden = false;
}

function renderAgyModelMenu(): void {
  const menu = document.getElementById("agy-model-menu");
  const list = menu?.querySelector(".agy-model-list") as HTMLElement | null;
  const submenu = document.getElementById("agy-effort-submenu");
  if (!menu || !list) return;

  if (!agyModelMenuOpen) {
    menu.hidden = true;
    if (submenu) submenu.hidden = true;
    return;
  }

  list.innerHTML = "";
  for (const spec of AGY_BASE_MODELS) {
    const row = document.createElement("button");
    row.type = "button";
    row.className = "agy-model-row";
    if (agySubmenuModelId === spec.id) {
      row.classList.add("is-submenu-open");
    }

    const main = document.createElement("span");
    main.className = "agy-model-row-main";

    const nameSpan = document.createElement("span");
    nameSpan.className = "agy-model-name";
    nameSpan.textContent = spec.label;
    main.appendChild(nameSpan);

    if (spec.efforts.length > 0) {
      const effSpan = document.createElement("span");
      effSpan.className = "agy-model-row-meta";
      effSpan.textContent = EFFORT_LABELS[clampEffortForModel(spec, agySelectedEffort)];
      main.appendChild(effSpan);
    } else if (spec.subtitle) {
      const subSpan = document.createElement("span");
      subSpan.className = "agy-model-row-meta";
      subSpan.textContent = spec.subtitle;
      main.appendChild(subSpan);
    }

    if (spec.isNew) {
      const badge = document.createElement("span");
      badge.className = "agy-badge-new";
      badge.textContent = "New";
      main.appendChild(badge);
    }

    const right = document.createElement("span");
    right.className = "agy-model-row-right";

    const check = document.createElement("span");
    check.className = "agy-check";
    check.textContent = agySelectedBaseModel === spec.id ? "✓" : "";
    right.appendChild(check);

    if (spec.efforts.length > 0) {
      const arrow = document.createElement("span");
      arrow.className = "agy-submenu-arrow";
      arrow.textContent = "›";
      arrow.title = "Wybierz Thinking effort";
      arrow.addEventListener("click", (e) => {
        e.stopPropagation();
        agySubmenuModelId = spec.id;
        list.querySelectorAll(".agy-model-row").forEach((el) => el.classList.remove("is-submenu-open"));
        row.classList.add("is-submenu-open");
        renderAgyEffortSubmenu(spec.id, row.offsetTop);
      });
      right.appendChild(arrow);
    }

    row.append(main, right);

    row.addEventListener("mouseenter", () => {
      list.querySelectorAll(".agy-model-row").forEach((el) => el.classList.remove("is-submenu-open"));
      if (spec.efforts.length > 0) {
        agySubmenuModelId = spec.id;
        row.classList.add("is-submenu-open");
        renderAgyEffortSubmenu(spec.id, row.offsetTop);
      } else {
        agySubmenuModelId = null;
        if (submenu) submenu.hidden = true;
      }
    });

    row.addEventListener("click", () => {
      agySelectedBaseModel = spec.id;
      if (spec.efforts.length > 0) {
        agySelectedEffort = clampEffortForModel(spec, agySelectedEffort);
      }
      syncAgyModelControls();
      closeAgyModelMenu();
    });

    list.appendChild(row);
  }

  menu.hidden = false;
  if (submenu) {
    if (agySubmenuModelId) {
      const hoveredRow = Array.from(list.children).find(
        (_, idx) => AGY_BASE_MODELS[idx]?.id === agySubmenuModelId
      ) as HTMLElement | undefined;
      renderAgyEffortSubmenu(agySubmenuModelId, hoveredRow?.offsetTop ?? 24);
    } else {
      submenu.hidden = true;
    }
  }
}

function bindAgyMenuGlobalListeners(): void {
  if (agyMenuListenersBound) return;
  agyMenuListenersBound = true;

  document.addEventListener("pointerdown", (e) => {
    if (!agyModelMenuOpen) return;
    const target = e.target as Node | null;
    if (!target) return;
    const picker = document.getElementById("agy-model-picker");
    const menu = document.getElementById("agy-model-menu");
    if (picker?.contains(target) || menu?.contains(target)) return;
    closeAgyModelMenu();
  });

  window.addEventListener(
    "keydown",
    (e) => {
      if (e.key === "Escape" && agyModelMenuOpen) {
        e.preventDefault();
        e.stopImmediatePropagation();
        closeAgyModelMenu();
      }
    },
    true
  );
}

function mountAntigravityView(content: HTMLElement): void {
  if (content.querySelector(".agy-chat")) {
    syncAgyModelControls();
    renderAttachedPaths();
    renderAntigravityHistory();
    renderAntigravityMessages();
    return;
  }

  content.replaceChildren();

  const chat = document.createElement("div");
  chat.className = "agy-chat";

  // Nagłówek z wyborem modelu i przyciskiem resetu
  const header = document.createElement("div");
  header.className = "agy-header";

  const titleWrap = document.createElement("div");
  titleWrap.className = "agy-header-title";
  const icon = document.createElement("span");
  icon.className = "agy-header-icon";
  renderBrandIcon(icon, "antigravity", "✨");
  const titleText = document.createElement("span");
  titleText.textContent = "Antigravity";
  titleWrap.append(icon, titleText);

  const controlsWrap = document.createElement("div");
  controlsWrap.className = "agy-header-controls";

  const modelPicker = document.createElement("div");
  modelPicker.className = "agy-model-picker";
  modelPicker.id = "agy-model-picker";

  const modelTrigger = document.createElement("button");
  modelTrigger.type = "button";
  modelTrigger.className = "agy-model-trigger";
  modelTrigger.id = "agy-model-trigger";
  modelTrigger.title = "Wybierz model AI";
  modelTrigger.setAttribute("aria-haspopup", "menu");
  modelTrigger.setAttribute("aria-expanded", "false");

  const triggerLabel = document.createElement("span");
  triggerLabel.className = "agy-model-trigger-label";
  const triggerChevron = document.createElement("span");
  triggerChevron.className = "agy-chevron";
  triggerChevron.textContent = "▾";
  modelTrigger.append(triggerLabel, triggerChevron);

  modelTrigger.addEventListener("click", (e) => {
    e.stopPropagation();
    agyModelMenuOpen = !agyModelMenuOpen;
    if (agyModelMenuOpen) {
      const currentSpec = findBaseModel(agySelectedBaseModel);
      agySubmenuModelId = currentSpec.efforts.length > 0 ? currentSpec.id : null;
    } else {
      agySubmenuModelId = null;
    }
    syncAgyModelControls();
    renderAgyModelMenu();
  });
  modelPicker.appendChild(modelTrigger);

  const effortSelect = document.createElement("select");
  effortSelect.className = "agy-effort-select";
  effortSelect.id = "agy-effort-select";
  effortSelect.title = "Poziom Thinking effort";
  effortSelect.setAttribute("aria-label", "Thinking effort");
  effortSelect.addEventListener("change", () => {
    agySelectedEffort = effortSelect.value as EffortLevel;
    persistAgyModelSelection();
    if (agyModelMenuOpen) {
      renderAgyModelMenu();
    }
  });

  const modelMenu = document.createElement("div");
  modelMenu.className = "agy-model-menu";
  modelMenu.id = "agy-model-menu";
  modelMenu.hidden = true;

  const menuTitle = document.createElement("div");
  menuTitle.className = "agy-menu-title";
  menuTitle.textContent = "Model";

  const modelList = document.createElement("div");
  modelList.className = "agy-model-list";

  const effortSubmenu = document.createElement("div");
  effortSubmenu.className = "agy-effort-submenu";
  effortSubmenu.id = "agy-effort-submenu";
  effortSubmenu.hidden = true;

  modelMenu.append(menuTitle, modelList, effortSubmenu);

  const historyBtn = document.createElement("button");
  historyBtn.type = "button";
  historyBtn.className = "agy-history-btn";
  historyBtn.id = "agy-history-btn";
  historyBtn.title = "Pokaż lub ukryj historię czatów";
  historyBtn.setAttribute("aria-expanded", String(agyHistoryOpen));
  historyBtn.addEventListener("click", () => {
    agyHistoryOpen = !agyHistoryOpen;
    renderAntigravityHistory();
  });

  const resetBtn = document.createElement("button");
  resetBtn.className = "agy-reset-btn";
  resetBtn.id = "agy-reset-btn";
  resetBtn.textContent = "Nowa";
  resetBtn.title = "Rozpocznij nową rozmowę";
  resetBtn.addEventListener("click", () => createNewAgyChat());

  controlsWrap.append(historyBtn, modelPicker, effortSelect, resetBtn);
  header.append(titleWrap, controlsWrap, modelMenu);

  agyModelMenuOpen = false;
  agySubmenuModelId = null;
  bindAgyMenuGlobalListeners();

  // Obszar roboczy: opcjonalny panel historii + lista wiadomości
  const chatBody = document.createElement("div");
  chatBody.className = "agy-chat-body";

  const historyPanel = document.createElement("aside");
  historyPanel.className = "agy-history-panel";
  historyPanel.id = "agy-history-panel";
  historyPanel.hidden = !agyHistoryOpen;

  const historyHead = document.createElement("div");
  historyHead.className = "agy-history-header";
  const historyTitle = document.createElement("span");
  historyTitle.textContent = "Historia czatów";
  const historyNewBtn = document.createElement("button");
  historyNewBtn.type = "button";
  historyNewBtn.className = "agy-history-new-btn";
  historyNewBtn.textContent = "+ Nowy";
  historyNewBtn.addEventListener("click", () => createNewAgyChat());
  historyHead.append(historyTitle, historyNewBtn);

  const historyList = document.createElement("div");
  historyList.className = "agy-history-list";
  historyList.id = "agy-history-list";
  historyPanel.append(historyHead, historyList);

  const messages = document.createElement("div");
  messages.className = "agy-messages";
  messages.id = "agy-messages";
  chatBody.append(historyPanel, messages);

  // Pasek szybkiego dodawania ścieżek
  const footer = document.createElement("div");
  footer.className = "agy-footer";

  const pathsBar = document.createElement("div");
  pathsBar.className = "agy-paths-bar";
  pathsBar.id = "agy-paths-bar";
  pathsBar.hidden = true;

  const pathAdder = document.createElement("div");
  pathAdder.className = "agy-path-adder";
  pathAdder.id = "agy-path-adder";
  pathAdder.hidden = !agyPathInputVisible;

  const pathInput = document.createElement("input");
  pathInput.type = "text";
  pathInput.className = "agy-path-input";
  pathInput.id = "agy-path-input";
  pathInput.placeholder = "Wklej ścieżkę pliku lub folderu (np. E:\\projekt\\plik.ts)…";

  const commitManualPath = (): void => {
    const val = pathInput.value.trim();
    if (val) {
      addAttachedPaths([val]);
      pathInput.value = "";
    }
    agyPathInputVisible = false;
    pathAdder.hidden = true;
    input.focus();
  };

  pathInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      commitManualPath();
    } else if (e.key === "Escape") {
      e.stopPropagation();
      agyPathInputVisible = false;
      pathAdder.hidden = true;
      input.focus();
    }
  });

  const pathAddConfirm = document.createElement("button");
  pathAddConfirm.type = "button";
  pathAddConfirm.className = "agy-tool-btn primary";
  pathAddConfirm.textContent = "Dodaj";
  pathAddConfirm.addEventListener("click", commitManualPath);

  pathAdder.append(pathInput, pathAddConfirm);

  const toolbar = document.createElement("div");
  toolbar.className = "agy-toolbar";

  const addFileBtn = document.createElement("button");
  addFileBtn.type = "button";
  addFileBtn.className = "agy-tool-btn";
  addFileBtn.title = "Wybierz pliki z dysku";
  addFileBtn.textContent = "+ Plik";
  addFileBtn.addEventListener("click", () => {
    void invoke<string[]>("pick_antigravity_files")
      .then((paths) => {
        if (paths && paths.length > 0) addAttachedPaths(paths);
      })
      .catch(console.error);
  });

  const addFolderBtn = document.createElement("button");
  addFolderBtn.type = "button";
  addFolderBtn.className = "agy-tool-btn";
  addFolderBtn.title = "Wybierz folder projektu";
  addFolderBtn.textContent = "+ Folder";
  addFolderBtn.addEventListener("click", () => {
    void invoke<string | null>("pick_antigravity_folder")
      .then((folder) => {
        if (folder) addAttachedPaths([folder]);
      })
      .catch(console.error);
  });

  const addManualBtn = document.createElement("button");
  addManualBtn.type = "button";
  addManualBtn.className = "agy-tool-btn";
  addManualBtn.title = "Wklej ścieżkę ręcznie";
  addManualBtn.textContent = "+ Ścieżka";
  addManualBtn.addEventListener("click", () => {
    agyPathInputVisible = !agyPathInputVisible;
    pathAdder.hidden = !agyPathInputVisible;
    if (agyPathInputVisible) {
      pathInput.focus();
    }
  });

  const dropHint = document.createElement("span");
  dropHint.className = "agy-drop-hint";
  dropHint.textContent = "lub upuść pliki tutaj";

  toolbar.append(addFileBtn, addFolderBtn, addManualBtn, dropHint);

  // Pole wiadomości
  const form = document.createElement("form");
  form.className = "agy-input-bar";
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    void sendAntigravityPrompt();
  });

  const input = document.createElement("textarea");
  input.className = "agy-input";
  input.id = "agy-input";
  input.rows = 1;
  input.placeholder = "Napisz do Antigravity…";
  input.value = agyDraft;
  input.addEventListener("input", () => {
    agyDraft = input.value;
  });
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void sendAntigravityPrompt();
    }
  });

  const sendBtn = document.createElement("button");
  sendBtn.className = "agy-send-btn";
  sendBtn.id = "agy-send-btn";
  sendBtn.type = "submit";
  sendBtn.title = "Wyślij";
  sendBtn.setAttribute("aria-label", "Wyślij");
  sendBtn.innerHTML = `
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <path d="M8 13V3M3.5 7.5 8 3l4.5 4.5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  `;

  form.append(input, sendBtn);
  footer.append(pathsBar, pathAdder, toolbar, form);
  chat.append(header, chatBody, footer);
  content.appendChild(chat);

  syncAgyModelControls();
  renderAttachedPaths();
  renderAntigravityHistory();
  renderAntigravityMessages();
  setTimeout(() => input.focus(), 40);
}

function renderState(state: IslandState): void {
  if (!state.antigravity_running && currentState.antigravity_running) {
    agyDraft = "";
    agyPending = false;
    agyAttachedPaths = [];
    agyPathInputVisible = false;
  }

  currentState = state;
  activeModule = state.active_module;
  document.body.classList.toggle("expanded", state.expanded);
  document.body.classList.toggle("collapsed", !state.expanded);
  document.body.classList.toggle("large", state.large);

  const toggleLargeBtn = document.getElementById("toggle-large");
  if (toggleLargeBtn) {
    toggleLargeBtn.innerHTML = state.large ? SHRINK_ICON : EXPAND_ICON;
    const label = state.large ? "Zmniejsz okno" : "Powiększ okno";
    toggleLargeBtn.title = label;
    toggleLargeBtn.setAttribute("aria-label", label);
  }

  for (const item of document.querySelectorAll<HTMLElement>(".app-item")) {
    const id = item.dataset.id ?? "";
    const running = isModuleRunning(state, id);
    const isClosed = !running;
    item.classList.toggle("closed", isClosed);
    item.classList.toggle("running", running);
    const btn = item.querySelector<HTMLButtonElement>(".app-btn");
    if (btn) {
      const name = btn.dataset.name ?? "Aplikacja";
      btn.title = isClosed ? `${name} (wyłączona — kliknij, aby włączyć)` : name;
    }
  }

  const content = el<HTMLElement>("content");
  const activeId = state.active_module;
  const activeRunning = isModuleRunning(state, activeId);

  if (state.expanded && !activeRunning) {
    const mod = cachedModules.find((m) => m.id === activeId);
    const modName = mod?.name ?? activeId;
    const modIcon = mod?.icon ?? "💬";

    content.replaceChildren();
    const screen = document.createElement("div");
    screen.className = "closed-screen";
    screen.dataset.brand = activeId;

    const card = document.createElement("div");
    card.className = "closed-card";
    card.dataset.brand = activeId;

    const iconEl = document.createElement("div");
    iconEl.className = "closed-screen-icon";
    renderBrandIcon(iconEl, activeId, modIcon);

    const titleEl = document.createElement("div");
    titleEl.className = "closed-screen-title";
    titleEl.textContent = `${modName} jest wyłączony`;

    const descEl = document.createElement("div");
    descEl.className = "closed-screen-desc";
    descEl.textContent =
      "Aplikacja nie zużywa teraz pamięci. Kliknij ikonę na pasku, przycisk poniżej lub naciśnij Enter, aby ją uruchomić.";

    const startBtn = document.createElement("button");
    startBtn.className = "closed-screen-btn";
    startBtn.id = "start-app-btn";
    startBtn.dataset.brand = activeId;
    startBtn.textContent = `Uruchom ${modName}`;
    startBtn.addEventListener("click", () => {
      void invoke("start_module", { moduleId: activeId }).catch(console.error);
    });

    card.append(iconEl, titleEl, descEl, startBtn);
    screen.appendChild(card);
    content.appendChild(screen);
  } else if (state.expanded && activeId === "antigravity" && activeRunning) {
    mountAntigravityView(content);
  } else {
    content.replaceChildren();
  }

  markActive();
}

function renderApps(modules: ModuleInfo[]): void {
  cachedModules = modules;
  const apps = el<HTMLElement>("apps");
  apps.replaceChildren(
    ...modules.map((m) => {
      const item = document.createElement("div");
      item.className = "app-item";
      item.dataset.id = m.id;
      item.dataset.brand = m.id;

      const btn = document.createElement("button");
      btn.className = "app-btn";
      btn.dataset.id = m.id;
      btn.dataset.name = m.name;
      btn.dataset.brand = m.id;
      btn.title = m.name;
      btn.setAttribute("aria-label", m.name);
      renderBrandIcon(btn, m.id, m.icon);

      btn.addEventListener("click", () => {
        if (!isModuleRunning(currentState, m.id)) {
          void invoke("start_module", { moduleId: m.id }).catch(console.error);
          return;
        }
        activeModule = m.id;
        markActive();
        void invoke("activate_module", { moduleId: m.id }).catch(console.error);
      });

      const closeBtn = document.createElement("button");
      closeBtn.className = "app-close-btn";
      closeBtn.title = "Zamknij aplikację";
      closeBtn.setAttribute("aria-label", `Zamknij ${m.name}`);
      closeBtn.innerHTML = `
        <svg viewBox="0 0 10 10" aria-hidden="true">
          <path d="M2.5 2.5 L7.5 7.5 M7.5 2.5 L2.5 7.5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
        </svg>
      `;

      closeBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        void invoke("close_module", { moduleId: m.id }).catch(console.error);
      });

      item.appendChild(btn);
      item.appendChild(closeBtn);
      return item;
    }),
  );
  renderState(currentState);
}

function markActive(): void {
  for (const item of document.querySelectorAll<HTMLElement>(".app-item")) {
    const id = item.dataset.id ?? "";
    const running = isModuleRunning(currentState, id);
    const isSelected = id === activeModule;
    const isActive = running && isSelected;
    item.classList.toggle("active", isActive);
    item.classList.toggle("selected", isSelected);
    const btn = item.querySelector<HTMLButtonElement>(".app-btn");
    if (btn) {
      btn.setAttribute("aria-pressed", String(isActive));
    }
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

  // Drag & Drop plików i folderów bezpośrednio do okna Antigravity
  await listen<{ paths?: string[] }>("tauri://drag-drop", (event) => {
    const chat = document.querySelector(".agy-chat");
    chat?.classList.remove("drag-over");
    if (
      currentState.expanded &&
      currentState.active_module === "antigravity" &&
      currentState.antigravity_running &&
      event.payload?.paths?.length
    ) {
      addAttachedPaths(event.payload.paths);
    }
  });
  await listen("tauri://drag-enter", () => {
    if (
      currentState.expanded &&
      currentState.active_module === "antigravity" &&
      currentState.antigravity_running
    ) {
      document.querySelector(".agy-chat")?.classList.add("drag-over");
    }
  });
  await listen("tauri://drag-leave", () => {
    document.querySelector(".agy-chat")?.classList.remove("drag-over");
  });

  const tab = el<HTMLElement>("tab");
  tab.addEventListener("click", () => setExpanded(true));
  tab.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") setExpanded(true);
  });
  el<HTMLButtonElement>("collapse").addEventListener("click", () => setExpanded(false));
  el<HTMLButtonElement>("toggle-large").addEventListener("click", () => {
    void invoke("set_large", { large: !currentState.large }).catch(console.error);
  });

  // Obsługa skrótów w rozwiniętym oknie: Escape (zwiń), Tab / Shift+Tab (zmiana aplikacji), Enter (uruchom wybraną wyłączoną aplikację)
  window.addEventListener(
    "keydown",
    (e) => {
      if (e.key === "Escape") {
        setExpanded(false);
        return;
      }
      if (!currentState.expanded) return;

      if (e.key === "Tab" && !e.ctrlKey && !e.altKey && !e.metaKey) {
        e.preventDefault();
        e.stopImmediatePropagation();
        void invoke("cycle_module", { delta: e.shiftKey ? -1 : 1 }).catch(console.error);
        return;
      }

      if (
        e.key === "Enter" &&
        !e.shiftKey &&
        !e.ctrlKey &&
        !e.altKey &&
        !e.metaKey &&
        currentState.active_module &&
        !isModuleRunning(currentState, currentState.active_module)
      ) {
        e.preventDefault();
        e.stopImmediatePropagation();
        void invoke("start_module", { moduleId: currentState.active_module }).catch(console.error);
      }
    },
    true
  );
}

void boot().catch(console.error);
