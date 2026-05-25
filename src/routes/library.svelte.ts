import { open, message } from "@tauri-apps/plugin-dialog";
import { SvelteMap } from "svelte/reactivity";
import { TauriAPI, type ItemSummary, type ItemMedia } from "./api";

export type MediaStatus = "idle" | "loading" | "loaded" | "error";
export interface MediaEntry {
  status: MediaStatus;
  media?: ItemMedia;
}

/** 検索欄への属性チップ追加で許可するプレフィックス。 */
export type SearchPrefix =
  | "tag"
  | "artist"
  | "group"
  | "series"
  | "character"
  | "type"
  | "language";

class LibraryStore {
  query = $state("");
  searchResults = $state<ItemSummary[]>([]);
  isImporting = $state(false);

  // 表示モード（永続化しない・ウィンドウ毎に独立。起動毎にリストへリセット）
  viewMode = $state<"list" | "grid">("list");

  // 各アイテム（キー = item.id）のメディア取得状態。
  // SvelteMap を使い、.set()/.has()/.clear() の変更がリアクティブに伝播するようにする。
  mediaState = new SvelteMap<number, MediaEntry>();

  suggestions = $state<string[]>([]);
  showSuggestions = $state(false);
  activeSuggestionIndex = $state(0);

  async fetchSuggestions(prefix: string, keyword: string) {
    if (!prefix) {
      this.clearSuggestions();
      return;
    }
    try {
      this.suggestions = await TauriAPI.getSuggestions(prefix, keyword);
      this.showSuggestions = this.suggestions.length > 0;
      this.activeSuggestionIndex = 0;
    } catch (error) {
      console.error("Suggestion fetch failed:", error);
    }
  }

  clearSuggestions() {
    this.suggestions = [];
    this.showSuggestions = false;
    this.activeSuggestionIndex = 0;
  }

  async search() {
    if (!this.query.trim()) {
      this.searchResults = [];
      this.mediaState.clear();
      return;
    }
    try {
      // 新しい結果に切り替えるため取得状態をクリア（Rust側キャッシュは残るため再取得は高速）
      this.mediaState.clear();
      this.searchResults = await TauriAPI.searchItems(this.query);
    } catch (error) {
      console.error("Search failed:", error);
    }
  }

  /**
   * 可視アイテムのメディア（ページ数・サムネイル）を遅延取得する。
   * 既に取得開始済み（loading/loaded/error）のものは再取得しない（重複呼び出し防止）。
   */
  async loadItemMedia(item: ItemSummary) {
    if (this.mediaState.has(item.id)) return;

    this.mediaState.set(item.id, { status: "loading" });
    try {
      const media = await TauriAPI.getItemMedia(item.path);
      this.mediaState.set(item.id, { status: "loaded", media });
    } catch (error) {
      console.error("Media load failed:", error);
      this.mediaState.set(item.id, { status: "error" });
    }
  }

  /**
   * 属性チップのクリックで `prefix:値` を検索欄末尾に追記する。検索の実行はユーザー操作に委ねる。
   * 値に空白を含む場合は `"prefix:値"` のようにダブルクォートで囲む（既存のクォート規則に合わせる）。
   */
  appendToQuery(prefix: SearchPrefix, value: string) {
    const token = value.includes(" ")
      ? `"${prefix}:${value}"`
      : `${prefix}:${value}`;
    const trimmed = this.query.trimEnd();
    this.query = trimmed.length > 0 ? `${trimmed} ${token} ` : `${token} `;
  }

  async importDirectory() {
    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: "インポートするディレクトリを選択してください",
      });

      if (!selectedPath) return;

      this.isImporting = true;
      const count = await TauriAPI.scanAndImport(selectedPath);
      await message(`インポートが完了しました: ${count} 件`, {
        title: "インポート成功",
        kind: "info",
      });
    } catch (error) {
      console.error("Import failed:", error);
      await message(`インポート中にエラーが発生しました:\n${error}`, {
        title: "エラー",
        kind: "error",
      });
    } finally {
      this.isImporting = false;
    }
  }
}

export const library = new LibraryStore();
