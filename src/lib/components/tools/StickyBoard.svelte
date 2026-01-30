<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

  let { targetPeerId, isDetached = false } = $props();
  let text = $state('');
  let lastUpdate = 0;

  function onInput() {
      const now = Date.now();
      lastUpdate = now;
      // If we are connected to a peer, send update
      if (targetPeerId) {
          invoke('send_signal', {
              peerId: targetPeerId,
              payload: JSON.stringify({ type: 'NOTE_UPDATE', text, timestamp: now })
          }).catch(() => {});
      }
      // Also emit local event for other windows (like detached one)
      invoke('send_signal', {
          peerId: 'SELF', // Special handling? Or just emit event from Rust? 
          // Actually, Rust handles P2P. For local window sync, we might need a separate event or just rely on P2P loopback if implemented.
          // But since we share the same backend, we can just use `emit` if we had a command for it.
          // For now, let's assume P2P sync is the main goal. 
          // If I detach, I want the detached window to have the content.
          // I'll pass it via URL param for initial state.
          payload: ''
      }).catch(() => {});
  }

  async function detach() {
      const webview = new WebviewWindow('note_window', {
          url: `/note?peerId=${targetPeerId || ''}&text=${encodeURIComponent(text)}`,
          transparent: true,
          alwaysOnTop: true,
          decorations: false,
          width: 300,
          height: 300
      });
  }

  onMount(() => {
      // Initialize from URL if detached
      if (isDetached) {
          const params = new URLSearchParams(window.location.search);
          const initialText = params.get('text');
          if (initialText) text = initialText;
      }

      const unlisten = listen('signal-event', (event: any) => {
          const payload = event.payload;
          if (targetPeerId && payload.peerId !== targetPeerId) return;
          
          try {
              const data = JSON.parse(payload.payload);
              if (data.type === 'NOTE_UPDATE') {
                  if (data.timestamp > lastUpdate) {
                      text = data.text;
                      lastUpdate = data.timestamp;
                  }
              }
          } catch (e) {}
      });
      return () => { unlisten.then(f => f()); }
  });
</script>

<div class={`bg-black/40 backdrop-blur-md border border-white/10 p-4 w-full h-full flex flex-col ${!isDetached ? 'rounded-lg h-64' : ''}`}>
  <div class="flex justify-between items-center mb-2 handle cursor-move">
      <span class="text-xs font-bold text-white/50">GHOST NOTES</span>
      {#if !isDetached}
        <button onclick={detach} class="text-xs hover:text-white text-white/50 transition-colors">DETACH ⬈</button>
      {/if}
  </div>
  <textarea 
      bind:value={text} 
      oninput={onInput}
      class="flex-1 bg-transparent resize-none outline-none text-sm font-mono text-white/80 placeholder-white/20"
      placeholder="Type here to sync..."
      data-tauri-drag-region
  ></textarea>
</div>
