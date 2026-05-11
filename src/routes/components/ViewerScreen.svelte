<script lang="ts">
    import { viewer } from "../viewer.svelte";

    function handleKeydown(event: KeyboardEvent) {
        switch (event.key) {
            case "ArrowLeft":
                return viewer.next();
            case "ArrowRight":
                return viewer.prev();
            case "Escape":
                return viewer.close();
        }
    }

    function handleMouseClick(event: MouseEvent) {
        switch (event.button) {
            case 0:
                return viewer.next(); // 左クリック
            case 2:
                return viewer.prev(); // 右クリック
        }
    }
</script>

<svelte:window onkeydown={handleKeydown} />

<svelte:head>
    {#each viewer.preloadImages as src (src)}
        <link rel="preload" as="image" href={src} />
    {/each}
</svelte:head>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="viewer"
    onmousedown={handleMouseClick}
    oncontextmenu={(e) => e.preventDefault()}
>
    {#if viewer.currentImage}
        <img src={viewer.currentImage} alt="Comic page" />
    {/if}
    <div class="info">Press Esc to return to search</div>
</div>

<style>
    .viewer {
        width: 100%;
        height: 100%;
        display: flex;
        justify-content: center;
        align-items: center;
        background: black;
        position: relative;
    }
    img {
        height: 100%;
        width: 100%;
        object-fit: contain;
        user-select: none;
    }
    .info {
        position: absolute;
        top: 10px;
        color: oklch(50% 0 0 / 30%);
        font-size: 0.8rem;
        pointer-events: none;
    }
</style>
