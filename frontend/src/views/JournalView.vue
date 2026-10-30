<script setup lang="ts">
import { Lock, ShieldCheck, KeyRound } from 'lucide-vue-next'
</script>

<template>
  <div class="space-y-8 animate-fade-in">
    <header class="flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div>
        <h1 class="text-3xl font-extrabold tracking-tight text-white font-outfit">
          Diário Criptografado
        </h1>
        <p class="text-sm text-slate-400 mt-1">
          Escrita reflexiva com garantia estrita de Zero-Knowledge: cifragem local cliente-side com AES-256-GCM e Argon2id.
        </p>
      </div>
      <div class="flex items-center gap-2">
        <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
          <ShieldCheck class="w-4 h-4" />
          Zero-Knowledge Ativo
        </span>
      </div>
    </header>

    <div class="glass-panel p-8 rounded-2xl border border-surface-border text-center max-w-lg mx-auto space-y-4">
      <div class="w-12 h-12 mx-auto rounded-full bg-indigo-500/10 flex items-center justify-center border border-indigo-500/20 text-indigo-400">
        <Lock class="w-6 h-6" />
      </div>
      <h2 class="text-xl font-bold text-white">Cofre de Reflexões Fechado</h2>
      <p class="text-sm text-slate-400">
        Insira sua frase-secreta de desbloqueio para derivar a chave mestra em memória via Web Crypto API. O servidor armazena apenas dados cifrados.
      </p>
      <div class="pt-2">
        <button
          class="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl text-sm font-semibold bg-brand-600 hover:bg-brand-500 text-white shadow-lg shadow-brand-500/20 transition-all"
        >
          <KeyRound class="w-4 h-4" />
          Desbloquear Diário
        </button>
      </div>
    </div>
  </div>
</template>
