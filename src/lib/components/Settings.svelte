<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { settings } from '$lib/stores';
  import { fade, fly } from 'svelte/transition';
  import ScreenShareSettings from './ScreenShareSettings.svelte';
  import AudioSettings from './AudioSettings.svelte';

  export let show = false;

  function close() {
    show = false;
    settings.save($settings);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if show}
  <div class="fixed inset-0 bg-black/80 backdrop-blur-md z-[60] flex items-center justify-center p-4" transition:fade={{ duration: 200 }}>
    <div 
      class="bg-[#161A22] border border-gray-700 w-full max-w-3xl rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh]"
      transition:fly={{ y: 20, duration: 300 }}
    >
      <!-- Header -->
      <div class="p-5 border-b border-gray-700 flex justify-between items-center bg-[#0F1115] shrink-0">
        <h3 class="font-bold text-white tracking-wider flex items-center space-x-2 text-lg">
          <span class="text-blue-500">⚙️</span> <span>SYSTEM SETTINGS</span>
        </h3>
        <button on:click={close} class="text-gray-400 hover:text-white transition p-2 hover:bg-gray-800 rounded-full">✖</button>
      </div>
      
      <!-- Scrollable Content -->
      <div class="flex-1 overflow-y-auto p-6 space-y-8 custom-scrollbar">
        
        <!-- Audio Settings Section (New) -->
        <section>
             <AudioSettings />
        </section>

        <div class="h-px bg-gray-700/50 w-full"></div>

        <!-- Ringtone & Screen Share -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <!-- Ringtone -->
            <section class="space-y-4">
                 <h4 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Ringtone</h4>
                 <div class="bg-gray-800/50 p-4 rounded-xl border border-gray-700/50 h-full">
                     <div class="flex items-center justify-between mb-2">
                         <span class="text-gray-300">Volume</span>
                         <span class="text-blue-400 font-mono">{Math.round($settings.ringVolume * 100)}%</span>
                     </div>
                     <input 
                        type="range" 
                        min="0" max="1" step="0.05" 
                        bind:value={$settings.ringVolume}
                        class="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
                     />
                 </div>
            </section>

            <!-- Screen Share -->
            <section class="space-y-4">
                 <h4 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Screen Share</h4>
                 <div class="bg-gray-800/50 p-4 rounded-xl border border-gray-700/50 h-full">
                    <ScreenShareSettings />
                 </div>
            </section>
        </div>

        <div class="h-px bg-gray-700/50 w-full"></div>

        <!-- Voice Activation (PTT) -->
        <section class="space-y-3">
          <h4 class="text-xs font-bold text-gray-500 uppercase tracking-widest border-b border-gray-800 pb-2">Voice Activation</h4>
          <div class="flex items-center justify-between p-4 bg-gray-800/30 rounded-xl border border-gray-700/50 hover:border-gray-600 transition">
             <div class="flex flex-col">
                <span class="text-white font-medium">Push to Talk</span>
                <span class="text-xs text-gray-500">Hold key to speak</span>
             </div>
             <button 
               on:click={() => $settings.isPTTEnabled = !$settings.isPTTEnabled} 
               class={`w-14 h-7 rounded-full p-1 transition-colors duration-300 relative ${$settings.isPTTEnabled ? 'bg-blue-600' : 'bg-gray-700'}`}
             >
                <div class={`w-5 h-5 bg-white rounded-full shadow-md transform transition-transform duration-300 ${$settings.isPTTEnabled ? 'translate-x-7' : 'translate-x-0'}`}></div>
             </button>
          </div>

          {#if $settings.isPTTEnabled}
            <div class="flex items-center justify-between p-4 bg-gray-800/30 rounded-xl border border-gray-700/50" transition:fly={{ y: -10, duration: 200 }}>
               <span class="text-gray-300 text-sm">Activation Key</span>
               <input 
                  type="text" 
                  maxlength="1" 
                  bind:value={$settings.pttKey} 
                  on:input={() => $settings.pttKey = $settings.pttKey.toLowerCase()}
                  class="w-12 h-12 bg-black border border-gray-600 rounded-lg text-center text-white uppercase font-bold focus:border-blue-500 outline-none text-xl"
               />
            </div>
          {/if}
        </section>

        <!-- Notifications -->
        <section class="space-y-3">
          <h4 class="text-xs font-bold text-gray-500 uppercase tracking-widest border-b border-gray-800 pb-2">Notifications</h4>
          <div class="flex items-center justify-between p-4 bg-gray-800/30 rounded-xl border border-gray-700/50 hover:border-gray-600 transition">
             <div class="flex flex-col">
                <span class="text-white font-medium">Desktop Notifications</span>
                <span class="text-xs text-gray-500">Popups for calls and messages</span>
             </div>
             <button 
               on:click={() => $settings.notifications = !$settings.notifications} 
               class={`w-14 h-7 rounded-full p-1 transition-colors duration-300 relative ${$settings.notifications ? 'bg-green-600' : 'bg-gray-700'}`}
             >
                <div class={`w-5 h-5 bg-white rounded-full shadow-md transform transition-transform duration-300 ${$settings.notifications ? 'translate-x-7' : 'translate-x-0'}`}></div>
             </button>
          </div>
        </section>

        <!-- Shortcuts -->
        <section class="space-y-3">
          <h4 class="text-xs font-bold text-gray-500 uppercase tracking-widest border-b border-gray-800 pb-2">Keyboard Shortcuts</h4>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
             {#each Object.entries($settings.shortcuts) as [name, key]}
                <div class="p-3 bg-gray-800/30 rounded-lg border border-gray-700/50 flex justify-between items-center">
                   <span class="text-gray-300 capitalize text-sm font-medium">{name}</span>
                   <div class="relative">
                      <input 
                         type="text" 
                         maxlength="1" 
                         value={key}
                         on:input={(e) => $settings.shortcuts[name] = e.currentTarget.value.toLowerCase()}
                         class="w-8 h-8 bg-black border border-gray-600 rounded text-center text-white uppercase text-xs font-bold focus:border-blue-500 outline-none"
                      />
                   </div>
                </div>
             {/each}
          </div>
        </section>

      </div>
      
      <!-- Footer -->
      <div class="p-4 bg-[#0F1115] border-t border-gray-700 text-center shrink-0">
        <span class="text-[10px] text-gray-600">VOID COMMUNICATIONS PROTOCOL v2.0</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .custom-scrollbar::-webkit-scrollbar { width: 6px; }
  .custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
  .custom-scrollbar::-webkit-scrollbar-thumb { background: #374151; border-radius: 3px; }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #4b5563; }
</style>