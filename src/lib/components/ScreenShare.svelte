<script lang="ts">
  import ScreenSharePanel from './ScreenSharePanel.svelte';

  export let stream: MediaStream | null = null;
  export let label = "You are sharing";
  export let onClose: () => void;
  export let peerConnection: RTCPeerConnection | null = null;
  export let onChangeSource: (() => void) | undefined = undefined;
</script>

{#if stream}
  <ScreenSharePanel 
     {stream} 
     {label} 
     {onClose} 
     {peerConnection}
     {onChangeSource}
  />
{/if}
