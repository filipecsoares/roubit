<script setup lang="ts">
import { ref, computed } from 'vue'
import { ChevronDown, ChevronUp, Target, Trash2 } from 'lucide-vue-next'
import KeyResultProgressBar from './KeyResultProgressBar.vue'
import type { Objective, KeyResult } from '@/types'

const props = withDefaults(
  defineProps<{
    objective: Objective
    keyResults: KeyResult[]
    habitsMap?: Record<string, string>
  }>(),
  {
    habitsMap: () => ({}),
  }
)

const emit = defineEmits<{
  (e: 'linkHabit', krId: string): void
  (e: 'delete', objectiveId: string): void
}>()

const isExpanded = ref(true)

const progressClamped = computed(() => {
  return Math.min(100, Math.max(0, Math.round(props.objective.progress_percentage)))
})

const badgeStatusClasses = computed(() => {
  if (props.objective.is_completed || progressClamped.value >= 100) {
    return 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
  }
  if (progressClamped.value >= 70) {
    return 'bg-brand-500/10 text-brand-300 border-brand-500/20'
  }
  if (progressClamped.value >= 30) {
    return 'bg-amber-500/10 text-amber-400 border-amber-500/20'
  }
  return 'bg-slate-500/10 text-slate-400 border-slate-500/20'
})
</script>

<template>
  <div class="rounded-2xl bg-surface-card border border-surface-border overflow-hidden transition-all duration-200 hover:border-slate-700 shadow-sm">
    <!-- Card Header -->
    <div
      class="p-5 flex items-start justify-between gap-4 cursor-pointer select-none"
      @click="isExpanded = !isExpanded"
    >
      <div class="space-y-1.5 min-w-0">
        <div class="flex items-center gap-2">
          <!-- Quarter Badge -->
          <span class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-brand-500/10 text-brand-300 border border-brand-500/20">
            <Target class="w-3 h-3" />
            {{ objective.quarter }}
          </span>

          <!-- Status Badge -->
          <span class="px-2.5 py-0.5 rounded-full text-xs font-medium border" :class="badgeStatusClasses">
            {{ progressClamped }}% Concluído
          </span>
        </div>

        <h3 class="text-base font-bold text-white tracking-tight">
          {{ objective.title }}
        </h3>

        <p v-if="objective.description" class="text-xs text-slate-400 line-clamp-2">
          {{ objective.description }}
        </p>
      </div>

      <!-- Right: Toggle & Overall Progress Bar -->
      <div class="flex items-center gap-4 shrink-0">
        <div class="hidden sm:flex flex-col items-end gap-1">
          <span class="text-xs text-slate-400 font-mono">{{ keyResults.length }} Resultados-Chave</span>
          <div class="w-24 h-2 rounded-full bg-surface-border/40 overflow-hidden">
            <div
              class="h-full rounded-full bg-brand-500 transition-all duration-500"
              :style="{ width: `${progressClamped}%` }"
            />
          </div>
        </div>

        <button
          type="button"
          @click.stop="emit('delete', objective.id)"
          title="Excluir objetivo"
          class="p-1.5 rounded-lg text-slate-500 hover:text-rose-400 hover:bg-rose-500/10 transition-colors"
        >
          <Trash2 class="w-4 h-4" />
        </button>

        <button
          type="button"
          class="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors"
        >
          <ChevronUp v-if="isExpanded" class="w-5 h-5" />
          <ChevronDown v-else class="w-5 h-5" />
        </button>
      </div>
    </div>

    <!-- Expandable Key Results Section -->
    <div v-show="isExpanded" class="px-5 pb-5 pt-1 border-t border-surface-border/40 space-y-3">
      <div v-if="keyResults.length === 0" class="text-xs text-slate-500 py-2 italic">
        Nenhum Resultado-Chave cadastrado para este objetivo.
      </div>
      <div v-else class="space-y-2.5">
        <KeyResultProgressBar
          v-for="kr in keyResults"
          :key="kr.id"
          :kr="kr"
          :linked-habit-name="kr.linked_habit_id ? habitsMap[kr.linked_habit_id] : undefined"
          @link-habit="(id) => emit('linkHabit', id)"
        />
      </div>
    </div>
  </div>
</template>
