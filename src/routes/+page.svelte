<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { auth, db, signInAnonymously, updateProfile, doc, setDoc, getDoc, onSnapshot, collection, addDoc, updateDoc, arrayUnion, deleteField, query, where, deleteDoc } from '$lib/firebase';
  import { onAuthStateChanged } from 'firebase/auth';
  import { user, activeChat, settings, pendingCallAccept } from '$lib/stores';
  import { playRingtone, stopRingtone, audioEngine, attachStreamToPeer } from '$lib/audio';
  import { replaceVideoTrack } from '$lib/webrtc';
  
  // Components
  import Chat from '$lib/components/Chat.svelte';
  import Settings from '$lib/components/Settings.svelte';
  
  // Layout Components
  import ActiveCallView from '$lib/components/layout/ActiveCallView.svelte';
  import CallControls from '$lib/components/layout/CallControls.svelte';

  // --- State ---
  // view: 'login' | 'main' | 'chat'
  let view: 'login' | 'main' | 'chat' = 'login';
  
  // Watch for pending call acceptance (if already on page)
  $: if ($pendingCallAccept && $user && view === 'main') {
      handlePendingCall();
  }

  async function handlePendingCall() {
      const chatId = $pendingCallAccept;
      if (!$activeChat || $activeChat.chatId !== chatId) return;

      console.log("Handling pending call accept:", chatId);
      view = 'chat';
      
      // Setup listeners
      if (messagesUnsub) messagesUnsub();
      messagesUnsub = onSnapshot(collection(db, "chats", chatId, "messages"), (snap) => {
          messages = snap.docs.map(d => d.data()).sort((a, b) => a.timestamp - b.timestamp);
      });
      if (signalingUnsub) signalingUnsub();
      subscribeToSignaling(chatId);

      // Trigger Accept
      await acceptCall();
      $pendingCallAccept = null;
  }

  let username = "";
  let myUid = "";
  let friendUidInput = "";
  let friends: any[] = [];
  $: filteredFriends = friendUidInput 
     ? friends.filter(f => (f.displayName || "").toLowerCase().includes(friendUidInput.toLowerCase()) || f.uid.includes(friendUidInput))
     : friends;
  
  // Settings
  let showSettings = false;

  let messages: any[] = [];
  
  // WebRTC & Call State
  let localStream: MediaStream | null = null;
  let remoteStream: MediaStream | null = null;
  let peerConnection: RTCPeerConnection | null = null;
  let dataChannel: RTCDataChannel | null = null;
  let currentCallChat: any = null; // Stores the chat session for the active call
  
  // Call Status Model: idle -> ringing -> accepted/connected -> ended
  let callState: 'idle' | 'ringing' | 'connected' = 'idle'; 
  let incomingCall: { caller: string, callerName: string, chatId: string } | null = null;
  let isCaller = false; // Am I the one who started the call?
  let isNegotiating = false; // Prevents race conditions during offer/answer exchange
  
  // Audio/Video Controls
  let isMuted = false;
  let isDeafened = false;
  let isScreenSharing = false;
  let isMusicActive = false; // Controls Music Player visibility in ActiveCallView

  // UI/UX State
  let isPTTActive = false;

  // File Transfer State
  let receivedBuffers: ArrayBuffer[] = [];
  let receivedSize = 0;
  let incomingFileMeta: any = null;
  let transferProgress = 0;

  const servers = {
    iceServers: [
      { urls: ['stun:stun.l.google.com:19302'] }
    ]
  };

  let signalingUnsub: (() => void) | null = null;
  let globalCallUnsub: (() => void) | null = null;
  let messagesUnsub: (() => void) | null = null;
  let globalMessageUnsub: (() => void) | null = null;
  let settingsUnsub: (() => void) | null = null;
  
  // --- Lifecycle ---

  onMount(() => {
    // Load Settings
    settings.load();

    // Keydown for PTT and Shortcuts
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);

    // Auth Listener
    const authUnsub = onAuthStateChanged(auth, async (u) => {
      user.set(u);
      if (u) {
        myUid = u.uid;
        view = 'main';
        saveUserToDb();
        loadFriends();
        subscribeToGlobalMessages();
      } else {
        view = 'login';
        friends = [];
      }
    });

    return () => {
      authUnsub();
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
      if (signalingUnsub) signalingUnsub();
      if (globalCallUnsub) globalCallUnsub();
      if (messagesUnsub) messagesUnsub();
      if (globalMessageUnsub) globalMessageUnsub();
      if (settingsUnsub) settingsUnsub();
      cleanupCall();
    };
  });

  // --- Logic ---

  function subscribeToGlobalMessages() {
    // Optional: listen for new messages across all chats (if needed for notifications)
    // For now, we rely on individual chat listeners or simplified logic
  }

  // NOTE: subscribeToSignaling and call logic functions (startCall, acceptCall, etc.) are below
  
  function subscribeToSignaling(chatId: string) {
    if (!chatId) return;
    
    signalingUnsub = onSnapshot(doc(db, "chats", chatId), async (snap) => {
      if (!snap.exists()) return;
      const data = snap.data();
      
      // 1. Connection Logic
      if (data.status === 'ringing') {
          // Handled by GlobalCallListener mostly, but we can update local state if we are in the chat
          if (!isCaller && data.caller !== myUid) {
               // We are callee
               incomingCall = { caller: data.caller, callerName: data.callerName || 'Unknown', chatId };
               callState = 'ringing';
          }
      } 
      else if (data.status === 'connected' || data.status === 'accepted') {
          if (callState === 'ringing') {
              // Call accepted
              callState = 'connected';
              stopRingtone();
              
              // If we are caller, we need to handle answer
              if (isCaller && data.answer && !peerConnection?.currentRemoteDescription) {
                  await peerConnection?.setRemoteDescription(new RTCSessionDescription(data.answer));
                  
                  // Send candidates
                  const pendingCandidates = await getDoc(doc(db, "chats", chatId)).then(d => d.data()?.offerCandidates || []);
                  // Actually we send them as they come, but we might need to flush or wait
                  // ICE candidates are handled via onicecandidate
              }
              // If we are callee, we already set remote desc in acceptCall
          }
      }
      else if (data.status === 'rejected') {
        if (callState === 'ringing' || incomingCall) {
          alert("Call rejected.");
          cleanupCall();
        }
      }
      else if (data.status === 'ended') {
        if (callState !== 'idle') {
          cleanupCall();
        }
      }

      // Handle ICE Candidates
      if (peerConnection && callState !== 'idle') {
        if (isCaller && data.answerCandidates) {
           data.answerCandidates.forEach((c: any) => peerConnection?.addIceCandidate(new RTCIceCandidate(c)).catch(e => {}));
        } else if (!isCaller && data.offerCandidates) {
           data.offerCandidates.forEach((c: any) => peerConnection?.addIceCandidate(new RTCIceCandidate(c)).catch(e => {}));
        }
      }
      
      // Handle Renegotiation (Offer/Answer updates during call)
       if (callState === 'connected' && !isNegotiating) {
          if (!isCaller && data.offer && (!peerConnection?.currentRemoteDescription || peerConnection.currentRemoteDescription.sdp !== data.offer.sdp)) {
              // Callee received new offer (renegotiation)
              console.log("Received renegotiation offer");
              // Only process if we are stable
              if (peerConnection?.signalingState === "stable" || peerConnection?.signalingState === "have-remote-offer") {
                  isNegotiating = true;
                  try {
                      await peerConnection.setRemoteDescription(new RTCSessionDescription(data.offer));
                      const answer = await peerConnection.createAnswer();
                      await peerConnection.setLocalDescription(answer);
                      await updateDoc(doc(db, "chats", chatId), {
                          answer: { type: answer.type, sdp: answer.sdp }
                      });
                  } catch(e) { console.error("Renegotiation error (callee):", e); }
                  finally { isNegotiating = false; }
              }
          }
          else if (isCaller && data.answer && (!peerConnection?.currentRemoteDescription || peerConnection.currentRemoteDescription.sdp !== data.answer.sdp)) {
               // Caller received new answer
               if (peerConnection?.signalingState === "have-local-offer") {
                    await peerConnection.setRemoteDescription(new RTCSessionDescription(data.answer));
               }
          }
      }
    });
  }

  async function login() {
    if (!username) return;
    try {
      const cred = await signInAnonymously(auth);
      await updateProfile(cred.user, { displayName: username });
    } catch (e) {
      console.error(e);
      alert("Login failed: " + e);
    }
  }

  async function saveUserToDb() {
    if(!$user) return;
    try {
      await setDoc(doc(db, "users", $user.uid), {
        uid: $user.uid,
        displayName: $user.displayName || username
      }, { merge: true });
    } catch (e) {
      console.error("Error saving user:", e);
    }
  }

  async function addFriend() {
    if (!friendUidInput || friendUidInput === myUid) return;
    
    try {
      const friendDoc = await getDoc(doc(db, "users", friendUidInput));
      if (!friendDoc.exists()) {
        alert("User not found! Make sure the UID is correct.");
        return;
      }
      const friendData = friendDoc.data();
  
      // Add to my friends
      await setDoc(doc(db, "users", myUid, "friends", friendUidInput), friendData);
      
      // Add me to their friends (auto-accept)
      await setDoc(doc(db, "users", friendUidInput, "friends", myUid), {
        uid: myUid,
        displayName: $user.displayName
      });
  
      friendUidInput = "";
    } catch (e) {
      alert("Error adding friend: " + e);
    }
  }

  function loadFriends() {
    onSnapshot(collection(db, "users", myUid, "friends"), (snap) => {
      friends = snap.docs.map(d => {
        const data = d.data();
        return { ...data, uid: data.uid || d.id };
      });
    });
  }

  function getChatId(uid1: string, uid2: string) {
    return uid1 < uid2 ? `${uid1}_${uid2}` : `${uid2}_${uid1}`;
  }

  async function openChat(friend: any) {
    $activeChat = { ...friend, chatId: getChatId(myUid, friend.uid) };
    view = 'chat';
    
    // Subscribe to messages
    if (messagesUnsub) messagesUnsub();
    messagesUnsub = onSnapshot(collection(db, "chats", $activeChat.chatId, "messages"), (snap) => {
      messages = snap.docs
        .map(d => d.data())
        .sort((a, b) => a.timestamp - b.timestamp);
    });

    // Subscribe to signaling
    if (signalingUnsub) signalingUnsub();
    subscribeToSignaling($activeChat.chatId);
  }

  function goBack() {
    view = 'main';
    $activeChat = null;
    
    // Stop listening to messages for the specific chat
    if (messagesUnsub) {
        messagesUnsub();
        messagesUnsub = null;
    }

    // If callState is idle AND no incoming call, we can clean up signaling.
    if (callState === 'idle' && !incomingCall) {
        if (signalingUnsub) {
            signalingUnsub();
            signalingUnsub = null;
        }
        cleanupCall();
    }
  }

  // --- Messaging & File Transfer ---

  async function sendMessage(event: CustomEvent<string>) {
    const text = event.detail;
    if (!text || !$activeChat) return;

    const chatDocRef = doc(db, "chats", $activeChat.chatId);

    // 1. Add message to subcollection
    await addDoc(collection(chatDocRef, "messages"), {
      text: text,
      sender: myUid,
      timestamp: Date.now()
    });

    // 2. Update parent doc for global listeners
    await setDoc(chatDocRef, {
        lastMessage: text,
        lastMessageSender: myUid,
        lastMessageTimestamp: Date.now(),
        participants: [myUid, $activeChat.uid]
    }, { merge: true });
  }

  function setupDataChannel(channel: RTCDataChannel) {
    dataChannel = channel;
    dataChannel.onopen = () => console.log("DataChannel Open");
    dataChannel.onmessage = handleDataChannelMessage;
  }

  function handleDataChannelMessage(event: MessageEvent) {
    const data = event.data;
    if (typeof data === 'string') {
      const msg = JSON.parse(data);
      if (msg.type === 'file-meta') {
        incomingFileMeta = msg;
        receivedBuffers = [];
        receivedSize = 0;
      } else if (msg.type === 'file-end') {
        const blob = new Blob(receivedBuffers, { type: incomingFileMeta.mime });
        const url = URL.createObjectURL(blob);
        messages = [...messages, {
          text: `Received file: ${incomingFileMeta.name}`,
          imageUrl: incomingFileMeta.mime.startsWith('image/') ? url : null,
          sender: 'system',
          timestamp: Date.now()
        }];
        incomingFileMeta = null;
        receivedBuffers = [];
      }
    } else {
      receivedBuffers.push(data);
      receivedSize += data.byteLength;
      if (incomingFileMeta) transferProgress = (receivedSize / incomingFileMeta.size) * 100;
    }
  }

  async function handleSendFile(event: CustomEvent<File>) {
    const file = event.detail;
    if (callState !== 'connected') {
        alert("Please CALL first to send files (P2P).");
        return;
    }

    if (!dataChannel || dataChannel.readyState !== 'open') {
      alert("P2P Connection not ready.");
      return;
    }
    dataChannel.send(JSON.stringify({ type: 'file-meta', name: file.name, size: file.size, mime: file.type }));
    
    const chunkSize = 16384;
    const reader = new FileReader();
    let offset = 0;

    reader.onload = (e) => {
      if (e.target?.result) {
        dataChannel?.send(e.target.result as ArrayBuffer);
        offset += chunkSize;
        if (offset < file.size) {
          readSlice(offset);
        } else {
          dataChannel?.send(JSON.stringify({ type: 'file-end' }));
          messages = [...messages, { text: `Sent file: ${file.name}`, sender: 'system', timestamp: Date.now() }];
        }
      }
    };
    const readSlice = (o: number) => {
      const slice = file.slice(o, o + chunkSize);
      reader.readAsArrayBuffer(slice);
    };
    readSlice(0);
  }

  // --- Call Logic ---

  async function startCall(video: boolean = false, existingStream?: MediaStream) {
    if (!$activeChat || !$activeChat.uid) {
        console.error("Cannot start call: invalid active chat (missing uid)", $activeChat);
        alert("Cannot start call: user data is incomplete. Please try re-adding the friend.");
        return;
    }
    currentCallChat = $activeChat;
    isCaller = true;
    callState = 'ringing';
    
    try {
      if (existingStream) {
         localStream = existingStream;
      } else {
         const constraints = {
            audio: {
                deviceId: $settings.selectedMicId ? { exact: $settings.selectedMicId } : undefined,
                noiseSuppression: $settings.noiseSuppression,
                echoCancellation: $settings.echoCancellation,
                autoGainControl: $settings.autoGainControl
            },
            video: video
         };
         
         const processedAudioStream = await audioEngine.setInput(constraints.audio);
         
         if (video) {
             const videoStream = await navigator.mediaDevices.getUserMedia({ video: true });
             localStream = new MediaStream([
                 ...processedAudioStream.getAudioTracks(),
                 ...videoStream.getVideoTracks()
             ]);
         } else {
             localStream = processedAudioStream;
         }
      }
      isScreenSharing = video;
      
      peerConnection = new RTCPeerConnection(servers);
      
      if (localStream) {
        attachStreamToPeer(peerConnection, localStream);
      }
      
      if ($settings.isPTTEnabled) {
        audioEngine.setPTTActive(false);
      }
      
      const dc = peerConnection.createDataChannel("files");
      setupDataChannel(dc);

      peerConnection.onnegotiationneeded = async () => {
         console.log("Negotiation needed");
         if (!isCaller || callState !== 'connected') return;
         try {
             const offer = await peerConnection!.createOffer();
             await peerConnection!.setLocalDescription(offer);
             await updateDoc(doc(db, "chats", currentCallChat.chatId), {
                 offer: { type: offer.type, sdp: offer.sdp },
                 status: 'connected'
             });
         } catch(e) { console.error("Renegotiation error:", e); }
      };

      peerConnection.ontrack = (event) => {
        remoteStream = event.streams[0];
      };

      peerConnection.onicecandidate = (event) => {
        if (event.candidate) {
          updateDoc(doc(db, "chats", currentCallChat.chatId), {
            offerCandidates: arrayUnion(event.candidate.toJSON())
          });
        }
      };

      const offer = await peerConnection.createOffer();
      await peerConnection.setLocalDescription(offer);

      await setDoc(doc(db, "chats", currentCallChat.chatId), {
        caller: myUid,
        callerName: $user?.displayName,
        to: $activeChat.uid,
        offer: { type: offer.type, sdp: offer.sdp },
        status: 'ringing',
        timestamp: Date.now()
      }, { merge: true });

    } catch (e) {
      console.error("Call error:", e);
      cleanupCall();
    }
  }

  async function acceptCall() {
    if ((!incomingCall && !$pendingCallAccept) || !$activeChat) return;
    
    currentCallChat = $activeChat;
    
    const chatDoc = await getDoc(doc(db, "chats", currentCallChat.chatId));
    const data = chatDoc.data();
    if (!data || !data.offer) {
      alert("Call expired or invalid.");
      cleanupCall();
      return;
    }

    incomingCall = null;
    callState = 'connected';
    isCaller = false;
    stopRingtone();

    try {
      const constraints = {
          audio: {
              deviceId: $settings.selectedMicId ? { exact: $settings.selectedMicId } : undefined,
              noiseSuppression: $settings.noiseSuppression,
              echoCancellation: $settings.echoCancellation,
              autoGainControl: $settings.autoGainControl
          },
          video: false
      };
      
      const processedAudioStream = await audioEngine.setInput(constraints.audio);
      localStream = processedAudioStream;
      
      peerConnection = new RTCPeerConnection(servers);

      if (localStream) {
         attachStreamToPeer(peerConnection, localStream);
      }

      if ($settings.isPTTEnabled) {
        audioEngine.setPTTActive(false);
      }

      peerConnection.ontrack = (event) => {
        remoteStream = event.streams[0];
      };

      peerConnection.ondatachannel = (event) => setupDataChannel(event.channel);

      peerConnection.onicecandidate = (event) => {
        if (event.candidate) {
          updateDoc(doc(db, "chats", currentCallChat.chatId), {
            answerCandidates: arrayUnion(event.candidate.toJSON())
          });
        }
      };

      await peerConnection.setRemoteDescription(new RTCSessionDescription(data.offer));
      const answer = await peerConnection.createAnswer();
      await peerConnection.setLocalDescription(answer);

      await updateDoc(doc(db, "chats", currentCallChat.chatId), {
        status: 'accepted',
        answer: { type: answer.type, sdp: answer.sdp }
      });

    } catch (e) {
      console.error("Accept error:", e);
      cleanupCall();
    } finally {
        isNegotiating = false;
    }
  }

  async function rejectCall() {
    if (!incomingCall && !$activeChat) return;
    const cid = incomingCall?.chatId || $activeChat?.chatId;
    if (!cid) return;

    incomingCall = null;
    await updateDoc(doc(db, "chats", cid), {
      status: 'rejected'
    });
    cleanupCall();
  }

  async function endCall() {
    const chat = currentCallChat || $activeChat;
    if (!chat) return;
    
    try {
        await updateDoc(doc(db, "chats", chat.chatId), {
            status: 'ended'
        });
        setTimeout(async () => {
            try { await deleteDoc(doc(db, "chats", chat.chatId)); } catch (e) {}
        }, 2000);
    } catch(e) {}
    cleanupCall();
  }

  function cleanupCall() {
    callState = 'idle';
    currentCallChat = null;
    incomingCall = null;
    isCaller = false;
    isNegotiating = false;
    isMuted = false;
    isDeafened = false;
    isScreenSharing = false;
    isPTTActive = false;
    stopRingtone();
    audioEngine.stopInput();

    if (peerConnection) {
      peerConnection.close();
      peerConnection = null;
    }
    if (localStream) {
      localStream.getTracks().forEach(t => t.stop());
      localStream = null;
    }
    remoteStream = null;
  }

  // --- In-Call Controls ---

  function toggleMute() {
    isMuted = !isMuted;
    audioEngine.setMute(isMuted);
  }

  function toggleDeaf() {
    isDeafened = !isDeafened;
    // Deafen logic implementation left to specific needs, handled visually for now
  }

  async function startScreenShare() {
     if (!navigator.mediaDevices || !navigator.mediaDevices.getDisplayMedia) {
         alert("Screen sharing is not supported in this browser.");
         return;
     }

     if (isScreenSharing) {
         // Stop Screen Share
         // Revert to just audio (mic)
         // This logic is complex, for now we just toggle flag and rely on endCall/cleanup or simple switch
         // But let's implement basic stop
         isScreenSharing = false;
         // Ideally we switch back to Mic only video=false.
         // For simplicity in this iteration:
         alert("To stop screen share, please re-join the call or toggle properly (implementation pending).");
         return;
     }

     let stream: MediaStream | null = null;
     try {
        stream = await navigator.mediaDevices.getDisplayMedia({ video: true, audio: $settings.shareSystemAudio });
        if (!stream) throw new Error("No stream");

        const videoTrack = stream.getVideoTracks()[0];
        videoTrack.onended = () => { isScreenSharing = false; };

        if (callState === 'idle') {
             if (stream.getAudioTracks().length === 0) {
                  try {
                      const micStream = await navigator.mediaDevices.getUserMedia({ audio: true });
                      stream.addTrack(micStream.getAudioTracks()[0]);
                  } catch(e) {}
             }
             await startCall(true, stream); 
        } else {
             if (peerConnection) {
                  if (!localStream) localStream = stream;
                  await replaceVideoTrack(videoTrack, peerConnection, localStream);
                  isScreenSharing = true;
             }
        }
     } catch (e: any) {
         console.error("Screen share error:", e);
     }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.repeat) return;
    const target = e.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) return;

    const key = e.key.toLowerCase();
    if ($settings.isPTTEnabled && key === $settings.pttKey.toLowerCase()) {
        isPTTActive = true;
        audioEngine.setPTTActive(true);
    }
    if (key === $settings.shortcuts.mute.toLowerCase()) toggleMute();
    if (key === $settings.shortcuts.deafen.toLowerCase()) toggleDeaf();
    if (key === $settings.shortcuts.end.toLowerCase() && callState !== 'idle') endCall();
  }

  function handleKeyUp(e: KeyboardEvent) {
      if ($settings.isPTTEnabled && e.key.toLowerCase() === $settings.pttKey.toLowerCase()) {
          isPTTActive = false;
          audioEngine.setPTTActive(false);
      }
  }

