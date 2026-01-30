<script lang="ts">
  import Canvas from '$lib/components/Canvas.svelte';
  import SyncPlayer from '$lib/components/media/SyncPlayer.svelte';
  import StickyBoard from '$lib/components/tools/StickyBoard.svelte';
  import Vault from '$lib/components/tools/Vault.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';

  let status = $state('OFFLINE');
  let logs: string[] = $state([]);
  let voidCode = $state('');
  let targetCode = $state('');
  let activeTab = $state('HOME');
  let connectedPeerId = $state('');

  async function initNode() {
    try {
      status = 'INITIALIZING...';
      const res = await invoke('start_node');
      logs = [...logs, `System: ${res}`];
      status = 'ONLINE';
      
      setTimeout(async () => {
        try {
          voidCode = await invoke('get_my_void_code');
        } catch (e) {
          console.error(e);
        }
      }, 2000);

    } catch (e) {
      status = 'ERROR';
      logs = [...logs, `Error: ${e}`];
    }
  }
  
  function extractPeerId(code: string): string {
      try {
          if (!code) return '';
          const decoded = atob(code);
          // Format: void://<peer_id>@...
          return decoded.split('@')[0].replace('void://', '');
      } catch (e) { return ''; }
  }

  async function joinPeer() {
    if (!targetCode) return;
    try {
      await invoke('connect_via_code', { code: targetCode });
      logs = [...logs, `Dialing peer...`];
      connectedPeerId = extractPeerId(targetCode);
      targetCode = '';
    } catch (e) {
      logs = [...logs, `Dial Error: ${e}`];
    }
  }

  async function copyCode() {
    if (!voidCode) return;
    await navigator.clipboard.writeText(voidCode);
    logs = [...logs, 'Void Code copied to clipboard'];
  }

  onMount(() => {
    listen('network-event', (event: any) => {
      logs = [...logs, `> ${event.payload}`].slice(-5);
    });
  });
</script>

