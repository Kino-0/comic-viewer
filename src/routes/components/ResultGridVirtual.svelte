<script lang="ts">
    import ResultItem from "./ResultItem.svelte";
    import type { ItemSummary } from "../api";

    let { items }: { items: ItemSummary[] } = $props();
</script>

<div class="viewport">
    <div class="grid">
        {#each items as item (item.id)}
            <div class="item-wrapper">
                <ResultItem {item} mode="grid" />
            </div>
        {/each}
    </div>
</div>

<style>
    .viewport {
        margin-top: 0.75rem;
        flex: 1;
        min-height: 0;
        overflow-y: auto;
        padding-right: 0.25rem;
    }
    .grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
        gap: 0.75rem;
        align-content: start;
    }
    .item-wrapper {
        content-visibility: auto;
        contain-intrinsic-size: auto 186px;
    }
</style>
