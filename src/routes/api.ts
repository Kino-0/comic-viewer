import { invoke } from "@tauri-apps/api/core";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export interface ItemSummary {
  id: number;
  title: string;
  path: string | null;
  pageCount: number;
  typeName: string;
  language: string | null;
  artists: string[];
  groups: string[];
  series: string[];
  characters: string[];
  tags: string[];
  coverPath: string | null;
}

export const TauriAPI = {
  async scanAndImport(path: string): Promise<number> {
    return invoke<number>("scan_and_import", { path });
  },
  async getSuggestions(prefix: string, keyword: string): Promise<string[]> {
    return invoke<string[]>("get_suggestions", { prefix, keyword });
  },
  async searchItems(query: string): Promise<ItemSummary[]> {
    return invoke<ItemSummary[]>("search_items", { query });
  },
  async getImagesInDir(path: string): Promise<string[]> {
    return invoke<string[]>("get_images_in_dir", { path });
  },
  async openMirrorWindow() {
    const label = `mirror-${Date.now()}`;
    const mirror = new WebviewWindow(label, {
      url: "/",
      title: "Comic Viewer",
      width: 800,
      height: 600,
    });
    mirror.once("tauri://error", (e) => {
      console.error("ウィンドウの作成に失敗しました:", e);
    });
  },
};
