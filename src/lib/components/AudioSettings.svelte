<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import { settings } from '$lib/stores';

  export let show = true;

  // Dispatcher for external integration if needed
  const dispatch = createEventDispatcher();

  // --- State ---
  let microphones: MediaDeviceInfo[] = [];
  let speakers: MediaDeviceInfo[] = [];
  
  // Audio Processing State
  let audioContext: AudioContext | null = null;
  let sourceNode: MediaStreamAudioSourceNode | null = null;
  let gainNode: GainNode | null = null; // Used for Noise Gate
  let analyser: AnalyserNode | null = null;
  let stream: MediaStream | null = null;
  let animationFrame: number;
  
  // Noise Gate Runtime State
  let currentLevel = -100; // dB
  let isGated = false;

  // Monitoring
  let monitorAudio = false; // Toggle to hear yourself
  let audioOutputElement: HTMLAudioElement;

  // Visualizer
  let canvas: HTMLCanvasElement;
  let canvasCtx: CanvasRenderingContext2D | null = null;

  onMount(async () => {
    await getDevices();
    navigator.mediaDevices.addEventListener('devicechange', getDevices);
    
    // Initialize Audio Context (user interaction usually required, but here we assume context of settings panel)
    // We'll start processing when a mic is selected or immediately if permission exists
    startAudioProcessing();
  });

  onDestroy(() => {
    stopAudioProcessing();
    navigator.mediaDevices.removeEventListener('devicechange', getDevices);
  });

  async function getDevices() {
    try {
      // Request permission first to get labels
      const permStream = await navigator.mediaDevices.getUserMedia({ audio: true });
      permStream.getTracks().forEach(t => t.stop());

      const devices = await navigator.mediaDevices.enumerateDevices();
      microphones = devices.filter(d => d.kind === 'audioinput');
      speakers = devices.filter(d => d.kind === 'audiooutput');

      // Set defaults if not set in store
      if (!$settings.selectedMicId && microphones.length > 0) {
          $settings.selectedMicId = microphones[0].deviceId;
      }
      if (!$settings.selectedSpeakerId && speakers.length > 0) {
          $settings.selectedSpeakerId = speakers[0].deviceId;
      }
    } catch (err) {
      console.error("Error enumerating devices:", err);
    }
  }

  async function startAudioProcessing() {
    stopAudioProcessing(); // Cleanup previous

    if (!$settings.selectedMicId) return;

    try {
      const constraints = {
        audio: {
          deviceId: { exact: $settings.selectedMicId },
          noiseSuppression: $settings.noiseSuppression,
          echoCancellation: $settings.echoCancellation,
          autoGainControl: $settings.autoGainControl
        }
      };

      stream = await navigator.mediaDevices.getUserMedia(constraints);
      
      // Notify parent/external integration
      dispatch('stream', stream);

      // Web Audio API Setup
      audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
      sourceNode = audioContext.createMediaStreamSource(stream);
      analyser = audioContext.createAnalyser();
      gainNode = audioContext.createGain();
      
      analyser.fftSize = 256;
      analyser.smoothingTimeConstant = 0.3;

      // Connect Chain: Source -> Analyser -> Gain (Gate) -> Destination (if monitoring)
      sourceNode.connect(analyser);
      analyser.connect(gainNode);
      
      // For monitoring/speaker selection, we use a MediaStreamDestination or Audio Element
      // Since we need setSinkId, we must use HTMLAudioElement
      if (audioOutputElement) {
        const dest = audioContext.createMediaStreamDestination();
        gainNode.connect(dest);
        audioOutputElement.srcObject = dest.stream;
        
        // Apply Speaker Selection
        if ('setSinkId' in audioOutputElement && $settings.selectedSpeakerId) {
          // @ts-ignore
          audioOutputElement.setSinkId($settings.selectedSpeakerId).catch(e => console.warn("setSinkId failed", e));
        }
      }

      drawVisualizer();

    } catch (err) {
      console.error("Error starting audio processing:", err);
    }
  }

  function stopAudioProcessing() {
    if (stream) stream.getTracks().forEach(t => t.stop());
    if (audioContext) audioContext.close();
    if (animationFrame) cancelAnimationFrame(animationFrame);
    stream = null;
    audioContext = null;
    sourceNode = null;
    gainNode = null;
    analyser = null;
  }

  function updateGate() {
    if (!analyser || !gainNode) return;

    const bufferLength = analyser.frequencyBinCount;
    const dataArray = new Uint8Array(bufferLength);
    analyser.getByteTimeDomainData(dataArray);

    // Calculate RMS
    let sum = 0;
    for(let i = 0; i < bufferLength; i++) {
        const x = (dataArray[i] - 128) / 128.0;
        sum += x * x;
    }
    const rms = Math.sqrt(sum / bufferLength);
    
    // Convert to dB
    const db = 20 * Math.log10(rms + 0.00001); // Prevent -Infinity
    currentLevel = db;

    // Gate Logic
    if (db < $settings.noiseGateThreshold) {
        // Close Gate (Mute)
        // Smooth transition
        gainNode.gain.setTargetAtTime(0, audioContext!.currentTime, 0.1);
        isGated = true;
      } else {
        // Open Gate (Unmute)
        gainNode.gain.setTargetAtTime(1, audioContext!.currentTime, 0.05);
        isGated = false;
      }
  }

  function drawVisualizer() {
    if (!canvas) return;
    if (!canvasCtx) canvasCtx = canvas.getContext('2d');
    if (!canvasCtx) return;

    updateGate(); // Run gate logic in loop

    const width = canvas.width;
    const height = canvas.height;

    canvasCtx.fillStyle = '#111827'; // Dark BG
    canvasCtx.fillRect(0, 0, width, height);

    // Draw Threshold Line
    // Map -100dB to 0dB range to canvas width
    // Range: -100 to 0
    const normalizeDB = (db: number) => Math.min(Math.max((db + 100) / 100, 0), 1);
    
    const thresholdX = normalizeDB($settings.noiseGateThreshold) * width;
    const currentX = normalizeDB(currentLevel) * width;

    // Draw Level Bar
    canvasCtx.fillStyle = isGated ? '#EF4444' : '#10B981'; // Red if gated, Green if open
    canvasCtx.fillRect(0, 0, currentX, height);

    // Draw Threshold Marker
    canvasCtx.beginPath();
    canvasCtx.moveTo(thresholdX, 0);
    canvasCtx.lineTo(thresholdX, height);
    canvasCtx.strokeStyle = '#FCD34D'; // Yellow
    canvasCtx.lineWidth = 2;
    canvasCtx.stroke();
    
    // Text Info
    canvasCtx.fillStyle = '#fff';
    canvasCtx.font = '10px sans-serif';
    canvasCtx.fillText(`Level: ${currentLevel.toFixed(1)} dB`, 5, 12);
    canvasCtx.fillText(`Gate: ${$settings.noiseGateThreshold} dB`, thresholdX + 5, 12);

    animationFrame = requestAnimationFrame(drawVisualizer);
  }

  // Handle changes
  function handleConstraintChange() {
    // Restart to apply constraints reliably
    startAudioProcessing();
  }
  
  function handleDeviceChange() {
      startAudioProcessing();
  }

  $: if (monitorAudio && audioOutputElement) {
      audioOutputElement.play().catch(() => {});
  } else if (audioOutputElement) {
      audioOutputElement.pause();
  }

  $: if ($settings.selectedSpeakerId && audioOutputElement && 'setSinkId' in audioOutputElement) {
      // @ts-ignore
      audioOutputElement.setSinkId($settings.selectedSpeakerId).catch(e => console.warn(e));
  }

