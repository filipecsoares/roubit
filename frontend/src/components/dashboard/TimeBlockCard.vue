<script setup lang="ts">
import { computed } from 'vue'
import { Check, Clock, SkipForward, Sparkles } from 'lucide-vue-next'
import type { TimeBlock, BlockStatusType } from '@/types'

const props = withDefaults(
  defineProps<{
    block: TimeBlock
    isActive?: boolean
    isPast?: boolean
    isCompleted?: boolean
    isSkipped?: boolean
    loading?: boolean
  }>(),
  {
    isActive: false,
    isPast: false,
    isCompleted: false,
    isSkipped: false,
    loading: false,
  }
)

const emit = defineEmits<{
  (e: 'complete', blockId: string, status: BlockStatusType): void
}>()

const formattedTime = computed(() => {
  const start = props.block.start_time.slice(0, 5)
  const end = props.block.end_time.slice(0, 5)
  return `${start} - ${end}`
})

const cardClasses = computed(() => {
  if (props.isActive) {
    return 'border-brand-500/80 bg-brand-950/20 shadow-lg shadow-brand-500/10 ring-1 ring-brand-500/30'
  }
  if (props.isCompleted) {
    return 'border-emerald-500/30 bg-emerald-950/10 opacity-80'
  }
  if (props.isSkipped) {
    return 'border-slate-800 bg-surface-card/40 opacity-50'
  }
  return 'border-surface-border bg-surface-card hover:border-slate-700'
})
</script>

<template>
  <div
    class="relative p-4 rounded-xl border transition-all duration-200 group"
    :class="cardClasses"
  >
    <!-- Glowing pulse bar for active block -->
    <div
      v-if="isActive"
      class="absolute -left-1 top-3 bottom-3 w-1.5 rounded-full bg-brand-500 shadow-[0_0_12px_#6366f1] animate-pulse"
    ></div>

    <div class="flex items-start justify-between gap-3">
      <div class="space-y-1 min-w-0">
        <div class="flex items-center gap-2">
          <!-- Time range badge -->
          <span
            class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-mono font-medium"
            :class="isActive ? 'bg-brand-500/20 text-brand-300' : 'bg-surface-border/60 text-slate-400'"
          >
            <Clock class="w-3 h-3" />
            {{ formattedTime }}
          </span>

          <!-- Active now badge -->
          <span
            v-if="isActive"
            class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-semibold bg-brand-500 text-white animate-bounce-short"
          >
            <Sparkles class="w-3 h-3" />
            Agora
          </span>

          <!-- Category tag -->
          <span
            v-if="block.category"
            class="px-2 py-0.5 rounded text-xs font-medium bg-surface-border/40 text-slate-400"
          >
            {{ block.category }}
          </span>
        </div>

        <h3
          class="text-base font-semibold truncate transition-colors"
          :class="{
            'text-white': isActive,
            'text-emerald-300 line-through': isCompleted,
            'text-slate-400 line-through': isSkipped,
            'text-slate-200': !isActive && !isCompleted && !isSkipped,
          }"
        >
          {{ block.name }}
        </h3>
      </div>

      <!-- Actions -->
      <div class="flex items-center gap-1.5 shrink-0">
        <button
          v-if="!isCompleted"
          type="button"
          :disabled="loading"
          @click="emit('complete', block.id, 'completed')"
          title="Marcar como concluído"
          class="p-2 rounded-lg bg-surface-border/40 hover:bg-emerald-500/20 hover:text-emerald-400 text-slate-400 transition-colors"
        >
          <Check class="w-4 h-4" />
        </button>

        <button
          v-if="!isCompleted && !isSkipped"
          type="button"
          :disabled="loading"
          @click="emit('complete', block.id, 'skipped')"
          title="Pular bloco"
          class="p-2 rounded-lg bg-surface-border/40 hover:bg-slate-700 text-slate-400 transition-colors"
        >
          <SkipForward class="w-4 h-4" />
        </button>

        <span
          v-if="isCompleted"
          class="inline-flex items-center gap-1 text-xs font-medium text-emerald-400 px-2 py-1 rounded bg-emerald-500/10 border border-emerald-500/20"
        >
          <Check class="w-3.5 h-3.5" /> Concluído
        </span>
      </div>
    </div>
  </div>
</template>
