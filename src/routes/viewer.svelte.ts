import { convertFileSrc } from "@tauri-apps/api/core";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { TauriAPI } from "./api";

const collator = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: "base",
});

class ViewerStore {
  imagePaths = $state<string[]>([]);
  currentIndex = $state(0);
  private unlistenFns: UnlistenFn[] = [];

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
            this.imagePaths = e.payload.imagePaths;
            this.currentIndex = e.payload.currentIndex;
          },
        ),
        listen<{ currentIndex: number }>("sync-index", (e) => {
          this.currentIndex = e.payload.currentIndex;
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
        this.imagePaths = files.sort(collator.compare);
        this.currentIndex = 0;
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
    this.imagePaths = [];
    this.currentIndex = 0;
    this.syncState();
  }
}

export const viewer = new ViewerStore();