<div class="flex h-screen bg-black text-white font-sans overflow-hidden">
  <!-- Sidebar -->
  <aside class="w-16 border-r border-white/10 flex flex-col items-center py-4 bg-black z-20">
    <div class="mb-8 font-bold text-xl tracking-tighter select-none">V</div>
    
    <button 
      onclick={() => activeTab = 'HOME'}
      class={`w-10 h-10 mb-4 rounded-lg flex items-center justify-center transition-all ${activeTab === 'HOME' ? 'bg-white text-black' : 'text-white/50 hover:text-white hover:bg-white/10'}`}
      title="Home"
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>
    </button>
    
    <button 
      onclick={() => activeTab = 'WORKSPACE'}
      class={`w-10 h-10 mb-4 rounded-lg flex items-center justify-center transition-all ${activeTab === 'WORKSPACE' ? 'bg-purple-600 text-white shadow-[0_0_15px_#9333ea]' : 'text-white/50 hover:text-white hover:bg-white/10'}`}
      title="Workspace"
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="7" height="9" x="3" y="3" rx="1"/><rect width="7" height="5" x="14" y="3" rx="1"/><rect width="7" height="9" x="14" y="12" rx="1"/><rect width="7" height="5" x="3" y="16" rx="1"/></svg>
    </button>
  </aside>

  <div class="flex-1 flex flex-col h-full relative">
      <!-- Top Bar -->
      <header data-tauri-drag-region class="h-10 border-b border-white/10 flex items-center px-4 justify-between bg-black/80 backdrop-blur z-50">
        <div class="font-bold tracking-widest text-sm select-none">VOID /// PROJECT</div>
        <div class="flex gap-4 text-xs text-white/50">
          <span class={status === 'ONLINE' ? 'text-green-500' : 'text-red-500'}>NET: {status}</span>
          <span>PEER: {connectedPeerId ? connectedPeerId.substring(0, 8) + '...' : 'NONE'}</span>
          <span>AI: IDLE</span>
        </div>
      </header>

      <main class="flex-1 relative overflow-hidden bg-black">
          {#if activeTab === 'HOME'}
            <Canvas>
              <div class="p-20 pointer-events-auto h-full overflow-y-auto">
                <h1 class="text-6xl font-black mb-8 tracking-tighter mix-blend-difference select-none">VOID NETWORK</h1>
                
                <div class="grid grid-cols-1 md:grid-cols-2 gap-8 max-w-4xl">
                  <!-- Control Panel -->
                  <div class="bg-white/5 border border-white/10 p-6 backdrop-blur-md">
                    <h2 class="text-xl font-bold mb-4 border-b border-white/10 pb-2">IDENTITY</h2>
                    
                    {#if status === 'OFFLINE' || status === 'ERROR'}
                      <button 
                        onclick={initNode}
                        class="w-full bg-white text-black py-3 font-bold hover:bg-gray-200 transition-colors mb-4"
                      >
                        INITIALIZE NODE
                      </button>
                    {:else}
                       <div class="mb-4">
                         <label class="text-xs text-gray-500 block mb-1">YOUR VOID ID</label>
                         <div class="flex gap-2">
                           <div class="flex-1 bg-black/50 border border-white/10 p-2 text-xs font-mono break-all h-20 overflow-y-auto custom-scrollbar">
                             {voidCode || 'Generating Identity...'}
                           </div>
                           <button 
                             onclick={copyCode}
                             class="px-4 border border-white/20 hover:bg-white/10 text-xs"
                             disabled={!voidCode}
                           >
                             COPY
                           </button>
                         </div>
                       </div>
                    {/if}

                    <div class="mt-8">
                      <h2 class="text-xl font-bold mb-4 border-b border-white/10 pb-2">CONNECTION</h2>
                      <div class="flex gap-2">
                        <input 
                          class="flex-1 bg-black border border-white/20 p-2 text-sm focus:border-white outline-none"
                          placeholder="Paste Void Code (void://...)" 
                          bind:value={targetCode} 
                        />
                        <button 
                          onclick={joinPeer}
                          class="bg-white text-black px-4 py-2 text-sm font-bold hover:bg-gray-200 transition-colors"
                        >
                          JOIN
                        </button>
                      </div>
                    </div>
                  </div>

                  <!-- Logs -->
                  <div class="bg-black/50 border border-white/10 p-6 backdrop-blur-md font-mono text-xs">
                    <h2 class="text-xl font-bold mb-4 border-b border-white/10 pb-2">SYSTEM LOGS</h2>
                    <div class="flex flex-col gap-1 text-gray-400">
                      {#each logs as log}
                        <div class="break-words">{log}</div>
                      {/each}
                      {#if logs.length === 0}
                        <div class="opacity-30">System ready...</div>
                      {/if}
                    </div>
                  </div>
                </div>
              </div>
            </Canvas>
          {:else if activeTab === 'WORKSPACE'}
             <!-- Workspace Grid -->
             <div class="p-6 h-full grid grid-cols-1 lg:grid-cols-3 gap-6 overflow-y-auto custom-scrollbar">
                <!-- Module 1: Silent Disco (Large, Top Left) -->
                <div class="lg:col-span-2 aspect-video">
                   <h3 class="text-xs font-bold text-white/50 mb-2 tracking-widest">SILENT DISCO</h3>
                   <SyncPlayer targetPeerId={connectedPeerId} />
                </div>
                
                <!-- Right Column Stack -->
                <div class="lg:col-span-1 flex flex-col gap-6">
                    <!-- Module 2: Ghost Notes -->
                    <div class="h-64">
                         <StickyBoard targetPeerId={connectedPeerId} />
                    </div>
                    
                    <!-- Module 3: Vault -->
                    <div class="flex-1 min-h-[300px]">
                        <Vault />
                    </div>
                </div>
             </div>
          {/if}
      </main>
  </div>
</div>

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 6px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: rgba(255, 255, 255, 0.05);
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.2);
    border-radius: 3px;
  }
</style>