</script>

{#if view === 'login'}
<div class="flex items-center justify-center min-h-screen bg-obsidian text-white">
  <div class="bg-surface p-8 rounded-2xl shadow-2xl w-full max-w-md border border-white/5">
    <h1 class="text-3xl font-bold mb-6 text-center tracking-tight bg-gradient-to-r from-accent to-blue-500 bg-clip-text text-transparent">VOID LOGIN</h1>
    <input 
      type="text" 
      bind:value={username} 
      placeholder="Enter Username" 
      class="w-full bg-black/50 border border-white/10 rounded-xl px-4 py-3 mb-4 focus:ring-2 focus:ring-accent focus:outline-none transition-all placeholder-gray-600"
    />
    <button 
      on:click={login} 
      class="w-full bg-accent hover:bg-cyan-400 text-black font-bold py-3 rounded-xl transition-all shadow-lg shadow-accent/20 hover:shadow-accent/40"
    >
      Enter Void
    </button>
  </div>
</div>
{:else}
<!-- MAIN LAYOUT CONTAINER -->
<div class="flex h-screen w-screen overflow-hidden bg-obsidian text-text-primary font-sans selection:bg-accent/30 selection:text-white">
   
   <!-- CENTER STAGE -->
   <main class="flex-1 relative flex flex-col min-w-0 bg-surface/50 overflow-hidden">
      
      <!-- Top Bar (Optional, can be integrated into views) -->
      
      {#if view === 'main'}
         <!-- Friends List View -->
         <div class="flex-1 overflow-y-auto p-8 custom-scrollbar">
            <div class="max-w-4xl mx-auto">
                <header class="flex items-center justify-between mb-8">
                    <h2 class="text-3xl font-bold text-white tracking-tight">Friends</h2>
                    <div class="flex space-x-2">
                        <button on:click={() => showSettings = true} class="p-2 rounded-lg bg-surface hover:bg-surface-hover text-gray-400 hover:text-white transition-colors">
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg>
                        </button>
                    </div>
                </header>

                <div class="bg-surface rounded-2xl p-6 mb-8 border border-white/5 shadow-lg">
                    <h3 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">Add Friend</h3>
                    <div class="flex space-x-4">
                        <input 
                            type="text" 
                            bind:value={friendUidInput} 
                            placeholder="Enter Friend UID" 
                            class="flex-1 bg-black/30 border border-white/10 rounded-xl px-4 py-2 text-white focus:ring-1 focus:ring-accent focus:outline-none transition-all"
                        />
                        <button 
                            on:click={addFriend}
                            class="bg-accent/10 hover:bg-accent/20 text-accent border border-accent/20 px-6 py-2 rounded-xl font-medium transition-all"
                        >
                            Add Friend
                        </button>
                    </div>
                    <div class="mt-2 text-xs text-gray-500 font-mono">Your UID: {myUid}</div>
                </div>

                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                    {#each filteredFriends as friend}
                        <button 
                            on:click={() => openChat(friend)}
                            class="flex items-center p-4 bg-surface hover:bg-surface-hover border border-white/5 rounded-2xl transition-all group text-left"
                        >
                            <div class="w-12 h-12 rounded-xl bg-gradient-to-br from-gray-700 to-gray-900 flex items-center justify-center text-lg font-bold text-white shadow-inner">
                                {friend.displayName?.slice(0, 2).toUpperCase()}
                            </div>
                            <div class="ml-4">
                                <div class="font-medium text-white group-hover:text-accent transition-colors">{friend.displayName}</div>
                                <div class="text-xs text-gray-500 truncate w-32">{friend.uid}</div>
                            </div>
                        </button>
                    {/each}
                </div>
            </div>
         </div>
      {:else if view === 'chat'}
         {#if callState === 'connected'}
             <!-- Active Call View -->
             <div class="flex-1 flex flex-col relative overflow-hidden">
                 <ActiveCallView 
                    {localStream} 
                    {remoteStream} 
                    {isScreenSharing}
                    activeChat={$activeChat}
                    {isMusicActive}
                    {friends}
                    onCloseMusic={() => isMusicActive = false}
                 />
                 <!-- Fixed Bottom Controls -->
                 <div class="absolute bottom-8 left-1/2 -translate-x-1/2 z-50">
                    <CallControls 
                        {isMuted} {isDeafened} {isScreenSharing} {isMusicActive}
                        on:toggleMute={toggleMute}
                        on:toggleDeafen={toggleDeafen}
                        on:toggleScreenShare={startScreenShare}
                        on:toggleMusic={() => isMusicActive = !isMusicActive}
                        on:endCall={endCall}
                    />
                 </div>
             </div>
         {:else}
             <!-- Text Chat View -->
             <div class="flex flex-col h-full">
                 <!-- Chat Header -->
                 <div class="h-16 bg-surface border-b border-white/5 flex items-center justify-between px-6 shrink-0 z-10">
                     <div class="flex items-center space-x-3">
                         <button on:click={goBack} class="p-2 rounded-lg hover:bg-white/10 text-gray-400 hover:text-white transition-colors mr-1">
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" /></svg>
                         </button>
                         <div class="w-10 h-10 rounded-full bg-gradient-to-br from-accent to-blue-500 flex items-center justify-center text-black font-bold">
                             {$activeChat?.displayName?.slice(0, 2).toUpperCase()}
                         </div>
                         <div>
                             <h3 class="font-bold text-white">{$activeChat?.displayName}</h3>
                             <span class="text-xs text-green-400 flex items-center gap-1">
                                <span class="w-1.5 h-1.5 rounded-full bg-green-400"></span> Online
                             </span>
                         </div>
                     </div>
                     <div class="flex items-center space-x-2">
                         <button on:click={() => startCall(false)} class="p-2 rounded-lg hover:bg-white/10 text-gray-400 hover:text-white transition-colors" title="Voice Call">
                             <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5a2 2 0 012-2h3.28a1 1 0 01.948.684l1.498 4.493a1 1 0 01-.502 1.21l-2.257 1.13a11.042 11.042 0 005.516 5.516l1.13-2.257a1 1 0 011.21-.502l4.493 1.498a1 1 0 01.684.949V19a2 2 0 01-2 2h-1C9.716 21 3 14.284 3 6V5z" /></svg>
                         </button>
                         <button on:click={() => startCall(true)} class="p-2 rounded-lg hover:bg-white/10 text-gray-400 hover:text-white transition-colors" title="Video Call">
                             <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
                         </button>
                     </div>
                 </div>
                 
                 <Chat 
                    chatId={$activeChat.chatId} 
                    friendName={$activeChat.friendName} 
                    messages={messages}
                    {transferProgress}
                    on:sendMessage={sendMessage}
                    on:sendFile={handleSendFile}
                 />
             </div>
         {/if}
      {/if}

      <!-- Incoming Call Overlay -->
      {#if incomingCall}
        <div class="absolute inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-[100]">
           <div class="bg-surface p-8 rounded-3xl shadow-2xl border border-white/10 flex flex-col items-center animate-bounce-slow">
               <div class="w-24 h-24 rounded-full bg-surface-hover mb-6 flex items-center justify-center ring-4 ring-accent/20">
                   <span class="text-3xl font-bold text-accent">{incomingCall.callerName.slice(0, 2).toUpperCase()}</span>
               </div>
               <h3 class="text-2xl font-bold text-white mb-2">{incomingCall.callerName}</h3>
               <p class="text-gray-400 mb-8">Incoming Call...</p>
               <div class="flex space-x-6">
                   <button on:click={rejectCall} class="w-16 h-16 rounded-full bg-red-500/20 text-red-500 hover:bg-red-500 hover:text-white flex items-center justify-center transition-all ring-1 ring-red-500/50">
                       <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /></svg>
                   </button>
                   <button on:click={acceptCall} class="w-16 h-16 rounded-full bg-green-500/20 text-green-500 hover:bg-green-500 hover:text-white flex items-center justify-center transition-all ring-1 ring-green-500/50 animate-pulse">
                       <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5a2 2 0 012-2h3.28a1 1 0 01.948.684l1.498 4.493a1 1 0 01-.502 1.21l-2.257 1.13a11.042 11.042 0 005.516 5.516l1.13-2.257a1 1 0 011.21-.502l4.493 1.498a1 1 0 01.684.949V19a2 2 0 01-2 2h-1C9.716 21 3 14.284 3 6V5z" /></svg>
                   </button>
               </div>
           </div>
        </div>
      {/if}

   </main>

</div>
{/if}

{#if showSettings}
  <div class="fixed inset-0 z-[200]">
      <Settings on:close={() => showSettings = false} />
  </div>
{/if}

<style>
  /* Custom scrollbar for local elements */
  .custom-scrollbar::-webkit-scrollbar { width: 6px; }
  .custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
  .custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.1); border-radius: 10px; }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.2); }
  
  /* Animations */
  @keyframes bounce-slow {
      0%, 100% { transform: translateY(0); }
      50% { transform: translateY(-5px); }
  }
  .animate-bounce-slow {
      animation: bounce-slow 2s infinite ease-in-out;
  }
</style>
