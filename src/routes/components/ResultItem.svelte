<script lang="ts">
    import { onMount, getContext } from "svelte";
    import { library } from "../library.svelte";
    import { viewer } from "../viewer.svelte";
    import type { ItemSummary } from "../api";
    import AttributeChip from "./AttributeChip.svelte";

    interface Props {
        item: ItemSummary;
        mode: "list" | "grid";
    }

    let { item, mode }: Props = $props();

    // 結果のスクロールコンテナを IntersectionObserver の root にするための getter（SearchScreen が提供）
    const getScrollRoot =
        getContext<() => HTMLElement | undefined>("resultScrollRoot");

    let rootEl: HTMLElement | undefined = $state();
    let visible = $state(false);

    onMount(() => {
        if (!rootEl) return;
        const observer = new IntersectionObserver(
            (entries) => {
                for (const entry of entries) {
                    if (entry.isIntersecting) {
                        visible = true; // 一度可視になれば維持（再スクロールで再取得不要）
                        observer.disconnect();
                        break;
                    }
                }
            },
            { root: getScrollRoot?.() ?? null, rootMargin: "200px" },
        );
        observer.observe(rootEl);
        return () => observer.disconnect();
    });

    // 可視かつ未取得なら遅延取得。再検索で mediaState がクリアされると再実行される。
    $effect(() => {
        if (visible && !library.mediaState.has(item.id)) {
            library.loadItemMedia(item);
        }
    });

    let entry = $derived(library.mediaState.get(item.id));
    let status = $derived(entry?.status ?? "idle");
    let thumbnail = $derived(entry?.media?.thumbnail ?? null);
    let pageCount = $derived(item.pageCount > 0 ? item.pageCount : undefined);

    let showThumb = $derived(status === "loaded" && thumbnail !== null);
    let showFallbackThumb = $derived(
        status === "error" || (status === "loaded" && thumbnail === null),
    );

    function openViewer() {
        // 欠損（path=null）はクリックしても何も開かない（既存動作のまま）
        if (item.path) viewer.load(item.path);
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            openViewer();
        }
    }
</script>

