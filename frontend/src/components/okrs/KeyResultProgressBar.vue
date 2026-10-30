<script setup lang="ts">
import { computed } from 'vue'
import { Link2 } from 'lucide-vue-next'
import type { KeyResult } from '@/types'

const props = defineProps<{
  kr: KeyResult
  linkedHabitName?: string
}>()

const emit = defineEmits<{
  (e: 'linkHabit', krId: string): void
}>()

const progressClamped = computed(() => {
  return Math.min(100, Math.max(0, Math.round(props.kr.progress_percentage)))
})

const barColor = computed(() => {
  if (progressClamped.value >= 100) return 'bg-emerald-500'
  if (progressClamped.value >= 70) return 'bg-brand-500'
  if (progressClamped.value >= 30) return 'bg-amber-500'
  return 'bg-slate-500'
})
</script>

<template>
  <div class="p-3.5 rounded-xl bg-surface-dark/50 border border-surface-border/60 space-y-2">
    <!-- Header: Title & Value -->
    <div class="flex items-start justify-between gap-2">
      <div class="space-y-0.5 min-w-0">
        <h4 class="text-xs font-semibold text-slate-200 truncate">
          {{ kr.title }}
        </h4>
        <div class="flex items-center gap-2 text-[11px] text-slate-400 font-mono">
          <span>{{ kr.current_value }} / {{ kr.target_value }} {{ kr.unit }}</span>
          <span v-if="kr.weight" class="text-slate-500">• Peso: {{ kr.weight }}</span>
        </div>
      </div>

      <div class="flex items-center gap-2 shrink-0">
        <!-- Linked Habit Badge or Link Action -->
        <span
          v-if="linkedHabitName"
          class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
          :title="`Vinculado ao hábito: ${linkedHabitName}`"
        >
          <Link2 class="w-3 h-3" />
          {{ linkedHabitName }}
        </span>
        <button
          v-else
          type="button"
          @click="emit('linkHabit', kr.id)"
          class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-surface-border/40 hover:bg-brand-600/20 text-slate-400 hover:text-brand-300 transition-colors"
          title="Vincular a um hábito diário"
        >
          <Link2 class="w-3 h-3" />
          Vincular Hábito
        </button>

        <span class="text-xs font-bold text-white font-mono w-10 text-right">
          {{ progressClamped }}%
        </span>
      </div>
    </div>

    <!-- Progress Track -->
    <div class="h-2 w-full rounded-full bg-surface-border/40 overflow-hidden">
      <div
        class="h-full rounded-full transition-all duration-500 ease-out"
        :class="barColor"
        :style="{ width: `${progressClamped}%` }"
      />
    </div>
  </div>
</template>
