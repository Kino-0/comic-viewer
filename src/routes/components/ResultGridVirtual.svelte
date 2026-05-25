<script lang="ts">
    import { onMount, setContext } from "svelte";
    import ResultItem from "./ResultItem.svelte";
    import type { ItemSummary } from "../api";

    let { items }: { items: ItemSummary[] } = $props();

    // グリッド CSS の minmax(140px, 1fr) と gap: 0.75rem に対応
    const MIN_ITEM_WIDTH = 140;
    const GAP = 12; // 0.75rem
    const BUFFER = 2; // 可視範囲の上下に余分に描画する行数

    let viewport: HTMLElement | undefined = $state();
    let viewportWidth = $state(800);
    let viewportHeight = $state(600);
    let scrollTop = $state(0);

    let prevItems: ItemSummary[] | undefined = $state();

    $effect(() => {
        if (items !== prevItems) {
            prevItems = items;
            scrollTop = 0;
            if (viewport) viewport.scrollTop = 0;
        }
    });

    // コンテナ幅から列数を算出（CSS auto-fill と同じロジック）
    let columns = $derived(
        Math.max(1, Math.floor((viewportWidth + GAP) / (MIN_ITEM_WIDTH + GAP))),
    );

    // アイテム幅 → aspect-ratio: 3/4 からサムネイル高 → それが行高
    let rowHeight = $derived(
        Math.ceil((viewportWidth - (columns - 1) * GAP) / columns * (4 / 3)),
    );

    // items を columns 個ずつ行に分割
    let rows = $derived.by(() => {
        const result: ItemSummary[][] = [];
        for (let i = 0; i < items.length; i += columns) {
            result.push(items.slice(i, i + columns));
        }
        return result;
    });

    let totalHeight = $derived(rows.length * rowHeight);

    let range = $derived.by(() => {
        const start = Math.max(
            0,
            Math.floor(scrollTop / rowHeight) - BUFFER,
        );
        const end = Math.min(
            rows.length,
            Math.ceil((scrollTop + viewportHeight) / rowHeight) + BUFFER,
        );
        return { start, end };
    });

    let visibleRows = $derived(rows.slice(range.start, range.end));

    let topFillerHeight = $derived(range.start * rowHeight);
    let bottomFillerHeight = $derived(
        Math.max(0, totalHeight - range.end * rowHeight),
    );

    function onScroll() {
        if (viewport) scrollTop = viewport.scrollTop;
    }

    onMount(() => {
        if (!viewport) return;
        viewportWidth = viewport.clientWidth;
        viewportHeight = viewport.clientHeight;
        const ro = new ResizeObserver(() => {
            if (viewport) {
                viewportWidth = viewport.clientWidth;
                viewportHeight = viewport.clientHeight;
            }
        });
        ro.observe(viewport);
        return () => ro.disconnect();
    });

    setContext("resultScrollRoot", () => viewport);
</script>

<div class="viewport" bind:this={viewport} onscroll={onScroll}>
    <div style="height: {topFillerHeight}px;" aria-hidden="true"></div>
    <div class="grid">
        {#each visibleRows as row}
            {#each row as item (item.id)}
                <ResultItem {item} mode="grid" />
            {/each}
        {/each}
    </div>
    <div style="height: {bottomFillerHeight}px;" aria-hidden="true"></div>
</div>

<style>
    .viewport {
        margin-top: 0.75rem;
        flex: 1;
        min-height: 0;
        overflow-y: auto;
        padding-right: 0.25rem;
        contain: layout paint style;
    }
    .grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
        gap: 0.75rem;
        align-content: start;
    }
</style>
