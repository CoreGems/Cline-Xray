/**
 * Stash Store — persists "best answers" from model chat responses to localStorage
 *
 * Each stashed answer stores the markdown content, a preview snippet,
 * the model name, session title, and timestamps.
 * Provides reactive state via Svelte 5 runes.
 */

const STORAGE_KEY = 'agent-chat-stashed-answers';

export interface StashedAnswer {
  /** Unique ID */
  id: string;
  /** Full markdown content of the model answer */
  content: string;
  /** First ~120 chars for preview */
  preview: string;
  /** The user question that led to this answer (if available) */
  userQuestion?: string;
  /** Model name at time of stash */
  model: string;
  /** Session title at time of stash */
  sessionTitle: string;
  /** Optional user-supplied note/label */
  note: string;
  /** Tags for categorization */
  tags: string[];
  /** When it was stashed */
  stashedAt: number;
}

function generateId(): string {
  return `stash-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
}

function makePreview(content: string): string {
  // Strip markdown syntax for a clean preview
  const plain = content
    .replace(/```[\s\S]*?```/g, '[code]')
    .replace(/`[^`]+`/g, '[code]')
    .replace(/[#*_~>\-|]/g, '')
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/\n+/g, ' ')
    .trim();
  return plain.length > 120 ? plain.substring(0, 120) + '…' : plain;
}

function loadStash(): StashedAnswer[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) return JSON.parse(stored) as StashedAnswer[];
  } catch { /* ignore corrupt data */ }
  return [];
}

function saveStash(items: StashedAnswer[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(items));
  } catch (e) {
    console.error('Failed to save stash:', e);
  }
}

// ── Reactive State ──────────────────────────────────────────────────────────

let _items = $state<StashedAnswer[]>(loadStash());
let _searchQuery = $state('');
let _activeTagFilter = $state<string | null>(null);

// ── Derived ─────────────────────────────────────────────────────────────────

/** All unique tags across all stashed items, sorted alphabetically */
let _allTags = $derived.by(() => {
  const tagSet = new Set<string>();
  for (const item of _items) {
    if (item.tags) for (const t of item.tags) tagSet.add(t);
  }
  return [...tagSet].sort((a, b) => a.localeCompare(b));
});

let _filtered = $derived.by(() => {
  let result = _items;
  // Tag filter
  if (_activeTagFilter) {
    const tag = _activeTagFilter;
    result = result.filter(item => item.tags && item.tags.includes(tag));
  }
  // Text search
  if (_searchQuery.trim()) {
    const q = _searchQuery.toLowerCase();
    result = result.filter(
      item =>
        item.content.toLowerCase().includes(q) ||
        item.note.toLowerCase().includes(q) ||
        item.model.toLowerCase().includes(q) ||
        item.sessionTitle.toLowerCase().includes(q) ||
        (item.tags && item.tags.some(t => t.toLowerCase().includes(q)))
    );
  }
  return result;
});

export interface SessionGroup {
  sessionTitle: string;
  items: StashedAnswer[];
}

/** Group filtered items by sessionTitle, preserving order of first appearance */
let _groupedBySession = $derived.by(() => {
  const groups: SessionGroup[] = [];
  const map = new Map<string, SessionGroup>();
  for (const item of _filtered) {
    let group = map.get(item.sessionTitle);
    if (!group) {
      group = { sessionTitle: item.sessionTitle, items: [] };
      map.set(item.sessionTitle, group);
      groups.push(group);
    }
    group.items.push(item);
  }
  return groups;
});

export const stashStore = {
  // ── Getters ───────────────────────────────────────────────────────────────

  get items(): StashedAnswer[] { return _items; },
  get filteredItems(): StashedAnswer[] { return _filtered; },
  get groupedBySession(): SessionGroup[] { return _groupedBySession; },
  get count(): number { return _items.length; },
  get searchQuery(): string { return _searchQuery; },
  set searchQuery(v: string) { _searchQuery = v; },
  get activeTagFilter(): string | null { return _activeTagFilter; },
  set activeTagFilter(v: string | null) { _activeTagFilter = v; },
  get allTags(): string[] { return _allTags; },

  // ── Queries ───────────────────────────────────────────────────────────────

  /** Check if a specific content string is already stashed */
  isStashed(content: string): boolean {
    return _items.some(item => item.content === content);
  },

  /** Find stash entry by content (for toggle) */
  findByContent(content: string): StashedAnswer | undefined {
    return _items.find(item => item.content === content);
  },

  // ── Mutations ─────────────────────────────────────────────────────────────

  /** Stash a model answer */
  stash(content: string, model: string, sessionTitle: string, userQuestion?: string): StashedAnswer {
    const entry: StashedAnswer = {
      id: generateId(),
      content,
      preview: makePreview(content),
      userQuestion: userQuestion || undefined,
      model,
      sessionTitle,
      note: '',
      tags: [],
      stashedAt: Date.now(),
    };
    _items = [entry, ..._items];
    saveStash(_items);
    return entry;
  },

  /** Remove a stashed answer by ID */
  unstash(id: string) {
    _items = _items.filter(item => item.id !== id);
    saveStash(_items);
  },

  /** Remove a stashed answer by content (for toggle from chat bubble) */
  unstashByContent(content: string) {
    _items = _items.filter(item => item.content !== content);
    saveStash(_items);
  },

  /** Toggle stash: stash if not stashed, unstash if already stashed */
  toggle(content: string, model: string, sessionTitle: string, userQuestion?: string): boolean {
    if (this.isStashed(content)) {
      this.unstashByContent(content);
      return false; // now unstashed
    } else {
      this.stash(content, model, sessionTitle, userQuestion);
      return true; // now stashed
    }
  },

  /** Update the note/label for a stashed answer */
  updateNote(id: string, note: string) {
    const item = _items.find(i => i.id === id);
    if (item) {
      item.note = note;
      _items = [..._items]; // trigger reactivity
      saveStash(_items);
    }
  },

  /** Add a tag to a stashed answer (preserves original casing, deduplicates case-insensitively) */
  addTag(id: string, tag: string) {
    const item = _items.find(i => i.id === id);
    if (item) {
      if (!item.tags) item.tags = [];
      const trimmed = tag.trim();
      if (trimmed && !item.tags.some(t => t.toLowerCase() === trimmed.toLowerCase())) {
        item.tags = [...item.tags, trimmed];
        _items = [..._items];
        saveStash(_items);
      }
    }
  },

  /** Remove a tag from a stashed answer */
  removeTag(id: string, tag: string) {
    const item = _items.find(i => i.id === id);
    if (item && item.tags) {
      item.tags = item.tags.filter(t => t !== tag);
      _items = [..._items];
      saveStash(_items);
    }
  },

  /** Clear all stashed answers */
  clearAll() {
    _items = [];
    saveStash(_items);
  },
};
