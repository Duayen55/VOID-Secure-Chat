<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  let { targetPeerId } = $props(); 
  let videoElement: HTMLVideoElement;
  let src = $state('');
  let syncStatus = $state('red'); 
  let ignoreNextSeek = false;

  async function handleFileDrop(e: DragEvent) {
      e.preventDefault();
      const file = e.dataTransfer?.files[0];
      if (file) {
          src = URL.createObjectURL(file);
      }
  }

  function broadcast(action: string, time: number) {
      if (!targetPeerId) return;
      invoke('send_signal', {
          peerId: targetPeerId,
          payload: JSON.stringify({ type: 'MEDIA_SYNC', action, time, timestamp: Date.now() })
      }).catch(e => console.error(e));
  }

  function onPlay() {
      broadcast('play', videoElement.currentTime);
      syncStatus = 'green';
  }

  function onPause() {
      broadcast('pause', videoElement.currentTime);
      syncStatus = 'red'; 
  }

  function onSeek() {
      if (ignoreNextSeek) {
          ignoreNextSeek = false;
          return;
      }
      broadcast('seek', videoElement.currentTime);
  }

  onMount(() => {
      const unlisten = listen('signal-event', (event: any) => {
          const payload = event.payload; 
          
          if (payload.peerId !== targetPeerId) return;

          try {
              const data = JSON.parse(payload.payload);
              if (data.type === 'MEDIA_SYNC') {
                  if (Math.abs(data.time - videoElement.currentTime) > 0.1) {
                      ignoreNextSeek = true;
                      videoElement.currentTime = data.time;
                  }
                  
                  if (data.action === 'play') {
                      videoElement.play().catch(() => {});
                      syncStatus = 'green';
                  } else if (data.action === 'pause') {
                      videoElement.pause();
                      syncStatus = 'red';
                  }
              }
          } catch (e) {
              console.error("Sync Parse Error", e);
          }
      });

      return () => {
          unlisten.then(f => f());
      };
  });
</script>

<div 
  class="relative w-full aspect-video bg-black border border-white/10 rounded-lg overflow-hidden group"
  ondrop={handleFileDrop}
  ondragover={(e) => e.preventDefault()}
  role="region"
  aria-label="Video Player"
>
  <!-- Sync Indicator -->
  <div class={`absolute top-2 right-2 w-2 h-2 rounded-full ${syncStatus === 'green' ? 'bg-green-500 shadow-[0_0_10px_#22c55e]' : 'bg-red-500 shadow-[0_0_10px_#ef4444]'} z-20 transition-colors`}></div>

  {#if src}
      <video 
          bind:this={videoElement}
          src={src}
          class="w-full h-full object-contain"
          controls
          onplay={onPlay}
          onpause={onPause}
          onseeking={onSeek}
      >
        <track kind="captions">
      </video>
  {:else}
      <div class="absolute inset-0 flex items-center justify-center text-white/30 text-xs pointer-events-none border-2 border-dashed border-white/5 m-4 rounded">
          DRAG VIDEO FILE HERE
      </div>
  {/if}
</div>
