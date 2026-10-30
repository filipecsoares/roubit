<script setup lang="ts">
import { computed } from 'vue'
import { Check, Flame, Plus } from 'lucide-vue-next'
import type { Habit, StreakMetrics } from '@/types'

const props = withDefaults(
  defineProps<{
    habit: Habit
    streak: StreakMetrics
    isCompletedToday?: boolean
    currentValue?: number
    loading?: boolean
  }>(),
  {
    isCompletedToday: false,
    currentValue: 0,
    loading: false,
  }
)

const emit = defineEmits<{
  (e: 'checkin', habitId: string, value: number, isIncrement: boolean): void
}>()

const targetDisplay = computed(() => {
  if (props.habit.habit_type === 'binary') return null
  const unit = props.habit.unit ? ` ${props.habit.unit}` : ''
  return `${props.currentValue} / ${props.habit.target_value}${unit}`
})

const numericIncrementStep = computed(() => {
  if (props.habit.habit_type === 'numeric') {
    if (props.habit.target_value >= 1000) return 250
    if (props.habit.target_value >= 100) return 25
    if (props.habit.target_value >= 10) return 5
    return 1
  }
  if (props.habit.habit_type === 'timer') {
    return 15 // 15 minutes increment
  }
  return 1
})

function handleBinaryToggle() {
  const nextValue = props.isCompletedToday ? 0 : 1
  emit('checkin', props.habit.id, nextValue, false)
}

function handleNumericIncrement(step: number) {
  emit('checkin', props.habit.id, step, true)
}
</script>

<template>
  <div
    class="flex items-center justify-between p-3.5 rounded-xl border transition-all duration-200 group"
    :class="
      isCompletedToday
        ? 'border-emerald-500/30 bg-emerald-950/10'
        : 'border-surface-border bg-surface-card hover:border-slate-700'
    "
  >
    <!-- Left: Habit Info & Streaks -->
    <div class="space-y-1 min-w-0 pr-3">
      <div class="flex items-center gap-2">
        <h4
          class="text-sm font-semibold truncate transition-colors"
          :class="isCompletedToday ? 'text-emerald-300 line-through' : 'text-slate-100 group-hover:text-white'"
        >
          {{ habit.name }}
        </h4>

        <!-- Streak Badge -->
        <span
          class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium"
          :class="
            streak.current_streak > 0
              ? 'bg-amber-500/10 text-amber-400 border border-amber-500/20'
              : 'bg-surface-border/40 text-slate-500'
          "
        >
          <Flame
            class="w-3 h-3"
            :class="streak.current_streak > 0 ? 'fill-amber-400 text-amber-400 animate-pulse' : 'text-slate-500'"
          />
          {{ streak.current_streak }} {{ streak.current_streak === 1 ? 'dia' : 'dias' }}
        </span>
      </div>

      <!-- Habit Progress Subtitle -->
      <p v-if="targetDisplay" class="text-xs text-slate-400 font-mono">
        {{ targetDisplay }}
      </p>
      <p v-else-if="habit.description" class="text-xs text-slate-400 truncate">
        {{ habit.description }}
      </p>
    </div>

    <!-- Right: Quick Checkin Actions -->
    <div class="flex items-center gap-2 shrink-0">
      <!-- Numeric/Timer Quick Increment Button -->
      <button
        v-if="habit.habit_type !== 'binary' && !isCompletedToday"
        type="button"
        :disabled="loading"
        @click="handleNumericIncrement(numericIncrementStep)"
        class="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-medium bg-surface-border/60 hover:bg-brand-600/30 text-slate-300 hover:text-white transition-colors"
        :title="`Adicionar +${numericIncrementStep}`"
      >
        <Plus class="w-3 h-3" />
        +{{ numericIncrementStep }}{{ habit.unit ? ` ${habit.unit}` : '' }}
      </button>

      <!-- Main Toggle Checkmark Button -->
      <button
        type="button"
        :disabled="loading"
        @click="handleBinaryToggle"
        :title="isCompletedToday ? 'Marcar como não concluído' : 'Concluir hábito'"
        class="relative w-9 h-9 rounded-xl flex items-center justify-center transition-all duration-300 active:scale-95"
        :class="
          isCompletedToday
            ? 'bg-emerald-500 text-white shadow-md shadow-emerald-500/25 ring-2 ring-emerald-400/40'
            : 'bg-surface-border/60 hover:bg-slate-700 text-slate-400 hover:text-white border border-surface-border'
        "
      >
        <Check
          class="w-5 h-5 transition-transform duration-300"
          :class="isCompletedToday ? 'scale-100 rotate-0' : 'scale-75 text-transparent'"
        />
      </button>
    </div>
  </div>
</template>
