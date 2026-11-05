<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    percentage?: number
    size?: number
    strokeWidth?: number
    routinePercentage?: number
    habitPercentage?: number
  }>(),
  {
    percentage: 0,
    size: 140,
    strokeWidth: 10,
    routinePercentage: 0,
    habitPercentage: 0,
  }
)

const radius = computed(() => (props.size - props.strokeWidth) / 2)
const circumference = computed(() => 2 * Math.PI * radius.value)
const strokeDashoffset = computed(() => {
  const clamped = Math.min(100, Math.max(0, props.percentage))
  return circumference.value - (clamped / 100) * circumference.value
})

const progressColor = computed(() => {
  if (props.percentage >= 100) return '#10b981' // emerald-500
  if (props.percentage >= 60) return '#e4e4e7'  // zinc-200
  if (props.percentage >= 30) return '#a1a1aa'  // zinc-400
  return '#71717a' // zinc-500
})
</script>

<template>
  <div class="w-full sm:w-auto flex items-center justify-between sm:justify-start gap-4 sm:gap-6 p-4 rounded-2xl bg-surface-card border border-surface-border shadow-md glow-card">
    <div class="relative flex items-center justify-center shrink-0" :style="{ width: `${size}px`, height: `${size}px` }">
      <svg class="transform -rotate-90" :width="size" :height="size">
        <!-- Background track -->
        <circle
          class="text-zinc-800"
          :stroke-width="strokeWidth"
          stroke="currentColor"
          fill="transparent"
          :r="radius"
          :cx="size / 2"
          :cy="size / 2"
        />
        <!-- Progress track -->
        <circle
          class="transition-all duration-700 ease-out"
          :stroke="progressColor"
          :stroke-width="strokeWidth"
          stroke-linecap="round"
          fill="transparent"
          :r="radius"
          :cx="size / 2"
          :cy="size / 2"
          :style="{
            strokeDasharray: circumference,
            strokeDashoffset: strokeDashoffset,
          }"
        />
      </svg>
      <!-- Center content -->
      <div class="absolute inset-0 flex flex-col items-center justify-center text-center">
        <span class="text-2xl sm:text-3xl font-extrabold tracking-tight text-white font-mono">
          {{ Math.round(percentage) }}%
        </span>
        <span class="text-[11px] font-medium text-slate-400">Score</span>
      </div>
    </div>

    <div class="flex flex-col justify-center space-y-2 text-xs sm:text-sm min-w-0 pr-2">
      <div class="text-[10px] sm:text-xs uppercase tracking-wider font-semibold text-slate-400">Progresso Geral</div>
      <div class="flex items-center gap-2">
        <span class="w-2.5 h-2.5 rounded-full bg-brand-500 shrink-0"></span>
        <span class="text-slate-300">Rotinas:</span>
        <span class="font-bold text-white font-mono">{{ Math.round(routinePercentage) }}%</span>
      </div>
      <div class="flex items-center gap-2">
        <span class="w-2.5 h-2.5 rounded-full bg-emerald-500 shrink-0"></span>
        <span class="text-slate-300">Hábitos:</span>
        <span class="font-bold text-white font-mono">{{ Math.round(habitPercentage) }}%</span>
      </div>
    </div>
  </div>
</template>
