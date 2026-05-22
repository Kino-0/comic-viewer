import { convertFileSrc } from "@tauri-apps/api/core";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { TauriAPI } from "./api";

const collator = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: "base",
});

const PRELOAD_PREV = 1;
const PRELOAD_NEXT = 2;

class ViewerStore {
  imagePaths = $state<string[]>([]);
  currentIndex = $state(0);
  private unlistenFns: UnlistenFn[] = [];
  private imageCache = new Map<number, HTMLImageElement>();
  private pendingIndices = new Set<number>();

  get currentImage() {
    return this.imagePaths.length > 0
      ? convertFileSrc(this.imagePaths[this.currentIndex])
      : null;
  }

  get isViewing() {
    return this.imagePaths.length > 0;
  }

  async initListeners() {
    if (this.unlistenFns.length > 0) return;

    try {
      this.unlistenFns = await Promise.all([
        listen<{ imagePaths: string[]; currentIndex: number }>(
          "sync-state",
          (e) => {
            this.clearCache();
            this.imagePaths = e.payload.imagePaths;
            this.currentIndex = e.payload.currentIndex;
            this.updateCache();
          },
        ),
        listen<{ currentIndex: number }>("sync-index", (e) => {
          this.currentIndex = e.payload.currentIndex;
          this.updateCache();
        }),
        listen("request-sync", () => {
          if (this.imagePaths.length > 0) this.syncState();
        }),
      ]);
      emit("request-sync");
    } catch (error) {
      console.error("[Fatal] イベントリスナーの登録に失敗しました:", error);
    }
  }

  destroyListeners() {
    this.unlistenFns.forEach((unlisten) => unlisten());
    this.unlistenFns = [];
  }

  private syncState() {
    emit("sync-state", {
      imagePaths: $state.snapshot(this.imagePaths),
      currentIndex: this.currentIndex,
    });
  }

  private syncIndex() {
    emit("sync-index", { currentIndex: this.currentIndex });
  }

  async load(dirPath: string) {
    try {
      const files = await TauriAPI.getImagesInDir(dirPath);
      if (files.length > 0) {
        this.clearCache();
        this.imagePaths = files.sort(collator.compare);
        this.currentIndex = 0;
        this.updateCache();
        this.syncState();
      } else {
        console.warn("このディレクトリには画像がありません");
      }
    } catch (error) {
      console.error("画像の読み込みに失敗しました:", error);
    }
  }

  private navigate(offset: number) {
    const newIndex = Math.max(
      0,
      Math.min(this.currentIndex + offset, this.imagePaths.length - 1),
    );
    if (newIndex !== this.currentIndex) {
      this.currentIndex = newIndex;
      this.updateCache();
      this.syncIndex();
    }
  }

  next() {
    this.navigate(1);
  }

  prev() {
    this.navigate(-1);
  }

  close() {
    this.clearCache();
    this.imagePaths = [];
    this.currentIndex = 0;
    this.syncState();
  }

  private updateCache() {
    const keepIndices = this.keepIndices();
    this.evict(keepIndices);
    for (const idx of keepIndices) {
      this.preloadIndex(idx);
    }
  }

  private keepIndices(): Set<number> {
    const keep = new Set<number>();
    for (let i = -PRELOAD_PREV; i <= PRELOAD_NEXT; i++) {
      const idx = this.currentIndex + i;
      if (idx >= 0 && idx < this.imagePaths.length) {
        keep.add(idx);
      }
    }
    return keep;
  }

  private evict(keepIndices: Set<number>) {
    for (const [idx, img] of this.imageCache) {
      if (!keepIndices.has(idx)) {
        img.src = "";
        this.imageCache.delete(idx);
      }
    }
    // pending中でも不要になったものをキャンセル
    for (const idx of this.pendingIndices) {
      if (!keepIndices.has(idx)) {
        this.pendingIndices.delete(idx);
      }
    }
  }

  private async preloadIndex(index: number) {
    if (this.imageCache.has(index) || this.pendingIndices.has(index)) return;
    this.pendingIndices.add(index);
    const img = new Image();
    img.src = convertFileSrc(this.imagePaths[index]);
    try {
      await img.decode();
      // evict によってキャンセルされていなければキャッシュに追加
      if (this.pendingIndices.has(index)) {
        this.imageCache.set(index, img);
      } else {
        img.src = "";
      }
    } catch {
      img.src = "";
    } finally {
      this.pendingIndices.delete(index);
    }
  }

  private clearCache() {
    for (const img of this.imageCache.values()) {
      img.src = "";
    }
    this.imageCache.clear();
    this.pendingIndices.clear();
  }
}

export const viewer = new ViewerStore();
