<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  export let friends: any[] = [];
  export let activeChat: any = null;

  const dispatch = createEventDispatcher();
</script>

<aside class="w-[72px] h-full bg-surface flex flex-col items-center py-4 space-y-2 border-r border-surface z-20 shrink-0">
    <!-- Home / Server Icon -->
    <button 
        class="w-12 h-12 rounded-2xl bg-surface-hover hover:bg-accent hover:text-black transition-fast flex items-center justify-center group relative mb-2 cursor-pointer"
        on:click={() => dispatch('goHome')}
    >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
        </svg>
        <div class="absolute left-16 bg-black text-white text-xs px-2 py-1 rounded opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap pointer-events-none z-50 border border-white/10 shadow-xl font-medium">
            Home
        </div>
    </button>

    <div class="w-8 h-[2px] bg-white/5 rounded-full my-2"></div>

    <!-- Friends List -->
    <div class="flex-1 w-full flex flex-col items-center space-y-3 overflow-y-auto overflow-x-hidden custom-scrollbar py-2">
        {#each friends as friend}
            <button 
                class="w-12 h-12 rounded-2xl transition-fast flex items-center justify-center group relative cursor-pointer {activeChat?.uid === friend.uid ? 'bg-accent/20 text-accent ring-1 ring-accent' : 'bg-surface-hover hover:bg-white/10 text-gray-400 hover:text-white'}"
                on:click={() => dispatch('select', friend)}
            >
                <span class="font-bold text-sm uppercase">{friend.displayName?.slice(0, 2) || '??'}</span>
                
                <!-- Active Indicator Pill -->
                {#if activeChat?.uid === friend.uid}
                    <div class="absolute -left-4 top-1/2 -translate-y-1/2 w-2 h-8 bg-accent rounded-r-full"></div>
                {/if}

                <!-- Tooltip -->
                <div class="absolute left-16 bg-black text-white text-xs px-2 py-1 rounded opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap pointer-events-none z-50 border border-white/10 shadow-xl font-medium">
                    {friend.displayName}
                </div>
            </button>
        {/each}
    </div>
</aside>

<style>
    .custom-scrollbar::-webkit-scrollbar { width: 0px; }
</style>