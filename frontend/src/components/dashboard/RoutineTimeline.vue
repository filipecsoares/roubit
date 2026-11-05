<script setup lang="ts">
import { computed } from 'vue'
import { Calendar, Plus, Timer } from 'lucide-vue-next'
import TimeBlockCard from './TimeBlockCard.vue'
import type { TimeBlock, BlockStatusType } from '@/types'

const props = withDefaults(
  defineProps<{
    blocks: TimeBlock[]
    activeBlockId?: string | null
    completedBlockIds?: string[]
    skippedBlockIds?: string[]
    loading?: boolean
  }>(),
  {
    activeBlockId: null,
    completedBlockIds: () => [],
    skippedBlockIds: () => [],
    loading: false,
  }
)

const emit = defineEmits<{
  (e: 'complete', blockId: string, status: BlockStatusType): void
  (e: 'openCreate'): void
  (e: 'openFocus', block?: TimeBlock): void
}>()

const sortedBlocks = computed(() => {
  return [...props.blocks].sort((a, b) => a.start_time.localeCompare(b.start_time))
})
</script>

<template>
  <div class="space-y-4">
    <!-- Loading skeleton -->
    <div
      v-if="loading && blocks.length === 0"
      class="text-center py-8 sm:py-12 px-4 rounded-xl border border-dashed border-surface-border/40 space-y-3 animate-pulse"
    >
      <div class="inline-flex p-3 rounded-2xl bg-surface-border/30 text-slate-500">
        <Calendar class="w-6 h-6 animate-spin" />
      </div>
      <p class="text-xs text-slate-500">Carregando cronograma de rotinas...</p>
    </div>

    <!-- Empty state -->
    <div
      v-else-if="blocks.length === 0"
      class="text-center py-8 sm:py-12 px-4 rounded-xl border border-dashed border-surface-border space-y-3"
    >
      <div class="inline-flex p-3 rounded-2xl bg-surface-border/40 text-slate-400">
        <Calendar class="w-6 h-6" />
      </div>
      <div class="max-w-xs mx-auto">
        <h3 class="text-sm font-medium text-white">Nenhum bloco de rotina ativo</h3>
        <p class="text-xs text-slate-400 mt-1">
          Estruture seus blocos de rotina diária ou use o Modo Foco a qualquer momento.
        </p>
      </div>
      <div class="flex items-center justify-center gap-2 flex-wrap pt-1">
        <button
          type="button"
          @click="emit('openCreate')"
          class="inline-flex items-center gap-1.5 px-4 py-2 rounded-xl text-xs font-semibold bg-brand-600 hover:bg-brand-500 text-white shadow-md shadow-brand-500/20 transition-all touch-target sm:min-h-0 sm:min-w-0"
        >
          <Plus class="w-3.5 h-3.5" />
          Configurar Rotina
        </button>
        <button
          type="button"
          @click="emit('openFocus')"
          class="inline-flex items-center gap-1.5 px-4 py-2 rounded-xl text-xs font-semibold bg-surface-card hover:bg-surface-border/60 text-brand-300 border border-brand-500/30 transition-all touch-target sm:min-h-0 sm:min-w-0"
        >
          <Timer class="w-3.5 h-3.5 text-brand-400" />
          Iniciar Foco Livre
        </button>
      </div>
    </div>

    <!-- Vertical Timeline -->
    <div v-else class="relative pl-6 space-y-4 before:absolute before:left-2.5 before:top-3 before:bottom-3 before:w-0.5 before:bg-surface-border">
      <div
        v-for="block in sortedBlocks"
        :key="block.id"
        class="relative group"
      >
        <!-- Timeline Node Marker -->
        <div
          class="absolute -left-6 top-5 w-3.5 h-3.5 rounded-full border-2 transition-all"
          :class="{
            'bg-brand-500 border-white shadow-[0_0_8px_#6366f1] scale-125': block.id === activeBlockId,
            'bg-emerald-500 border-emerald-400': completedBlockIds.includes(block.id),
            'bg-slate-700 border-slate-600': skippedBlockIds.includes(block.id),
            'bg-surface-card border-surface-border group-hover:border-slate-500':
              block.id !== activeBlockId && !completedBlockIds.includes(block.id) && !skippedBlockIds.includes(block.id),
          }"
        ></div>

        <!-- TimeBlock Card Component -->
        <TimeBlockCard
          :block="block"
          :is-active="block.id === activeBlockId"
          :is-completed="completedBlockIds.includes(block.id)"
          :is-skipped="skippedBlockIds.includes(block.id)"
          :loading="loading"
          @complete="(id, status) => emit('complete', id, status)"
          @open-focus="(b) => emit('openFocus', b)"
        />
      </div>
    </div>
  </div>
</template>
