<script setup lang="ts">
import { Flame, Plus, Check } from 'lucide-vue-next'
</script>

<template>
  <div class="space-y-8 animate-fade-in">
    <header class="flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div>
        <h1 class="text-3xl font-extrabold tracking-tight text-white font-outfit">
          Rastreamento de Hábitos
        </h1>
        <p class="text-sm text-slate-400 mt-1">
          Mantenha a consistência com check-ins atômicos e visualize suas sequências ininterruptas (*streaks*).
        </p>
      </div>
      <button
        class="inline-flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-semibold bg-brand-600 hover:bg-brand-500 text-white shadow-lg shadow-brand-500/20 transition-all"
      >
        <Plus class="w-4 h-4" />
        Novo Hábito
      </button>
    </header>

    <!-- Habit Cards -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <div class="glass-panel p-5 rounded-2xl border border-surface-border space-y-4">
        <div class="flex items-start justify-between">
          <div>
            <h3 class="text-base font-bold text-white">Beber Água</h3>
            <p class="text-xs text-slate-400">Meta: 2.5 Litros / dia</p>
          </div>
          <span class="inline-flex items-center gap-1 text-xs font-semibold px-2.5 py-1 rounded-lg bg-amber-500/10 text-amber-400 border border-amber-500/20">
            <Flame class="w-3.5 h-3.5" />
            14 dias
          </span>
        </div>

        <div class="flex items-center justify-between pt-2 border-t border-surface-border">
          <span class="text-xs text-slate-400">Hoje: 2.0 / 2.5L</span>
          <button class="p-2 rounded-lg bg-emerald-500/20 hover:bg-emerald-500/30 text-emerald-400 transition-colors">
            <Check class="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
