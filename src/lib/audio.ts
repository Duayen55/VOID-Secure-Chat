import { get } from 'svelte/store';
import { settings } from './stores';

let ringToneContext: AudioContext | null = null;
let ringInterval: any = null;

export function playRingtone() {
  if (ringInterval) return; 
  
  if (!ringToneContext) {
    ringToneContext = new (window.AudioContext || (window as any).webkitAudioContext)();
  }
  if (ringToneContext.state === 'suspended') ringToneContext.resume();
  
  const playBeep = () => {
    if (!ringToneContext) return;
    const osc = ringToneContext.createOscillator();
    const gain = ringToneContext.createGain();
    
    osc.connect(gain);
    gain.connect(ringToneContext.destination);
    
    osc.type = 'sine';
    osc.frequency.setValueAtTime(440, ringToneContext.currentTime); 
    osc.frequency.exponentialRampToValueAtTime(880, ringToneContext.currentTime + 0.1);
    
    const vol = get(settings).ringVolume;
    
    gain.gain.setValueAtTime(0, ringToneContext.currentTime);
    gain.gain.linearRampToValueAtTime(vol, ringToneContext.currentTime + 0.1);
    gain.gain.exponentialRampToValueAtTime(0.01, ringToneContext.currentTime + 0.5);
    
    osc.start(ringToneContext.currentTime);
    osc.stop(ringToneContext.currentTime + 0.5);
  };

  playBeep();
  ringInterval = setInterval(playBeep, 2000); 
}

export function stopRingtone() {
  if (ringInterval) {
    clearInterval(ringInterval);
    ringInterval = null;
  }
  if (ringToneContext) {
    ringToneContext.close();
    ringToneContext = null;
  }
}
