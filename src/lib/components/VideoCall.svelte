<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import { fade, slide, scale, fly } from 'svelte/transition';
  import { settings } from '$lib/stores';

  export let callState: 'idle' | 'ringing' | 'connected';
  export let isCaller: boolean;
  export let remoteStream: MediaStream | null = null;
  export let localStream: MediaStream | null = null;
  export let isMuted = false;
  export let isDeafened = false;
  export let isPTTActive = false;

  const dispatch = createEventDispatcher();

  let remoteAudioElement: HTMLAudioElement;
  let audioContext: AudioContext | null = null;
  let analyser: AnalyserNode | null = null;
  let audioLevels: number[] = new Array(5).fill(0);
  let animationFrameId: number;
  let remoteVolume = 1.0;

  // Visualizer Logic
  function setupAudioVisualizer(stream: MediaStream) {
    if (analyser || !stream) return; 
    
    try {
        if (!audioContext) {
            audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
        }
        if (audioContext.state === 'suspended') {
            audioContext.resume();
        }
        
        // Check if stream has audio tracks
        if (stream.getAudioTracks().length === 0) return;

        const source = audioContext.createMediaStreamSource(stream);
        analyser = audioContext.createAnalyser();
        analyser.fftSize = 32; 
        source.connect(analyser);
        
        drawVisualizer();
    } catch (e) {
        console.error("Visualizer setup failed", e);
    }
  }

  function drawVisualizer() {
    if (!analyser) return;
    const dataArray = new Uint8Array(analyser.frequencyBinCount);
    analyser.getByteFrequencyData(dataArray);
    
    const indices = [0, 2, 4, 6, 8];
    audioLevels = indices.map(i => dataArray[i] / 255); 
    
    animationFrameId = requestAnimationFrame(drawVisualizer);
  }

  $: if (remoteStream && callState === 'connected') {
     setupAudioVisualizer(remoteStream);
  }

  $: if (remoteAudioElement) {
     remoteAudioElement.volume = isDeafened ? 0 : remoteVolume;
  }

  $: if (remoteAudioElement && $settings.selectedSpeakerId && 'setSinkId' in remoteAudioElement) {
      // @ts-ignore
      remoteAudioElement.setSinkId($settings.selectedSpeakerId).catch(e => console.warn("setSinkId failed", e));
  }

  function handleVolumeChange(e: Event) {
    const v = parseFloat((e.target as HTMLInputElement).value);
    remoteVolume = v;
  }

  onDestroy(() => {
    if (animationFrameId) cancelAnimationFrame(animationFrameId);
    if (audioContext) audioContext.close();
  });
</script>

