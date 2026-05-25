<script lang="ts">
    import { onMount, setContext } from "svelte";
    import ResultItem from "./ResultItem.svelte";
    import type { ItemSummary } from "../api";

    let { items }: { items: ItemSummary[] } = $props();

    // heights[i] は padding-bottom(=GAP) を含む border-box 高さで測定するため ROW_HEIGHT_ESTIMATE もそれに合わせる。
    const ROW_HEIGHT_ESTIMATE = 188; // 未測定行の推定スロット高さ(px): カード~180 + ギャップ8
    const GAP = 8; // 行間(px) — .row の padding-bottom として適用
    const BUFFER = 6; // 可視範囲の上下に余分に描画する行数

    let viewport: HTMLElement | undefined = $state();
    let viewportHeight = $state(600);
    let scrollTop = $state(0);

    let heights = $state<number[]>([]);
    let prevItems: ItemSummary[] | undefined = $state();

    $effect(() => {
        if (items !== prevItems) {
            prevItems = items;
            heights = new Array(items.length).fill(ROW_HEIGHT_ESTIMATE);
            scrollTop = 0;
            if (viewport) viewport.scrollTop = 0;
        }
    });

    // heights[i] は gap 込みのスロット高さなので + GAP は不要。
    let offsets = $derived.by(() => {
        const arr = new Array<number>(items.length + 1);
        arr[0] = 0;
        for (let i = 0; i < items.length; i++) {
            arr[i + 1] = arr[i] + (heights[i] ?? ROW_HEIGHT_ESTIMATE);
        }
        return arr;
    });
    let totalHeight = $derived(offsets[items.length] ?? 0);

    let range = $derived.by(() => {
        const top = scrollTop;
        const bottom = scrollTop + viewportHeight;
        let start = 0;
        while (start < items.length && offsets[start + 1] <= top) start++;
        let end = start;
        while (end < items.length && offsets[end] < bottom) end++;
        return {
            start: Math.max(0, start - BUFFER),
            end: Math.min(items.length, end + BUFFER),
        };
    });

    let visibleItems = $derived(
        items.slice(range.start, range.end).map((item, i) => ({
            item,
            index: range.start + i,
        })),
    );

    let topFillerHeight = $derived(offsets[range.start] ?? 0);
    let bottomFillerHeight = $derived(
        Math.max(0, totalHeight - (offsets[range.end] ?? totalHeight)),
    );

    function onScroll() {
        if (viewport) scrollTop = viewport.scrollTop;
    }

    onMount(() => {
        if (!viewport) return;
        viewportHeight = viewport.clientHeight;
        const ro = new ResizeObserver(() => {
            if (viewport) viewportHeight = viewport.clientHeight;
        });
        ro.observe(viewport);
        return () => ro.disconnect();
    });

    // ResizeObserver のコールバック引数（borderBoxSize）を使うことで、
    // offsetHeight の同期読み取りによる強制レイアウト（forced reflow）を回避する。
    // border-box で観測するため padding-bottom(GAP) を含むスロット高さが得られる。
    function observeRowHeight(node: HTMLElement, index: number) {
        let current = index;
        const ro = new ResizeObserver((entries) => {
            const entry = entries[0];
            const h =
                entry?.borderBoxSize?.[0]?.blockSize ?? entry?.contentRect.height;
            if (h && h > 0 && heights[current] !== h) {
                heights[current] = h;
            }
        });
        ro.observe(node, { box: "border-box" });
        return {
            update(newIndex: number) {
                current = newIndex;
            },
            destroy() {
                ro.disconnect();
            },
        };
    }

    setContext("resultScrollRoot", () => viewport);
</script>

<div class="viewport" bind:this={viewport} onscroll={onScroll}>
    <!-- 可視範囲より上のアイテムをスペーサーで確保（transform は使わない） -->
    <div style="height: {topFillerHeight}px;" aria-hidden="true"></div>
    {#each visibleItems as { item, index } (item.id)}
        <div class="row" use:observeRowHeight={index}>
            <ResultItem {item} mode="list" />
        </div>
    {/each}
    <!-- 可視範囲より下のアイテムをスペーサーで確保 -->
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
    .row {
        /* padding-bottom が gap の役割を兼ねる。border-box 測定でこの分も heights に含まれる。 */
        padding-bottom: 8px;
        contain: layout;
    }
</style>
