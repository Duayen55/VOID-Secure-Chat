<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  
  let isMuted = false;
  let isPTT = false;
  let isTalking = false;
  let volume = 0; // 0-100
  let activeProfile = "";

  let bc: BroadcastChannel;

  onMount(() => {
      bc = new BroadcastChannel('void_overlay_sync');
      bc.onmessage = (event) => {
          const data = event.data;
          if (data.type === 'state') {
              isMuted = data.isMuted;
              isPTT = data.isPTT;
              isTalking = data.isTalking;
              volume = data.volume;
              activeProfile = data.activeProfile || "";
          }
      };
  });

  onDestroy(() => {
      if (bc) bc.close();
  });
</script>

<div class="w-full h-full bg-transparent overflow-hidden flex flex-col p-2">
    <!-- HUD Card -->
    <div class="bg-black/90 backdrop-blur-md rounded-lg p-3 text-white min-w-[180px] border-l-4 border-l-blue-500 shadow-2xl pointer-events-auto transition-all duration-200"
            class:border-l-red-500={isMuted}
            class:border-l-green-500={isTalking && !isMuted}
    >
        <!-- Header -->
        <div class="flex justify-between items-start mb-1">
            <div>
                <h1 class="text-xs font-bold text-gray-400 uppercase tracking-widest">VOID HUD</h1>
                {#if activeProfile}
                    <div class="text-[10px] text-blue-300 truncate max-w-[100px]">{activeProfile}</div>
                {/if}
            </div>
            
            <!-- Status Dot -->
            <div class="w-2 h-2 rounded-full transition-colors duration-200 shadow-[0_0_8px_currentColor]"
                    class:bg-red-500={isMuted}
                    class:bg-green-500={isTalking && !isMuted}
                    class:bg-gray-600={!isTalking && !isMuted}
                    class:text-red-500={isMuted}
                    class:text-green-500={isTalking && !isMuted}
            ></div>
        </div>

        <!-- Content -->
        <div class="flex items-center gap-3">
                <div class="flex-1">
                    <div class="text-lg font-mono font-bold leading-none">
                        {#if isMuted}
                        MUTED
                        {:else if isPTT}
                        PTT <span class="text-xs opacity-50">{isTalking ? 'ON' : 'IDLE'}</span>
                        {:else}
                        VOICE <span class="text-xs opacity-50">{isTalking ? 'ACT' : 'IDLE'}</span>
                        {/if}
                    </div>
                </div>
        </div>
        
        <!-- Volume Bar -->
        <div class="mt-2 h-1 bg-gray-700/50 rounded-full overflow-hidden w-full">
            <div class="h-full transition-all duration-75 ease-out"
                    class:bg-red-500={isMuted}
                    class:bg-green-500={!isMuted}
                    style="width: {isMuted ? 0 : volume}%"
            ></div>
        </div>
    </div>
</div>

<style>
    :global(body) {
        margin: 0;
        padding: 0;
        background: transparent;
    }
</style>