{#snippet thumb()}
    {#if showThumb}
        <img src={thumbnail} alt={item.title} draggable="false" />
    {:else if showFallbackThumb}
        <div class="placeholder">
            <svg viewBox="0 0 24 24" width="40" height="40" aria-hidden="true">
                <path
                    fill="currentColor"
                    d="M21 5v6.59l-3-3.01-4 4.01-4-4-4 4-3-3.01V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2zm-3 6.42 3 3.01V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-6.58l3 2.99 4-4 4 4 4-3.99z"
                />
            </svg>
        </div>
    {:else if status === "loading"}
        <div class="placeholder skeleton"></div>
    {:else}
        <!-- idle（未取得）はアニメーションを走らせず静的表示にし、画面外行のrepaintを抑える -->
        <div class="placeholder idle"></div>
    {/if}
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    bind:this={rootEl}
    class="card {mode}"
    role="button"
    tabindex="0"
    onclick={openViewer}
    onkeydown={handleKeydown}
>
    {#if mode === "list"}
        <div class="thumb-box list-thumb">{@render thumb()}</div>
        <div class="meta">
            <div class="title">{item.title}</div>
            <div class="chips">
                {#each item.artists as a}
                    <AttributeChip prefix="artist" value={a} />
                {/each}
                {#each item.groups as g}
                    <AttributeChip prefix="group" value={g} />
                {/each}
                {#each item.series as s}
                    <AttributeChip prefix="series" value={s} />
                {/each}
                {#each item.characters as c}
                    <AttributeChip prefix="character" value={c} />
                {/each}
                {#if item.typeName}
                    <AttributeChip prefix="type" value={item.typeName} />
                {/if}
                {#if item.language}
                    <AttributeChip prefix="language" value={item.language} />
                {/if}
                {#each item.tags as t}
                    <AttributeChip prefix="tag" value={t} />
                {/each}
            </div>
            <div class="footer">
                <span class="gallery-id">ID: {item.id}</span>
                {#if pageCount !== undefined}
                    <span class="page-count">{pageCount} ページ</span>
                {/if}
            </div>
        </div>
    {:else}
        <div class="thumb-box grid-thumb">{@render thumb()}</div>
        <div class="overlay">
            <span class="overlay-title">{item.title}</span>
        </div>
    {/if}
</div>

<style>
    .card {
        cursor: pointer;
        color: white;
        background: #262626;
        border: 1px solid #333;
        overflow: hidden;
    }
    .card:focus-visible {
        outline: 2px solid #396cd8;
        outline-offset: -2px;
    }

    /* サムネイル共通 */
    .thumb-box {
        position: relative;
        background: #1a1a1a;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
        overflow: hidden;
    }
    .thumb-box img {
        width: 100%;
        height: 100%;
        object-fit: contain; /* 全体表示・レターボックス */
        user-select: none;
    }
    .placeholder {
        width: 100%;
        height: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
        color: #555;
    }
    .placeholder.idle {
        background: #222; /* 未取得行は静的（アニメーションなし） */
    }
    .skeleton {
        background: linear-gradient(90deg, #2a2a2a 25%, #383838 50%, #2a2a2a 75%);
        background-size: 200% 100%;
        animation: shimmer 1.2s infinite;
    }
    @keyframes shimmer {
        0% {
            background-position: 200% 0;
        }
        100% {
            background-position: -200% 0;
        }
    }

    /* ===== リスト表示 ===== */
    /* 行間は仮想化側（ResultListVirtual）が offset で確保するため margin は持たない。 */
    .card.list {
        display: flex;
        gap: 1rem;
        padding: 0.75rem;
        border-radius: 8px;
        align-items: flex-start;
    }
    .card.list:hover {
        background: #2f2f2f;
        border-color: #444;
    }
    .list-thumb {
        width: 100px;
        height: 140px;
        border-radius: 4px;
    }
    .meta {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
    }
    .title {
        font-size: 1.05rem;
        font-weight: 600;
        line-height: 1.3;
        word-break: break-word;
    }
    .chips {
        display: flex;
        flex-wrap: wrap; /* タグが多い場合は全件折り返し */
        gap: 0.3rem;
    }
    .footer {
        display: flex;
        gap: 1rem;
        font-size: 0.8rem;
        color: #999;
        margin-top: 0.1rem;
    }

    /* ===== グリッド表示 ===== */
    /* aspect-ratio はグリッドアイテム自身ではなく内側のサムネ枠に持たせる。
       （アイテムに直接付けると WebKitGTK で行トラック高が確保されず上下が重なるため） */
    .card.grid {
        position: relative;
        border-radius: 6px;
        min-height: 0;
    }
    .grid-thumb {
        width: 100%;
        aspect-ratio: 3 / 4;
    }
    .grid-thumb img,
    .grid .placeholder {
        transition:
            filter 0.2s,
            transform 0.2s;
    }
    .overlay {
        position: absolute;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 0.5rem;
        opacity: 0;
        background: rgba(0, 0, 0, 0.45);
        transition: opacity 0.2s;
        pointer-events: none;
    }
    .overlay-title {
        color: white;
        font-size: 0.9rem;
        font-weight: 600;
        text-align: center;
        line-height: 1.3;
        /* 長いタイトルは省略 */
        display: -webkit-box;
        -webkit-line-clamp: 4;
        line-clamp: 4;
        -webkit-box-orient: vertical;
        overflow: hidden;
        text-shadow: 0 1px 3px rgba(0, 0, 0, 0.8);
    }
    .card.grid:hover .overlay,
    .card.grid:focus-visible .overlay {
        opacity: 1;
    }
    .card.grid:hover .grid-thumb img,
    .card.grid:focus-visible .grid-thumb img {
        filter: brightness(0.4) blur(2px);
    }
</style>
