<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { 
    db, 
    auth, 
    collection, 
    query, 
    where, 
    onSnapshot, 
    doc, 
    updateDoc 
  } from '$lib/firebase';
  import { user, activeChat, pendingCallAccept } from '$lib/stores';
  import { playRingtone, stopRingtone } from '$lib/audio';
  import { onAuthStateChanged } from 'firebase/auth';

  let incomingCall: { caller: string, callerName: string, chatId: string } | null = null;
  let unsubscribe: () => void;
  let authUnsubscribe: () => void;

  onMount(() => {
    if ('Notification' in window && Notification.permission !== 'granted') {
        Notification.requestPermission();
    }

    authUnsubscribe = onAuthStateChanged(auth, (u) => {
      if (u) {
        subscribeToCalls(u.uid);
      } else {
        if (unsubscribe) unsubscribe();
        incomingCall = null;
        stopRingtone();
      }
    });
  });

  onDestroy(() => {
    if (unsubscribe) unsubscribe();
    if (authUnsubscribe) authUnsubscribe();
    stopRingtone();
  });

  function subscribeToCalls(uid: string) {
    if (unsubscribe) unsubscribe();

    const q = query(
      collection(db, "chats"),
      where("to", "==", uid),
      where("status", "==", "ringing")
    );

    unsubscribe = onSnapshot(q, (snap) => {
      // If we have multiple ringing calls, just take the first one
      if (!snap.empty) {
        const change = snap.docChanges()[0];
        if (change.type === 'added' || change.type === 'modified') {
          const data = change.doc.data();
          // Only ring if we are not already in a call (activeChat check might be needed)
          // But for now, let's just show the notification
          if (!incomingCall) {
            incomingCall = {
              caller: data.from,
              callerName: data.callerName || "Unknown",
              chatId: change.doc.id
            };
            playRingtone();
            
            // OS Notification
            if (Notification.permission === 'granted') {
                new Notification("Incoming Call", {
                    body: `${data.callerName || "Unknown"} is calling you...`,
                    requireInteraction: true
                });
            }
          }
        } else if (change.type === 'removed') {
           if (incomingCall && incomingCall.chatId === change.doc.id) {
             dismissCall();
           }
        }
      } else {
        if (incomingCall) {
          dismissCall();
        }
      }
    });
  }

  function dismissCall() {
    incomingCall = null;
    stopRingtone();
  }

  async function acceptCall() {
    if (!incomingCall) return;
    
    stopRingtone();
    const chatId = incomingCall.chatId;
    const callerId = incomingCall.caller;

    // 1. Set global state
    $activeChat = { 
        uid: callerId, 
        chatId: chatId,
        displayName: incomingCall.callerName,
        friendName: incomingCall.callerName 
    };
    $pendingCallAccept = chatId; // Signal +page.svelte to start WebRTC

    // 2. Hide modal locally
    incomingCall = null;

    // 3. Navigate to main page if not there
    await goto('/');
  }

  async function rejectCall() {
    if (!incomingCall) return;
    const cid = incomingCall.chatId;
    dismissCall();

    try {
      await updateDoc(doc(db, "chats", cid), {
        status: 'rejected'
      });
    } catch (e) {
      console.error("Failed to reject call", e);
    }
  }
</script>

{#if incomingCall}
  <div class="fixed inset-0 z-[9999] flex items-center justify-center bg-black/80 backdrop-blur-sm animate-fade-in">
    <div class="bg-gray-900 border border-gray-700 rounded-2xl p-8 shadow-2xl flex flex-col items-center gap-6 max-w-sm w-full mx-4">
      <!-- Avatar/Icon -->
      <div class="w-24 h-24 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center shadow-lg animate-pulse">
        <span class="text-3xl font-bold text-white">{incomingCall.callerName[0].toUpperCase()}</span>
      </div>
      
      <!-- Text -->
      <div class="text-center">
        <h3 class="text-2xl font-bold text-white mb-1">{incomingCall.callerName}</h3>
        <p class="text-gray-400">Incoming Call...</p>
      </div>

      <!-- Actions -->
      <div class="flex items-center gap-4 w-full">
        <button 
          on:click={rejectCall}
          class="flex-1 py-3 px-4 bg-red-500/10 hover:bg-red-500/20 text-red-500 border border-red-500/50 rounded-xl font-semibold transition-all hover:scale-105 active:scale-95 flex items-center justify-center gap-2"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.68 13.31a16 16 0 0 0 3.41 2.6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7 2 2 0 0 1 1.72 2v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.42 19.42 0 0 1-3.33-2.67m-2.67-3.34a19.79 19.79 0 0 1-3.07-8.63A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91"/><line x1="23" y1="1" x2="1" y2="23"/></svg>
          Decline
        </button>
        
        <button 
          on:click={acceptCall}
          class="flex-1 py-3 px-4 bg-green-500 hover:bg-green-400 text-black rounded-xl font-bold shadow-lg shadow-green-500/20 transition-all hover:scale-105 active:scale-95 flex items-center justify-center gap-2"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7 2 2 0 0 1 1.72 2z"/></svg>
          Accept
        </button>
      </div>
    </div>
  </div>
{/if}
