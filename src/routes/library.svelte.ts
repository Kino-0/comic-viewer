import { open, message } from "@tauri-apps/plugin-dialog";
import { TauriAPI, type ItemSummary } from "./api";

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
      return;
    }
    try {
      this.searchResults = await TauriAPI.searchItems(this.query);
    } catch (error) {
      console.error("Search failed:", error);
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