<div class="relative w-full h-full bg-black flex flex-col items-center justify-center overflow-hidden">
  
  <!-- Remote Video / Audio -->
  {#if remoteStream}
    <!-- svelte-ignore a11y_media_has_caption -->
    <video 
      autoplay 
      playsinline 
      class="absolute inset-0 w-full h-full object-cover opacity-80"
      bind:this={remoteAudioElement} 
      srcObject={remoteStream}
    ></video>
    <!-- Fallback audio element if video is hidden/screen share -->
    <!-- svelte-ignore a11y_media_has_caption -->
    <audio autoplay bind:this={remoteAudioElement} srcObject={remoteStream} class="hidden"></audio>
  {:else}
    <div class="flex flex-col items-center animate-pulse">
        <div class="w-24 h-24 bg-gray-800 rounded-full flex items-center justify-center mb-4">
            <span class="text-4xl">👤</span>
        </div>
        <span class="text-gray-500 tracking-widest text-xs">WAITING FOR VIDEO...</span>
    </div>
  {/if}

  <!-- Connection Status -->
  {#if callState === 'ringing'}
      <div class="absolute top-10 bg-blue-600 px-6 py-2 rounded-full shadow-lg shadow-blue-900/50 z-20" transition:slide>
          <span class="text-white text-xs font-bold animate-pulse tracking-widest">
             {isCaller ? 'CALLING...' : 'INCOMING CALL...'}
          </span>
      </div>
  {/if}

  <!-- Local Video Preview (PiP) -->
  {#if localStream}
      <!-- svelte-ignore a11y_media_has_caption -->
      <div class="absolute bottom-4 right-4 w-32 h-24 bg-gray-900 rounded-lg overflow-hidden border border-gray-700 shadow-xl z-40 transition-all hover:scale-105 group">
         <video 
            autoplay 
            playsinline 
            muted 
            class="w-full h-full object-cover transform scale-x-[-1]" 
            srcObject={localStream}
         ></video>
         <div class="absolute inset-0 bg-black/20 opacity-0 group-hover:opacity-100 transition flex items-center justify-center">
             <span class="text-[10px] text-white font-mono">YOU</span>
         </div>
      </div>
  {/if}

  <!-- CONTROLS BAR -->
  <div class="absolute top-6 bg-[#161A22]/90 backdrop-blur-md border border-gray-700/50 px-8 py-3 rounded-2xl shadow-2xl flex items-center space-x-6 z-50" transition:fly={{ y: -50 }}>
      
      <!-- PTT / MIC -->
      <div class="flex items-center space-x-3">
        <!-- PTT -->
        <button 
            on:click={() => dispatch('togglePTT')}
            class={`flex flex-col items-center justify-center w-12 h-12 rounded-xl transition-all border border-transparent ${$settings.isPTTEnabled ? 'bg-blue-600 text-white' : 'bg-gray-800 text-gray-500 hover:bg-gray-700'}`}
            title={`Push to Talk (${$settings.pttKey.toUpperCase()})`}
        >
            <span class="text-[10px] font-bold">PTT</span>
            <div class={`w-1.5 h-1.5 rounded-full mt-1 ${isPTTActive ? 'bg-green-400 animate-ping' : 'bg-gray-400'}`}></div>
        </button>

        <!-- Mute -->
        <button 
            on:click={() => dispatch('toggleMute')}
            class={`p-3 rounded-xl transition-all duration-200 ${isMuted ? 'bg-red-500/20 text-red-500 border border-red-500/50' : 'bg-gray-700/50 text-white hover:bg-gray-700'}`}
            title="Toggle Mute"
        >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                {#if isMuted}
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 3l18 18" />
                {:else}
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
                {/if}
            </svg>
        </button>
      </div>

      <!-- END CALL -->
      <button 
        on:click={() => dispatch('endCall')}
        class="p-4 bg-red-600 rounded-2xl text-white shadow-lg shadow-red-900/50 hover:scale-105 active:scale-95 transition-transform"
        title="End Call"
      >
         <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
           <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 8l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2M5 3a2 2 0 00-2 2v1c0 8.284 6.716 15 15 15h1a2 2 0 002-2v-3.28a1 1 0 00-.684-.948l-4.493-1.498a1 1 0 00-1.21.502l-1.13 2.257a11.042 11.042 0 01-5.516-5.517l2.257-1.128a1 1 0 00.502-1.21L9.228 3.683A1 1 0 008.279 3H5z" />
         </svg>
      </button>

      <!-- EXTRAS -->
      <div class="flex items-center space-x-3">
         <!-- Music Toggle -->
         <button 
            on:click={() => dispatch('toggleMusic')}
            class="p-3 rounded-xl bg-gray-700/50 text-purple-400 hover:bg-gray-700 transition border border-transparent hover:border-purple-500/30"
            title="Toggle Music Player"
         >
            <span class="text-xl">🎵</span>
         </button>

         <!-- Screen Share -->
         <button 
            on:click={() => dispatch('startScreenShare')}
            class="p-3 rounded-xl bg-gray-700/50 text-blue-400 hover:bg-gray-700 transition border border-transparent hover:border-blue-500/30"
            title="Share Screen"
         >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
               <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
            </svg>
         </button>

         <!-- Deafen -->
         <button 
            on:click={() => dispatch('toggleDeaf')}
            class={`p-3 rounded-xl transition-all ${isDeafened ? 'bg-red-500/20 text-red-500' : 'bg-gray-700/50 text-white hover:bg-gray-700'}`}
            title="Deafen"
         >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                {#if isDeafened}
                     <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                     <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2" />
                {:else}
                     <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                {/if}
            </svg>
         </button>
      </div>

      <!-- Volume & Vis -->
      <div class="flex flex-col space-y-2 w-32 bg-gray-800/80 p-2 rounded-lg backdrop-blur-sm">
          <div class="flex items-center justify-between text-[10px] text-gray-400 font-mono">
              <span>USER VOL</span>
              <span>{Math.round(remoteVolume * 100)}%</span>
          </div>
          
          <!-- Visualizer -->
          <div class="flex items-end space-x-[2px] h-10 justify-center bg-gray-900/50 rounded p-1">
             {#each audioLevels as level}
                <div 
                   class="flex-1 bg-green-500 rounded-[1px] transition-all duration-75 shadow-[0_0_5px_rgba(34,197,94,0.5)]"
                   style="height: {Math.max(5, level * 100)}%; opacity: {0.3 + (level * 0.7)}"
                ></div>
             {/each}
          </div>
          
          <input 
            type="range" min="0" max="1" step="0.01" 
            bind:value={remoteVolume}
            on:input={handleVolumeChange}
            class="w-full h-1 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-green-500"
          />
      </div>

  </div>
</div>
