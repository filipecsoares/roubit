<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    percentage: number
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
  if (props.percentage >= 60) return '#6366f1'  // brand-500
  if (props.percentage >= 30) return '#f59e0b'  // amber-500
  return '#64748b' // slate-500
})
</script>

<template>
  <div class="flex items-center gap-6 p-4 rounded-2xl bg-surface-card border border-surface-border/60 shadow-lg shadow-black/20">
    <div class="relative flex items-center justify-center" :style="{ width: `${size}px`, height: `${size}px` }">
      <svg class="transform -rotate-90" :width="size" :height="size">
        <!-- Background track -->
        <circle
          class="text-surface-border/50"
          :stroke-width="strokeWidth"
          stroke="currentColor"
          fill="transparent"
          :r="radius"
          :cx="size / 2"
          :cy="size / 2"
        />
        <!-- Progress track with gradient glow -->
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
            filter: 'drop-shadow(0 0 6px rgba(99, 102, 241, 0.4))'
          }"
        />
      </svg>
      <!-- Center content -->
      <div class="absolute inset-0 flex flex-col items-center justify-center text-center">
        <span class="text-3xl font-extrabold tracking-tight text-white">
          {{ Math.round(percentage) }}%
        </span>
        <span class="text-xs font-medium text-slate-400">Score</span>
      </div>
    </div>

    <div class="flex flex-col justify-center space-y-2 text-sm">
      <div class="text-xs uppercase tracking-wider font-semibold text-slate-400">Progresso Geral</div>
      <div class="flex items-center gap-2">
        <span class="w-2.5 h-2.5 rounded-full bg-brand-500"></span>
        <span class="text-slate-300">Rotinas:</span>
        <span class="font-semibold text-white">{{ Math.round(routinePercentage) }}%</span>
      </div>
      <div class="flex items-center gap-2">
        <span class="w-2.5 h-2.5 rounded-full bg-emerald-500"></span>
        <span class="text-slate-300">Hábitos:</span>
        <span class="font-semibold text-white">{{ Math.round(habitPercentage) }}%</span>
      </div>
    </div>
  </div>
</template>
