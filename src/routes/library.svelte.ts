import { open, message } from "@tauri-apps/plugin-dialog";
import { TauriAPI } from "./api";

class LibraryStore {
  query = $state("");
  searchResults = $state<string[]>([]);
  isImporting = $state(false);

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
