<script setup lang="ts">
import { computed } from 'vue'
import { Calendar } from 'lucide-vue-next'
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
}>()

const sortedBlocks = computed(() => {
  return [...props.blocks].sort((a, b) => a.start_time.localeCompare(b.start_time))
})
</script>

<template>
  <div class="space-y-4">
    <!-- Empty state -->
    <div
      v-if="blocks.length === 0"
      class="text-center py-12 px-4 rounded-xl border border-dashed border-surface-border"
    >
      <div class="inline-flex p-3 rounded-2xl bg-surface-border/40 text-slate-400 mb-3">
        <Calendar class="w-6 h-6" />
      </div>
      <h3 class="text-sm font-medium text-white">Nenhum bloco de rotina ativo</h3>
      <p class="text-xs text-slate-400 mt-1 max-w-xs mx-auto">
        Estruture seus blocos de foco e hábitos ao longo do dia para manter sua produtividade.
      </p>
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
        />
      </div>
    </div>
  </div>
</template>
