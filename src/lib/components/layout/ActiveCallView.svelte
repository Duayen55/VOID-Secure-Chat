<script lang="ts">
  import DraggablePanel from '$lib/components/DraggablePanel.svelte';
  import MusicPlayer from '$lib/components/MusicPlayer.svelte';
  import { user } from '$lib/stores';
  import { onMount, onDestroy } from 'svelte';

  export let localStream: MediaStream | null = null;
  export let remoteStream: MediaStream | null = null;
  export let isScreenSharing = false;
  export let isMusicActive = false;
  export let activeChat: any = null;
  export let friends: any[] = [];
  export let onCloseMusic: () => void = () => {};

  let localVideoEl: HTMLVideoElement;
  let remoteVideoEl: HTMLVideoElement;
  
  let localVolume = 0;
  let remoteVolume = 0;
  let animationFrame: number;
  let audioContext: AudioContext;

  $: if (localVideoEl && localStream) localVideoEl.srcObject = localStream;
  $: if (remoteVideoEl && remoteStream) remoteVideoEl.srcObject = remoteStream;

  // Audio Analysis Logic
  function analyzeAudio(stream: MediaStream, update: (vol: number) => void) {
    if (!stream.getAudioTracks().length) return null;
    
    try {
        if (!audioContext) audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
        if (audioContext.state === 'suspended') audioContext.resume();

        const source = audioContext.createMediaStreamSource(stream);
        const analyzer = audioContext.createAnalyser();
        analyzer.fftSize = 64;
        source.connect(analyzer);
        
        const dataArray = new Uint8Array(analyzer.frequencyBinCount);
        return () => {
            analyzer.getByteFrequencyData(dataArray);
            // Get average volume
            let sum = 0;
            for(let i = 0; i < dataArray.length; i++) sum += dataArray[i];
            const avg = sum / dataArray.length;
            // Normalize roughly 0-100
            update(avg); 
        };
    } catch (e) {
        console.error("Audio analysis failed:", e);
        return null;
    }
  }

  let stopLocal: (() => void) | null = null;
  let stopRemote: (() => void) | null = null;

  $: if (localStream && !stopLocal) {
      const run = analyzeAudio(localStream, (v) => localVolume = v);
      if (run) stopLocal = run;
  }
  $: if (remoteStream && !stopRemote) {
      const run = analyzeAudio(remoteStream, (v) => remoteVolume = v);
      if (run) stopRemote = run;
  }

  function loop() {
      if (stopLocal) stopLocal();
      if (stopRemote) stopRemote();
      animationFrame = requestAnimationFrame(loop);
  }

  onMount(() => {
      loop();
  });

  onDestroy(() => {
      cancelAnimationFrame(animationFrame);
      if (audioContext) audioContext.close();
  });

  function getInitials(name: string) {
      return (name || 'Remote').slice(0, 2).toUpperCase();
  }

  $: displayName = activeChat?.displayName || activeChat?.friendName || 'Remote User';
</script>

<div class="flex-1 bg-obsidian relative p-4 flex items-center justify-center overflow-hidden">
    
    <!-- Music Player Overlay (Draggable) -->
    {#if isMusicActive}
        <DraggablePanel 
            title="Music Player" 
            left={100} top={100} 
            width="380px" height="auto"
            onClose={onCloseMusic}
        >
            <div class="bg-[#161A22] w-full h-full min-h-[400px]">
                <MusicPlayer 
                    {friends} 
                    {activeChat} 
                    embedded={true}
                    isMinimized={false}
                />
            </div>
        </DraggablePanel>
    {/if}

    <!-- Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 w-full h-full max-w-6xl max-h-[80vh]">
        
        <!-- Remote Stream (Main Focus) -->
        <div class="relative bg-surface rounded-2xl overflow-hidden border border-white/5 shadow-[0_0_20px_rgba(0,0,0,0.5)] ring-1 ring-white/5 group transition-all duration-300 hover:shadow-[0_0_30px_rgba(0,229,255,0.1)]">
             {#if remoteStream && remoteStream.getVideoTracks().length > 0 && remoteStream.getVideoTracks()[0].enabled}
                <!-- svelte-ignore a11y-media-has-caption -->
                <video 
                    autoplay 
                    playsinline 
                    class="w-full h-full object-cover"
                    bind:this={remoteVideoEl}
                ></video>
             {:else}
                <!-- Remote Avatar -->
                <div class="absolute inset-0 flex items-center justify-center flex-col space-y-4 bg-gradient-to-b from-gray-900 to-black">
                    <div 
                        class="w-32 h-32 rounded-full flex items-center justify-center text-4xl font-bold text-white shadow-2xl transition-all duration-100"
                        style="
                            background: linear-gradient(135deg, #00E5FF, #2979FF);
                            box-shadow: 0 0 {remoteVolume * 2}px rgba(0, 229, 255, {remoteVolume / 50});
                            transform: scale({1 + remoteVolume / 200});
                        "
                    >
                        {getInitials(displayName)}
                    </div>
                    {#if remoteVolume > 10}
                        <span class="text-accent text-sm font-medium animate-pulse">Speaking...</span>
                    {/if}
                </div>
                <!-- Hidden video element for audio playback -->
                <video autoplay playsinline class="hidden" bind:this={remoteVideoEl}></video>
             {/if}
             
             <!-- Overlay Name -->
             <div class="absolute bottom-4 left-4 bg-black/50 backdrop-blur-md px-3 py-1.5 rounded-lg text-white text-sm font-medium border border-white/10 flex items-center gap-2">
                 {#if remoteVolume > 10}
                    <span class="w-2 h-2 rounded-full bg-green-400 animate-pulse"></span>
                 {/if}
                 {displayName}
             </div>
        </div>

        <!-- Local Stream (Self) -->
        <div class="relative bg-surface rounded-2xl overflow-hidden border border-white/5 shadow-2xl ring-1 ring-white/5">
             {#if localStream && localStream.getVideoTracks().length > 0 && localStream.getVideoTracks()[0].enabled}
                 <!-- svelte-ignore a11y-media-has-caption -->
                 <video 
                    autoplay 
                    playsinline 
                    muted 
                    class="w-full h-full object-cover {isScreenSharing ? '' : 'transform -scale-x-100'}"
                    bind:this={localVideoEl}
                ></video>
             {:else}
                 <!-- Local Avatar -->
                 <div class="absolute inset-0 flex items-center justify-center bg-gradient-to-b from-gray-800 to-gray-900">
                    <div 
                        class="w-24 h-24 rounded-full bg-surface-hover flex items-center justify-center text-2xl font-bold text-white ring-1 ring-white/10 transition-all duration-100"
                        style="
                            box-shadow: 0 0 {localVolume * 1.5}px rgba(255, 255, 255, {localVolume / 100});
                            transform: scale({1 + localVolume / 300});
                        "
                    >
                        {getInitials($user?.displayName || 'You')}
                    </div>
                 </div>
                 <!-- Hidden video element just in case -->
                 <video autoplay playsinline muted class="hidden" bind:this={localVideoEl}></video>
             {/if}
             <div class="absolute bottom-4 left-4 bg-black/50 backdrop-blur-md px-3 py-1.5 rounded-lg text-white text-sm font-medium border border-white/10">
                 You
             </div>
        </div>

    </div>
</div>
