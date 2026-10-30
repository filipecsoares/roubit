<script setup lang="ts">
import { Target, TrendingUp, Plus } from 'lucide-vue-next'
</script>

<template>
  <div class="space-y-8 animate-fade-in">
    <header class="flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div>
        <h1 class="text-3xl font-extrabold tracking-tight text-white font-outfit">
          Objetivos & OKRs
        </h1>
        <p class="text-sm text-slate-400 mt-1">
          Alinhe objetivos de longo prazo diretamente aos seus hábitos diários com cálculo em cascata ponderado.
        </p>
      </div>
      <button
        class="inline-flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-semibold bg-brand-600 hover:bg-brand-500 text-white shadow-lg shadow-brand-500/20 transition-all"
      >
        <Plus class="w-4 h-4" />
        Novo Objetivo
      </button>
    </header>

    <div class="glass-panel p-6 rounded-2xl border border-surface-border space-y-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="p-2.5 rounded-xl bg-brand-500/10 text-brand-400 border border-brand-500/20">
            <Target class="w-5 h-5" />
          </div>
          <div>
            <h2 class="text-lg font-bold text-white">Dominar Arquitetura Web em Rust</h2>
            <p class="text-xs text-slate-400">Trimestre: Q4-2026</p>
          </div>
        </div>
        <span class="text-base font-extrabold text-brand-400 font-mono">75%</span>
      </div>

      <!-- Progress bar -->
      <div class="w-full h-2 rounded-full bg-slate-800 overflow-hidden">
        <div class="h-full bg-gradient-to-r from-brand-500 to-indigo-400 rounded-full" style="width: 75%"></div>
      </div>

      <div class="pt-3 border-t border-surface-border text-xs text-slate-400 flex items-center justify-between">
        <span class="inline-flex items-center gap-1">
          <TrendingUp class="w-3.5 h-3.5 text-emerald-400" />
          1 Resultado-Chave vinculado a hábitos ativos
        </span>
        <span>Prazo: 31 Dez 2026</span>
      </div>
    </div>
  </div>
</template>
