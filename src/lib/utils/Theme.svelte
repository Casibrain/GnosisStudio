<script>
    import { prefs } from "$lib/preferences.svelte";
    import { electron } from "$lib/globals.svelte";
    import { asset } from "$app/paths";

    let url = $derived.by(() => {
        if (prefs.params.theme.val === "custom") {
            return asset(`/${prefs.params.customTheme.val}`)
        } else {
            return asset(`/themes/${prefs.params.theme.val}.css`)
        }
    })
</script>

<svelte:head>
    {#if prefs.params.theme.val === "custom"}
        <!-- load theme from file if given one -->
        {#if electron}
            {#await electron.files.load(prefs.params.customTheme.val) then styling}
                {@html `<style type='text/css'>\n${styling}\n</style>`}
            {:catch err}
                <!-- fallback to psychopy.css if we can't load the custom file -->
                <link 
                    rel="stylesheet" 
                    href={asset("/themes/psychopy.css")}
                >
            {/await}
        {:else}
            <!-- fallback to psychopy.css if in web view (which can't access files by path) -->
            <link 
                rel="stylesheet" 
                href={asset("/themes/psychopy.css")}
            >
        {/if}
    {:else}
        <link 
            rel="stylesheet" 
            href={url}
            onerror={
                // fallback to psychopy.css if theme doesn't load
                evt => evt.target.href = asset("/themes/psychopy.css")
            }
        >
    {/if}
</svelte:head>