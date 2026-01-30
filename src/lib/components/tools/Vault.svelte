<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { appDataDir, join } from '@tauri-apps/api/path';
  import { listen } from '@tauri-apps/api/event';

  let pin = $state('');
  let files = $state<string[]>([]);
  let status = $state('');

  async function loadFiles() {
      try {
          files = await invoke('list_vault_files');
      } catch (e) {
          console.error(e);
      }
  }

  async function decrypt(filename: string) {
      if (!pin) {
          status = "ENTER PIN";
          return;
      }
      try {
          status = "DECRYPTING...";
          const appData = await appDataDir();
          const vaultPath = await join(appData, 'vault', filename);
          
          const bytes = await invoke('decrypt_file', { filePath: vaultPath, pin });
          
          const blob = new Blob([new Uint8Array(bytes as number[])]);
          const url = URL.createObjectURL(blob);
          
          const a = document.createElement('a');
          a.href = url;
          a.download = filename.replace('.void', '');
          a.click();
          URL.revokeObjectURL(url);
          status = "DECRYPTED";
      } catch (e) {
          status = "DECRYPT FAIL";
          console.error(e);
      }
  }

  onMount(() => {
      loadFiles();
      
      // Listen for file drops globally on the window
      const unlisten = listen('tauri://drag-drop', async (event: any) => {
           if (event.payload.paths && pin) {
               status = "ENCRYPTING...";
               for (const path of event.payload.paths) {
                   try {
                       await invoke('encrypt_file', { filePath: path, pin });
                   } catch (e) {
                       console.error(e);
                       status = "ERROR: " + e;
                   }
               }
               status = "SECURED";
               loadFiles();
           } else if (event.payload.paths && !pin) {
               status = "ENTER PIN TO SECURE";
           }
      });
      
      return () => {
          unlisten.then(f => f());
      }
  });
</script>

<div class="p-4 bg-black/40 backdrop-blur border border-white/10 rounded-lg h-full flex flex-col w-full">
  <h2 class="text-xs font-bold text-white/50 mb-4 tracking-widest">THE VAULT</h2>
  
  <input 
      type="password" 
      bind:value={pin}
      placeholder="PIN"
      class="w-full bg-black/50 border border-white/10 rounded p-2 text-center tracking-[0.5em] text-white focus:border-purple-500 outline-none mb-4 font-mono text-sm placeholder:tracking-normal placeholder:text-white/20"
  />
  
  <div class="flex-1 overflow-y-auto space-y-2 pr-1 custom-scrollbar">
      {#each files as file}
          <div class="flex justify-between items-center p-2 bg-white/5 rounded hover:bg-white/10 transition group border border-transparent hover:border-purple-500/30">
              <span class="text-xs font-mono text-white/70 truncate flex-1 mr-2" title={file}>{file}</span>
              <button onclick={() => decrypt(file)} class="text-[10px] bg-purple-900/50 hover:bg-purple-600 text-purple-200 px-2 py-1 rounded opacity-0 group-hover:opacity-100 transition whitespace-nowrap">
                  UNLOCK
              </button>
          </div>
      {/each}
      
      {#if files.length === 0}
          <div class="flex flex-col items-center justify-center h-20 text-white/20">
              <span class="text-xs">VAULT EMPTY</span>
          </div>
      {/if}
  </div>
  
  <div class="mt-4 text-center text-[10px] text-purple-400 font-mono h-4 truncate">
      {status}
  </div>
  
  <div class="mt-2 text-[10px] text-center text-white/30 border-t border-white/5 pt-2">
      DRAG FILES TO SECURE
  </div>
</div>

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: rgba(255, 255, 255, 0.05);
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.2);
    border-radius: 2px;
  }
</style>
