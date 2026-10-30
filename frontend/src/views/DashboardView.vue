<script setup lang="ts">
import { Clock, CheckCircle2, Target, Lock } from 'lucide-vue-next'
</script>

<template>
  <div class="space-y-8 animate-fade-in">
    <!-- Header -->
    <header class="flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div>
        <h1 class="text-3xl font-extrabold tracking-tight text-white font-outfit">
          Painel Diário
        </h1>
        <p class="text-sm text-slate-400 mt-1">
          Acompanhe o cronograma de blocos, hábitos ativos e o progresso trimestral dos seus OKRs.
        </p>
      </div>
      <div class="flex items-center gap-2">
        <span class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
          Sistema Operacional
        </span>
      </div>
    </header>

    <!-- Quick Stats Grid -->
    <section class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
      <div class="glass-panel p-5 rounded-xl border border-surface-border">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Rotina Atual</span>
          <Clock class="w-4 h-4 text-brand-400" />
        </div>
        <p class="mt-2 text-2xl font-bold text-white">Foco Profundo</p>
        <p class="text-xs text-slate-400 mt-1">Bloco atual: 08:30 – 10:30</p>
      </div>

      <div class="glass-panel p-5 rounded-xl border border-surface-border">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Hábitos do Dia</span>
          <CheckCircle2 class="w-4 h-4 text-emerald-400" />
        </div>
        <p class="mt-2 text-2xl font-bold text-white">4 / 5</p>
        <p class="text-xs text-slate-400 mt-1">80% de conclusão diária</p>
      </div>

      <div class="glass-panel p-5 rounded-xl border border-surface-border">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Meta Trimestral</span>
          <Target class="w-4 h-4 text-amber-400" />
        </div>
        <p class="mt-2 text-2xl font-bold text-white">74.2%</p>
        <p class="text-xs text-slate-400 mt-1">Q4-2026 Objetivo Principal</p>
      </div>

      <div class="glass-panel p-5 rounded-xl border border-surface-border">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Diário Zero-Knowledge</span>
          <Lock class="w-4 h-4 text-indigo-400" />
        </div>
        <p class="mt-2 text-2xl font-bold text-white">Criptografado</p>
        <p class="text-xs text-slate-400 mt-1">AES-256-GCM ativo</p>
      </div>
    </section>

    <!-- Main Content Placeholder Card -->
    <section class="glass-panel p-8 rounded-2xl border border-surface-border text-center">
      <div class="max-w-md mx-auto space-y-4">
        <div class="w-12 h-12 mx-auto rounded-full bg-brand-500/10 flex items-center justify-center border border-brand-500/20 text-brand-400">
          <Clock class="w-6 h-6" />
        </div>
        <h2 class="text-xl font-bold text-white">Cronograma do Dia</h2>
        <p class="text-sm text-slate-400">
          O scaffolding da interface Vue 3 foi concluído com sucesso. Os componentes reativos de rotinas e hábitos serão ativados nas próximas atividades.
        </p>
      </div>
    </section>
  </div>
</template>
