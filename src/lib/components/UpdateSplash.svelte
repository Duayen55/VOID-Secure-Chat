<script lang="ts">
  import { onMount } from 'svelte';
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { fade } from 'svelte/transition';

  export let onComplete: () => void;

  let status = 'Checking for updates...';
  let progress = 0;
  let hasUpdate = false;
  let error: string | null = null;
  let totalSize = 0;
  let downloaded = 0;

  onMount(async () => {
    // Artificial delay for splash screen feel (optional, but requested behavior "Splash screen göster")
    await new Promise(r => setTimeout(r, 1500));

    try {
      // @ts-ignore
      if (window.__TAURI_INTERNALS__) {
          await checkForUpdates();
      } else {
          // Browser mode fallback
          onComplete();
      }
    } catch (e) {
      console.error("Update check failed:", e);
      // In case of error (offline, etc), proceed to app after a brief message
      status = 'Offline mode ready.';
      setTimeout(onComplete, 1000);
    }
  });

  async function checkForUpdates() {
    status = 'Checking for updates...';
    
    try {
        const update = await check();
        
        if (update?.available) {
            hasUpdate = true;
            status = `Found version ${update.version}`;
            
            // Start download
            status = 'Downloading update...';
            let downloadedBytes = 0;
            let contentLength = 0;

            await update.downloadAndInstall((event) => {
                console.log("Update event:", event);
                switch (event.event) {
                    case 'Started':
                        contentLength = event.data.contentLength || 0;
                        status = `Downloading... (${(contentLength / 1024 / 1024).toFixed(2)} MB)`;
                        break;
                    case 'Progress':
                        downloadedBytes += event.data.chunkLength;
                        if (contentLength > 0) {
                            progress = (downloadedBytes / contentLength) * 100;
                            status = `Downloading... ${Math.round(progress)}%`;
                        } else {
                            status = `Downloading... ${(downloadedBytes / 1024 / 1024).toFixed(2)} MB`;
                        }
                        break;
                    case 'Finished':
                        status = 'Installing...';
                        progress = 100;
                        break;
                }
            });

            status = 'Restarting app...';
            await relaunch();
        } else {
            status = 'Up to date';
            setTimeout(onComplete, 800);
        }
    } catch (err) {
        console.error("Update error:", err);
        // Show error to user
        status = `Error: ${err}`;
        error = String(err);
        // Wait longer so user can see the error
        setTimeout(onComplete, 3000);
    }
  }
</script>

<div class="fixed inset-0 bg-black z-[9999] flex flex-col items-center justify-center select-none" in:fade out:fade>
  <!-- Background Ambient -->
  <div class="absolute inset-0 bg-[radial-gradient(circle_at_center,_var(--tw-gradient-stops))] from-gray-900 via-black to-black opacity-80"></div>

  <div class="relative z-10 flex flex-col items-center w-full max-w-md p-8">
      <!-- Logo -->
      <div class="mb-8 relative">
          <div class="w-24 h-24 bg-white rounded-full flex items-center justify-center shadow-[0_0_30px_rgba(255,255,255,0.2)] animate-pulse">
              <span class="text-black font-black text-3xl tracking-tighter">V</span>
          </div>
          <!-- Orbit Ring -->
          <div class="absolute inset-0 border-2 border-white/20 rounded-full animate-spin-slow" style="animation-duration: 3s;"></div>
      </div>

      <!-- Title -->
      <h1 class="text-3xl font-bold text-white mb-2 tracking-[0.2em]">VOID</h1>
      
      <!-- Status -->
      <div class="h-6 mb-4 flex items-center justify-center">
          <p class="text-gray-400 text-xs uppercase tracking-widest font-mono animate-pulse">{status}</p>
      </div>

      <!-- Progress Bar (Only visible if updating) -->
      {#if hasUpdate}
          <div class="w-full h-1 bg-gray-800 rounded-full overflow-hidden relative">
              <div class="h-full bg-white transition-all duration-200 ease-out shadow-[0_0_10px_white]" style="width: {progress}%"></div>
          </div>
          <div class="mt-2 w-full flex justify-between text-[10px] text-gray-500 font-mono">
              <span>{Math.round(progress)}%</span>
              <span>{hasUpdate ? 'DO NOT TURN OFF' : ''}</span>
          </div>
      {/if}
      
      <!-- Footer Version -->
      <div class="absolute bottom-4 text-gray-600 text-[10px] font-mono tracking-widest opacity-50">
          v2.1.1
      </div>
  </div>
</div>

<style>
  .animate-spin-slow {
      animation: spin 3s linear infinite;
  }
  @keyframes spin {
      from { transform: rotate(0deg); }
      to { transform: rotate(360deg); }
  }
</style>