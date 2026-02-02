<script lang="ts">
  import { createEventDispatcher, afterUpdate } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { activeChat, user } from '$lib/stores';

  export let messages: any[] = [];
  export let transferProgress = 0;

  let messageInput = "";
  let fileInput: FileList | null = null;
  let chatContainer: HTMLElement;

  const dispatch = createEventDispatcher();

  function handleSend() {
    if (messageInput.trim()) {
      dispatch('sendMessage', messageInput);
      messageInput = "";
    }
  }

  function handleFileSelect() {
    if (fileInput && fileInput.length > 0) {
      dispatch('sendFile', fileInput[0]);
      fileInput = null;
    }
  }

  function formatTime(ts: number) {
    return new Date(ts).toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'});
  }

  afterUpdate(() => {
    if (chatContainer) chatContainer.scrollTop = chatContainer.scrollHeight;
  });
</script>

<div class="flex flex-col h-full bg-[#0d1117] relative overflow-hidden">
  
  <!-- Messages Area -->
  <div bind:this={chatContainer} class="flex-1 overflow-y-auto p-4 space-y-4 custom-scrollbar pb-24 scroll-smooth">
    {#each messages as msg (msg.timestamp)}
      <div 
        class={`flex flex-col ${msg.sender === $user?.uid ? 'items-end' : 'items-start'}`}
        transition:fly={{ y: 10, duration: 200 }}
      >
         <div class={`max-w-[85%] p-3 rounded-2xl text-sm md:text-base ${msg.sender === $user?.uid ? 'bg-blue-600 text-white rounded-br-none' : 'bg-gray-800 text-gray-200 rounded-bl-none'} ${msg.sender === 'system' ? 'bg-transparent border border-gray-700 text-gray-500 italic text-xs w-full text-center' : 'shadow-md'}`}>
            {#if msg.imageUrl}
              <img src={msg.imageUrl} alt="attachment" class="max-w-full rounded-lg mb-2 border border-white/10" />
            {/if}
            {#if msg.text}
              <p class="leading-relaxed break-words whitespace-pre-wrap">{msg.text}</p>
            {/if}
         </div>
         {#if msg.sender !== 'system'}
            <span class="text-[10px] text-gray-600 mt-1 px-1 opacity-70">{formatTime(msg.timestamp)}</span>
         {/if}
      </div>
    {/each}
    
    {#if transferProgress > 0 && transferProgress < 100}
       <div class="w-full bg-gray-800/50 rounded-full h-1 mt-4 overflow-hidden border border-gray-700">
          <div class="bg-blue-500 h-full transition-all duration-300 relative" style="width: {transferProgress}%">
            <div class="absolute inset-0 bg-white/20 animate-pulse"></div>
          </div>
       </div>
       <p class="text-[10px] text-center text-blue-400 mt-1 font-mono">UPLOADING... {Math.round(transferProgress)}%</p>
    {/if}
  </div>

  <!-- Input Area -->
  <div class="p-4 bg-[#161b22] border-t border-gray-800 absolute bottom-0 w-full z-20 backdrop-blur-sm bg-opacity-95">
    
    <!-- File Preview -->
    {#if fileInput && fileInput.length > 0}
         <div class="flex items-center bg-gray-800 px-3 py-2 rounded-lg mb-2 border border-gray-700 w-fit" transition:fly={{ y: 10 }}>
            <span class="text-xs text-blue-300 truncate max-w-[200px] mr-2">📎 {fileInput[0].name}</span>
            <button on:click={() => fileInput = null} class="text-gray-400 hover:text-red-400 transition">✕</button>
         </div>
    {/if}

    <div class="flex items-center space-x-3">
      <label class="cursor-pointer text-gray-400 hover:text-blue-400 transition p-2 hover:bg-gray-800 rounded-full group relative">
         <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 transform group-hover:rotate-45 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.172 7l-6.586 6.586a2 2 0 102.828 2.828l6.414-6.586a4 4 0 00-5.656-5.656l-6.415 6.585a6 6 0 108.486 8.486L20.5 13" />
         </svg>
         <input type="file" bind:files={fileInput} on:change={handleFileSelect} class="hidden" />
      </label>
      
      <div class="relative flex-1">
        <input 
          type="text" 
          bind:value={messageInput} 
          on:keydown={(e) => e.key === 'Enter' && handleSend()}
          placeholder={`Message ${$activeChat?.displayName || ''}...`} 
          class="w-full bg-black text-white rounded-xl pl-4 pr-10 py-3 outline-none focus:ring-1 focus:ring-blue-500 transition border border-gray-800 placeholder-gray-600" 
        />
        <button 
           class="absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-500 hover:text-yellow-400 transition"
           title="Add Emoji"
        >
           😊
        </button>
      </div>
      
      <button 
        on:click={handleSend} 
        class="bg-blue-600 text-white p-3 rounded-xl hover:bg-blue-500 transition shadow-lg hover:shadow-blue-500/20 active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed"
        disabled={!messageInput.trim()}
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
            <path d="M10.894 2.553a1 1 0 00-1.788 0l-7 14a1 1 0 001.169 1.409l5-1.429A1 1 0 009 15.571V11a1 1 0 112 0v4.571a1 1 0 00.725.962l5 1.428a1 1 0 001.17-1.408l-7-14z" />
        </svg>
      </button>
    </div>
  </div>
</div>

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 6px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent; 
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: #374151; 
    border-radius: 3px;
  }
</style>
