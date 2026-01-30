<script lang="ts">
  import { onMount } from 'svelte';
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  let available = $state<{ version: string } | null>(null);
  let pendingUpdate: any | null = $state(null);
  async function checkForUpdates() {
    const update = await check();
    if (update) {
      available = { version: update.version };
      pendingUpdate = update;
    }
  }
  onMount(() => {
    checkForUpdates().catch(() => {});
  });
  export async function installUpdate() {
    if (pendingUpdate) {
      await pendingUpdate.downloadAndInstall();
      await relaunch();
    }
  }
</script>

{#if available}
  <div class="fixed top-3 right-3 text-xs bg-purple-600 text-white px-2 py-1 rounded shadow">
    Update Available {available.version}
  </div>
{/if}
