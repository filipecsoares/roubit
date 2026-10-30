<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import {
  Play,
  Pause,
  RotateCcw,
  SkipForward,
  CheckCircle2,
  X,
  Sparkles,
  Coffee,
  Clock,
  Flame,
} from 'lucide-vue-next'
import { useFocusStore } from '@/stores/focus'

const props = withDefaults(
  defineProps<{
    size?: number
    strokeWidth?: number
  }>(),
  {
    size: 240,
    strokeWidth: 10,
  }
)

const emit = defineEmits<{
  (e: 'completeBlock', blockId: string): void
}>()

const focusStore = useFocusStore()

const radius = computed(() => (props.size - props.strokeWidth) / 2)
const circumference = computed(() => 2 * Math.PI * radius.value)
const strokeDashoffset = computed(() => {
  const percent = focusStore.progressPercentage
  return circumference.value - (percent / 100) * circumference.value
})

const modeTitle = computed(() => {
  if (focusStore.mode === 'short_break') return 'Pausa Curta'
  if (focusStore.mode === 'long_break') return 'Pausa Longa'
  return 'Modo Foco'
})

const modeSubtitle = computed(() => {
  if (focusStore.isBreak) return 'Relaxe a mente, hidrate-se e descanse a visão'
  if (focusStore.associatedBlockName) return `Focando em: ${focusStore.associatedBlockName}`
  return 'Elimine distrações e concentre-se na tarefa atual'
})

const progressColor = computed(() => {
  if (focusStore.isBreak) return '#10b981' // emerald-500
  if (focusStore.state === 'completed') return '#10b981'
  return '#6366f1' // brand-500
})

function handleCompleteBlock() {
  if (focusStore.associatedBlockId) {
    emit('completeBlock', focusStore.associatedBlockId)
  }
}

