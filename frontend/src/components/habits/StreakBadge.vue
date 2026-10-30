<script setup lang="ts">
import { computed } from 'vue'
import { Flame, Trophy } from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    currentStreak: number
    longestStreak?: number
    isActiveToday?: boolean
    size?: 'sm' | 'md' | 'lg'
  }>(),
  {
    longestStreak: 0,
    isActiveToday: false,
    size: 'md',
  }
)

const badgeClasses = computed(() => {
  const activeColor = props.currentStreak > 0
    ? 'bg-amber-500/10 text-amber-400 border-amber-500/30 shadow-sm shadow-amber-500/10'
    : 'bg-surface-border/40 text-slate-500 border-surface-border'

  if (props.size === 'sm') return `px-2 py-0.5 text-xs ${activeColor}`
  if (props.size === 'lg') return `px-3.5 py-1.5 text-sm ${activeColor}`
  return `px-2.5 py-1 text-xs ${activeColor}`
})

const isNewRecord = computed(() => {
  return props.currentStreak > 0 && props.currentStreak >= props.longestStreak
})
</script>

<template>
  <div class="inline-flex items-center gap-1.5 rounded-full border font-medium" :class="badgeClasses">
    <Flame
      class="transition-all"
      :class="[
        size === 'lg' ? 'w-4 h-4' : 'w-3.5 h-3.5',
        currentStreak > 0 ? 'text-amber-400 fill-amber-400/30 animate-pulse' : 'text-slate-500',
      ]"
    />
    <span class="font-bold">{{ currentStreak }}</span>
    <span class="text-slate-400 text-[11px]">{{ currentStreak === 1 ? 'dia' : 'dias' }}</span>

    <span
      v-if="isNewRecord"
      class="inline-flex items-center gap-0.5 ml-1 px-1.5 py-0.2 rounded-full text-[10px] font-semibold bg-amber-400/20 text-amber-300"
      title="Novo recorde pessoal!"
    >
      <Trophy class="w-2.5 h-2.5 text-amber-300" />
      Recorde
    </span>
    <span
      v-else-if="longestStreak > 0"
      class="text-[10px] text-slate-500 ml-0.5"
      title="Melhor sequência histórica"
    >
      (max: {{ longestStreak }}d)
    </span>
  </div>
</template>
