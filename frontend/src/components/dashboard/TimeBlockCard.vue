<script setup lang="ts">
import { computed } from 'vue'
import { Check, Clock, SkipForward, Sparkles, Timer } from 'lucide-vue-next'
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
  (e: 'openFocus', block: TimeBlock): void
}>()

const formattedTime = computed(() => {
  const start = props.block.start_time.slice(0, 5)
  const end = props.block.end_time.slice(0, 5)
  return `${start} - ${end}`
})

const cardClasses = computed(() => {
  if (props.isActive) {
    return 'border-zinc-500 bg-zinc-800/60 shadow-md ring-1 ring-zinc-600/50'
  }
  if (props.isCompleted) {
    return 'border-emerald-500/30 bg-emerald-950/10 opacity-80'
  }
  if (props.isSkipped) {
    return 'border-zinc-800 bg-surface-card/40 opacity-50'
  }
  return 'border-surface-border bg-surface-card hover:border-zinc-700'
})
</script>

<template>
  <div
    class="relative p-3.5 sm:p-4 rounded-xl border transition-all duration-200 group"
    :class="cardClasses"
  >
    <!-- Sleek accent bar for active block -->
    <div
      v-if="isActive"
      class="absolute -left-1 top-3 bottom-3 w-1 rounded-full bg-zinc-200"
    ></div>

    <div class="flex items-start justify-between gap-3">
      <div class="space-y-1 min-w-0">
        <div class="flex items-center gap-2 flex-wrap">
          <!-- Time range badge -->
          <span
            class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-mono font-medium"
            :class="isActive ? 'bg-zinc-700 text-zinc-100' : 'bg-surface-border/60 text-zinc-400'"
          >
            <Clock class="w-3 h-3" />
            {{ formattedTime }}
          </span>

          <!-- Active now badge -->
          <span
            v-if="isActive"
            class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-semibold bg-zinc-200 text-zinc-950 shadow-sm"
          >
            <Sparkles class="w-3 h-3" />
            Agora
          </span>

          <!-- Category tag -->
          <span
            v-if="block.category"
            class="px-2 py-0.5 rounded text-xs font-medium bg-surface-border/40 text-zinc-400"
          >
            {{ block.category }}
          </span>
        </div>

        <h3
          class="text-sm sm:text-base font-semibold truncate transition-colors"
          :class="{
            'text-white': isActive,
            'text-emerald-300 line-through': isCompleted,
            'text-zinc-500 line-through': isSkipped,
            'text-zinc-200': !isActive && !isCompleted && !isSkipped,
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
          class="p-2 rounded-xl bg-surface-border/40 hover:bg-emerald-500/20 hover:text-emerald-400 text-zinc-400 transition-colors touch-target sm:min-h-0 sm:min-w-0"
        >
          <Check class="w-4 h-4" />
        </button>

        <button
          v-if="!isCompleted && !isSkipped"
          type="button"
          :disabled="loading"
          @click="emit('complete', block.id, 'skipped')"
          title="Pular bloco"
          class="p-2 rounded-xl bg-surface-border/40 hover:bg-zinc-700 text-zinc-400 transition-colors touch-target sm:min-h-0 sm:min-w-0"
        >
          <SkipForward class="w-4 h-4" />
        </button>

        <!-- Focus Mode Launch Button for Active Block -->
        <button
          v-if="isActive && !isCompleted"
          type="button"
          @click="emit('openFocus', block)"
          title="Iniciar Modo Foco neste bloco"
          class="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-xl text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 shadow-sm transition-all active:scale-95 touch-target sm:min-h-0 sm:min-w-0"
        >
          <Timer class="w-3.5 h-3.5" />
          <span class="hidden sm:inline">Focar</span>
        </button>

        <span
          v-if="isCompleted"
          class="inline-flex items-center gap-1 text-xs font-medium text-emerald-400 px-2 py-1 rounded-lg bg-emerald-500/10 border border-emerald-500/20"
        >
          <Check class="w-3.5 h-3.5" /> Concluído
        </span>
      </div>
    </div>
  </div>
</template>