</script>

<div class="bg-gray-900 text-gray-200 p-6 rounded-xl w-full max-w-2xl mx-auto shadow-xl border border-gray-700">
    <div class="mb-6 border-b border-gray-700 pb-4">
        <h2 class="text-xl font-bold text-white flex items-center">
            <span class="mr-2">🎙️</span> Audio Settings
        </h2>
        <p class="text-sm text-gray-400 mt-1">Configure input, output, and processing.</p>
    </div>

    <div class="space-y-6">
        <!-- Device Selection -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <!-- Microphone -->
            <div class="space-y-2">
                <label class="block text-xs font-semibold uppercase tracking-wider text-gray-400">Microphone</label>
                <select 
                    bind:value={$settings.selectedMicId} 
                    on:change={handleDeviceChange}
                    class="w-full bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500 outline-none text-white"
                >
                    {#each microphones as mic}
                        <option value={mic.deviceId}>{mic.label || `Microphone ${mic.deviceId.slice(0,5)}...`}</option>
                    {/each}
                </select>
            </div>

            <!-- Speaker -->
            <div class="space-y-2">
                <label class="block text-xs font-semibold uppercase tracking-wider text-gray-400">Speaker</label>
                <select 
                    bind:value={$settings.selectedSpeakerId} 
                    class="w-full bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500 outline-none text-white"
                >
                    {#each speakers as speaker}
                        <option value={speaker.deviceId}>{speaker.label || `Speaker ${speaker.deviceId.slice(0,5)}...`}</option>
                    {/each}
                </select>
            </div>
        </div>

        <!-- Processing Features -->
        <div class="bg-gray-800/50 rounded-lg p-4 space-y-3 border border-gray-700/50">
            <label class="block text-xs font-semibold uppercase tracking-wider text-gray-400 mb-2">Enhancements</label>
            <div class="flex flex-wrap gap-4">
                <label class="flex items-center space-x-2 cursor-pointer select-none">
                    <input type="checkbox" bind:checked={$settings.noiseSuppression} on:change={handleConstraintChange} class="rounded bg-gray-700 border-gray-600 text-blue-500 focus:ring-blue-500/50">
                    <span class="text-sm">Noise Suppression</span>
                </label>
                <label class="flex items-center space-x-2 cursor-pointer select-none">
                    <input type="checkbox" bind:checked={$settings.echoCancellation} on:change={handleConstraintChange} class="rounded bg-gray-700 border-gray-600 text-blue-500 focus:ring-blue-500/50">
                    <span class="text-sm">Echo Cancellation</span>
                </label>
                <label class="flex items-center space-x-2 cursor-pointer select-none">
                    <input type="checkbox" bind:checked={$settings.autoGainControl} on:change={handleConstraintChange} class="rounded bg-gray-700 border-gray-600 text-blue-500 focus:ring-blue-500/50">
                    <span class="text-sm">Auto Gain</span>
                </label>
            </div>
        </div>

        <!-- Noise Gate -->
        <div class="space-y-2">
            <div class="flex justify-between items-center">
                <label class="block text-xs font-semibold uppercase tracking-wider text-gray-400">Noise Gate Threshold</label>
                <span class="text-xs font-mono text-blue-400">{$settings.noiseGateThreshold} dB</span>
            </div>
            
            <div class="relative h-24 bg-black rounded-lg overflow-hidden border border-gray-700 shadow-inner">
                <canvas bind:this={canvas} width="600" height="100" class="w-full h-full block"></canvas>
            </div>
            
            <input 
                type="range" 
                min="-100" max="0" step="1" 
                bind:value={$settings.noiseGateThreshold}
                class="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500 mt-2"
            >
            <p class="text-xs text-gray-500">Audio below this level will be silenced.</p>
        </div>

        <!-- Monitoring -->
        <div class="flex items-center justify-between bg-gray-800/50 p-4 rounded-lg border border-gray-700/50">
            <div class="flex items-center space-x-3">
                <div class="p-2 bg-gray-700 rounded-lg">
                    🎧
                </div>
                <div>
                    <h4 class="text-sm font-medium text-white">Test Audio</h4>
                    <p class="text-xs text-gray-400">Listen to your microphone input</p>
                </div>
            </div>
            <label class="relative inline-flex items-center cursor-pointer">
                <input type="checkbox" bind:checked={monitorAudio} class="sr-only peer">
                <div class="w-11 h-6 bg-gray-700 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-800 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
            </label>
        </div>
        
        <!-- Hidden Audio Element for Monitoring -->
        <audio bind:this={audioOutputElement} class="hidden"></audio>
    </div>
</div>