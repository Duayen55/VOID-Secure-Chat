import { writable } from 'svelte/store';

export interface Settings {
  isPTTEnabled: boolean;
  pttKey: string;
  ringVolume: number;
  shortcuts: {
    mute: string;
    deafen: string;
    end: string;
  };
  notifications: boolean;
  screenResolution: '720p' | '1080p' | '4k';
  screenBitrate: number;
  screenFramerate: number;
  shareSystemAudio: boolean;
  // Audio Settings
  selectedMicId?: string;
  selectedSpeakerId?: string;
  noiseSuppression: boolean;
  echoCancellation: boolean;
  autoGainControl: boolean;
  noiseGateThreshold: number;
}

const defaultSettings: Settings = {
  isPTTEnabled: false,
  pttKey: 'p',
  ringVolume: 0.5,
  shortcuts: {
    mute: 'm',
    deafen: 'd',
    end: 'escape'
  },
  notifications: true,
  screenResolution: '1080p',
  screenBitrate: 3000000, // 3 Mbps
  screenFramerate: 30,
  shareSystemAudio: false,
  // Audio Defaults
  noiseSuppression: true,
  echoCancellation: true,
  autoGainControl: true,
  noiseGateThreshold: -50
};

function createSettingsStore() {
  const { subscribe, set, update } = writable<Settings>(defaultSettings);

  return {
    subscribe,
    set,
    update,
    load: () => {
        if (typeof localStorage !== 'undefined') {
            const saved = localStorage.getItem('void_settings');
            if (saved) {
                try {
                    const parsed = JSON.parse(saved);
                    update(s => ({ ...s, ...parsed }));
                } catch (e) {
                    console.error("Failed to load settings", e);
                }
            }
        }
    },
    save: (s: Settings) => {
        if (typeof localStorage !== 'undefined') {
            localStorage.setItem('void_settings', JSON.stringify(s));
        }
    }
  };
}

export const settings = createSettingsStore();
export const user = writable<any>(null);
export const activeChat = writable<any>(null);