function handleKeydown(event: KeyboardEvent) {
  if (!focusStore.isModalOpen) return

  // Prevent capturing space when typing in inputs/textareas
  const activeElement = document.activeElement?.tagName.toLowerCase()
  if (activeElement === 'input' || activeElement === 'textarea') return

  if (event.code === 'Space') {
    event.preventDefault()
    focusStore.togglePlayPause()
  } else if (event.code === 'Escape') {
    event.preventDefault()
    focusStore.closeFocusModal()
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="focusStore.isModalOpen"
      class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-surface-dark/95 backdrop-blur-2xl animate-fade-in"
      @click="focusStore.closeFocusModal"
    >
      <div
        class="relative w-full max-w-lg rounded-3xl bg-surface-card/90 border border-surface-border/80 shadow-2xl p-6 sm:p-8 flex flex-col items-center text-center space-y-6 max-h-[95vh] overflow-y-auto glow-card"
        @click.stop
      >
        <!-- Top bar: Close/minimize button -->
        <div class="w-full flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span
              class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold"
              :class="
                focusStore.isBreak
                  ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20'
                  : 'bg-brand-500/10 text-brand-300 border border-brand-500/20'
              "
            >
              <Coffee v-if="focusStore.isBreak" class="w-3.5 h-3.5" />
              <Sparkles v-else class="w-3.5 h-3.5" />
              <span>{{ modeTitle }}</span>
            </span>

            <span
              v-if="focusStore.completedCycles > 0"
              class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-mono font-medium bg-surface-dark text-slate-300 border border-surface-border"
              title="Ciclos de Pomodoro concluídos hoje"
            >
              <Flame class="w-3 h-3 text-amber-400 fill-amber-400/20" />
              <span>{{ focusStore.completedCycles }} ciclos</span>
            </span>
          </div>

          <button
            type="button"
            @click="focusStore.closeFocusModal"
            class="p-2 rounded-xl text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors touch-target sm:min-h-0 sm:min-w-0"
            title="Minimizar (mantém timer ativo no topo)"
          >
            <X class="w-5 h-5" />
          </button>
        </div>

        <!-- Block Context info -->
        <div class="space-y-1">
          <h2 class="text-xl sm:text-2xl font-extrabold text-white font-outfit tracking-tight">
            {{ focusStore.associatedBlockName || 'Sessão de Foco' }}
          </h2>
          <p class="text-xs sm:text-sm text-slate-400 max-w-sm">
            {{ modeSubtitle }}
          </p>
        </div>

        <!-- Circular Countdown Gauge -->
        <div class="relative flex items-center justify-center my-2" :style="{ width: `${size}px`, height: `${size}px` }">
          <svg class="transform -rotate-90" :width="size" :height="size">
            <!-- Background circle track -->
            <circle
              class="text-surface-border/40"
              :stroke-width="strokeWidth"
              stroke="currentColor"
              fill="transparent"
              :r="radius"
              :cx="size / 2"
              :cy="size / 2"
            />
            <!-- Progress circle with gradient glow -->
            <circle
              class="transition-all duration-500 ease-out"
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
                filter: focusStore.isBreak
                  ? 'drop-shadow(0 0 8px rgba(16, 185, 129, 0.4))'
                  : 'drop-shadow(0 0 10px rgba(99, 102, 241, 0.4))',
              }"
            />
          </svg>

          <!-- Center Content -->
          <div class="absolute inset-0 flex flex-col items-center justify-center">
            <span class="text-4xl sm:text-5xl font-extrabold text-white font-mono tracking-tighter">
              {{ focusStore.formattedTime }}
            </span>
            <span
              class="text-[11px] font-semibold uppercase tracking-wider mt-1 px-2 py-0.5 rounded-full"
              :class="
                focusStore.state === 'running' || focusStore.state === 'break'
                  ? 'text-brand-300 bg-brand-500/10'
                  : focusStore.state === 'paused'
                  ? 'text-amber-400 bg-amber-500/10'
                  : focusStore.state === 'completed'
                  ? 'text-emerald-400 bg-emerald-500/10'
                  : 'text-slate-400 bg-surface-border/40'
              "
            >
              {{
                focusStore.state === 'running'
                  ? 'Em foco'
                  : focusStore.state === 'break'
                  ? 'Em pausa'
                  : focusStore.state === 'paused'
                  ? 'Pausado'
                  : focusStore.state === 'completed'
                  ? 'Concluído!'
                  : 'Pronto'
              }}
            </span>
          </div>
        </div>

        <!-- Controls Toolbar -->
        <div class="flex items-center justify-center gap-3 w-full">
          <!-- Reset button -->
          <button
            type="button"
            @click="focusStore.resetTimer"
            title="Reiniciar sessão"
            class="p-3 rounded-2xl bg-surface-dark border border-surface-border text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors touch-target"
          >
            <RotateCcw class="w-5 h-5" />
          </button>

          <!-- Main Play / Pause button -->
          <button
            type="button"
            @click="focusStore.togglePlayPause"
            :title="focusStore.isRunning ? 'Pausar (Espaço)' : 'Iniciar (Espaço)'"
            class="px-8 py-3.5 rounded-2xl text-base font-semibold text-white shadow-xl flex items-center justify-center gap-2 transition-all transform active:scale-95 touch-target min-w-[140px]"
            :class="
              focusStore.isBreak
                ? 'bg-emerald-600 hover:bg-emerald-500 shadow-emerald-500/25'
                : 'bg-brand-600 hover:bg-brand-500 shadow-brand-500/25'
            "
          >
            <Pause v-if="focusStore.isRunning" class="w-5 h-5" />
            <Play v-else class="w-5 h-5 fill-white" />
            <span>{{ focusStore.isRunning ? 'Pausar' : 'Iniciar' }}</span>
          </button>

          <!-- Skip to Break or Focus button -->
          <button
            type="button"
            @click="focusStore.isBreak ? focusStore.skipToFocus() : focusStore.skipToBreak()"
            :title="focusStore.isBreak ? 'Pular para Foco' : 'Pular para Pausa'"
            class="p-3 rounded-2xl bg-surface-dark border border-surface-border text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors touch-target"
          >
            <SkipForward class="w-5 h-5" />
          </button>
        </div>

        <!-- Bottom Actions (Complete routine block if tied to one) -->
        <div
          v-if="focusStore.associatedBlockId"
          class="w-full pt-4 border-t border-surface-border/60 flex items-center justify-between gap-3 text-xs"
        >
          <div class="flex items-center gap-1.5 text-slate-400 text-left">
            <Clock class="w-3.5 h-3.5 text-brand-400 shrink-0" />
            <span class="truncate">Bloco de rotina ativo vinculado</span>
          </div>

          <button
            type="button"
            @click="handleCompleteBlock"
            class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-semibold bg-emerald-500/15 hover:bg-emerald-500/25 text-emerald-300 border border-emerald-500/30 transition-colors touch-target sm:min-h-0 sm:min-w-0"
            title="Marcar bloco da rotina como concluído"
          >
            <CheckCircle2 class="w-3.5 h-3.5" />
            <span>Concluir Bloco</span>
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
