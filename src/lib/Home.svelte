<script lang="ts">
  import * as dialog from "@tauri-apps/api/dialog";
  import { invoke } from "@tauri-apps/api";

  let baseWallpaperPath: string = "";
  let baseWallpaperUrl: string = "";
  let baseWallpaperType: "path" | "url" = "path";
  let isProcessing: boolean = false;

  async function changeBaseWallpaperPath() {
    isProcessing = true;
    await invoke("set_base_wallpaper_path", { imagePath: baseWallpaperPath });
    isProcessing = false;
  }

  async function changeBaseWallpaperUrl() {
    baseWallpaperType = "url";
    isProcessing = true;
    await invoke("set_base_wallpaper_url", { url: baseWallpaperUrl });
    isProcessing = false;
  }

  async function pickFile() {
    const result = await dialog.open({ multiple: false });
    if (result != null) {
      baseWallpaperPath = result.toString();
      await changeBaseWallpaperPath();
    }
  }

  async function drawTextToWallpaper() {
    await invoke("draw_text_to_base_wallpaper", { text: "Hello, World!!!!!" });
  }
</script>

<div>
  <button on:click={pickFile}>Select base wallpaper</button>
  <form class="row" on:submit|preventDefault={changeBaseWallpaperUrl}>
    <input id="greet-input" placeholder="Enter URL" type="text" bind:value={baseWallpaperUrl} />
    <button type="submit">Set base wallpaper from URL</button>
  </form>
  <button on:click={drawTextToWallpaper}>Draw text to base wallpaper</button>

  {#if isProcessing}
    <p>Processing...</p>
  {/if}

</div>