<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { Clock, AlertCircle, RefreshCw, Sparkles, Plus, Timer } from 'lucide-vue-next'
import { useRoutineStore } from '@/stores/routine'
import { useHabitStore } from '@/stores/habit'
import { useFocusStore } from '@/stores/focus'
import RoutineTimeline from '@/components/dashboard/RoutineTimeline.vue'
import HabitChecklist from '@/components/dashboard/HabitChecklist.vue'
import DailyProgressGauge from '@/components/dashboard/DailyProgressGauge.vue'
import RoutineManageModal from '@/components/dashboard/RoutineManageModal.vue'
import type { BlockStatusType } from '@/types'
import { formatLocalDate } from '@/utils/date'
import { useToast } from '@/composables/useToast'

const routineStore = useRoutineStore()
const habitStore = useHabitStore()
const focusStore = useFocusStore()
const { toastMessage, showToast } = useToast()

const now = ref(new Date())
let timerId: ReturnType<typeof setInterval> | null = null

const completedBlockIds = computed(() => routineStore.completedBlockIds)
const skippedBlockIds = computed(() => routineStore.skippedBlockIds)
const isRoutineModalOpen = ref(false)

// Format today's date in Portuguese (e.g., Sexta-feira, 30 de Outubro de 2026)
const currentDateFormatted = computed(() => {
  return now.value.toLocaleDateString('pt-BR', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  })
})

const currentTimeStr = computed(() => {
  const h = String(now.value.getHours()).padStart(2, '0')
  const m = String(now.value.getMinutes()).padStart(2, '0')
  const s = String(now.value.getSeconds()).padStart(2, '0')
  return `${h}:${m}:${s}`
})

// Determine currently active block based on current time
const activeBlock = computed(() => {
  if (routineStore.progress?.active_block) {
    return routineStore.progress.active_block
  }
  const current = currentTimeStr.value
  return (
    routineStore.blocks.find((b) => {
      const start = b.start_time.length === 5 ? `${b.start_time}:00` : b.start_time
      const end = b.end_time.length === 5 ? `${b.end_time}:00` : b.end_time
      return current >= start && current < end
    }) || null
  )
})

// Adherence & Progress calculations
const routinePercentage = computed(() => {
  if (routineStore.progress?.adherence_percentage !== undefined && routineStore.progress !== null) {
    return Math.round(routineStore.progress.adherence_percentage)
  }
  const total = routineStore.blocks.length
  if (total === 0) return 0
  const completed = completedBlockIds.value.length
  return Math.min(100, Math.round((completed / total) * 100))
})

const habitPercentage = computed(() => {
  const total = habitStore.habits.length
  if (total === 0) return 0
  const completed = habitStore.habits.filter((h) => h.streak.is_active_today).length
  return Math.min(100, Math.round((completed / total) * 100))
})

const combinedProgressPercentage = computed(() => {
  const hasRoutine = routineStore.blocks.length > 0
  const hasHabits = habitStore.habits.length > 0
  if (!hasRoutine && !hasHabits) return 0
  if (hasRoutine && !hasHabits) return routinePercentage.value
  if (!hasRoutine && hasHabits) return habitPercentage.value
  return Math.round((routinePercentage.value + habitPercentage.value) / 2)
})

async function handleCompleteBlock(blockId: string, status: BlockStatusType) {
  try {
    await routineStore.completeBlock(blockId, status)
    showToast(`Bloco atualizado como ${status === 'completed' ? 'concluído' : 'pulado'}`)
  } catch (err: any) {
    showToast(err, 'error')
  }
}

async function handleCreateRoutine(payload: {
  name: string
  description?: string
  is_active: boolean
  blocks: Array<{
    name: string
    start_time: string
    end_time: string
    category?: string
    color_hex?: string
    order_index: number
  }>
}) {
  try {
    await routineStore.createRoutine(payload)
    isRoutineModalOpen.value = false
    showToast('Rotina e cronograma de blocos salvos com sucesso!')
  } catch (err: any) {
    showToast(err, 'error')
  }
}

async function handleHabitCheckin(habitId: string, value: number, isIncrement: boolean) {
  const habitItem = habitStore.habits.find((h) => h.habit.id === habitId)
  if (!habitItem) return

  // Save previous state for optimistic rollback
  const prevActive = habitItem.streak.is_active_today
  const prevStreak = habitItem.streak.current_streak
  const prevValue = habitItem.today_value ?? 0

  // Optimistic update
  const nextValue = isIncrement ? prevValue + value : value
  const willBeActive = nextValue >= habitItem.habit.target_value
  habitItem.today_value = nextValue
  habitItem.streak.is_active_today = willBeActive
  if (willBeActive && !prevActive) {
    habitItem.streak.current_streak += 1
  } else if (!willBeActive && prevActive) {
    habitItem.streak.current_streak = Math.max(0, habitItem.streak.current_streak - 1)
  }

  const todayStr = formatLocalDate()

  try {
    await habitStore.checkin(habitId, todayStr, value, isIncrement)
    if (willBeActive) {
      showToast('Hábito concluído!')
    } else if (isIncrement) {
      showToast('Progresso registrado!')
    } else {
      showToast('Check-in desfeito')
    }
  } catch (err: any) {
    // Rollback on failure
    habitItem.today_value = prevValue
    habitItem.streak.is_active_today = prevActive
    habitItem.streak.current_streak = prevStreak
    showToast(err, 'error')
  }
}

async function loadData() {
  try {
    await Promise.allSettled([
      routineStore.fetchActiveRoutine(),
      habitStore.fetchHabits(),
    ])
  } catch (e) {
    // Errors handled in stores
  }
}

onMounted(() => {
  loadData()
  timerId = setInterval(() => {
    now.value = new Date()
  }, 30000)
})

onUnmounted(() => {
  if (timerId) clearInterval(timerId)
})
</script>

<template>
  <div class="space-y-6 sm:space-y-8 animate-fade-in">
    <!-- Toast Notification -->
    <div
      v-if="toastMessage"
      role="status"
      aria-live="polite"
      class="fixed bottom-20 md:bottom-6 right-4 sm:right-6 z-50 flex items-center gap-2 px-4 py-3 rounded-xl border shadow-xl text-sm transition-all"
      :class="
        toastMessage.type === 'success'
          ? 'bg-emerald-950/95 border-emerald-500/50 text-emerald-200'
          : 'bg-rose-950/95 border-rose-500/50 text-rose-200'
      "
    >
      <AlertCircle v-if="toastMessage.type === 'error'" class="w-4 h-4 shrink-0" />
      <Sparkles v-else class="w-4 h-4 shrink-0 text-emerald-400" />
      <span>{{ toastMessage.text }}</span>
    </div>

    <!-- Header & Gauge Banner -->
    <header class="flex flex-col lg:flex-row justify-between items-start lg:items-center gap-5">
      <div>
        <div class="flex items-center gap-2.5">
          <h1 class="text-2xl sm:text-3xl font-extrabold tracking-tight text-white font-outfit">
            Daily Focus
          </h1>
          <button
            type="button"
            @click="loadData"
            :disabled="routineStore.loading || habitStore.loading"
            class="p-2 rounded-xl text-slate-400 hover:text-white hover:bg-surface-border/60 border border-transparent hover:border-surface-border transition-colors touch-target"
            title="Atualizar dados"
          >
            <RefreshCw
              class="w-4 h-4"
              :class="{ 'animate-spin': routineStore.loading || habitStore.loading }"
            />
          </button>
        </div>
        <p class="text-xs sm:text-sm text-slate-400 capitalize mt-1">{{ currentDateFormatted }}</p>
      </div>

      <DailyProgressGauge
        :percentage="combinedProgressPercentage"
        :routine-percentage="routinePercentage"
        :habit-percentage="habitPercentage"
      />
    </header>

    <!-- Main Grid: Routines (Left) & Habits (Right) -->
    <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 sm:gap-8">
      <!-- Left Column: Routine Timeline -->
      <section class="lg:col-span-7 bg-surface-card border border-surface-border rounded-2xl p-4 sm:p-6 shadow-sm glow-card flex flex-col justify-between">
        <div>
          <div class="flex items-center justify-between mb-5 flex-wrap gap-2">
            <div class="flex items-center gap-2">
              <div class="p-1.5 rounded-lg bg-brand-500/10 text-brand-400">
                <Clock class="w-4 h-4 sm:w-5 sm:h-5" />
              </div>
              <h2 class="text-base sm:text-lg font-bold text-white font-outfit">Cronograma de Rotina</h2>
            </div>
            <div class="flex items-center gap-2">
              <span
                v-if="activeBlock"
                class="text-xs font-medium text-zinc-300 bg-zinc-800 border border-zinc-700 px-2.5 py-1 rounded-full hidden sm:inline"
              >
                Em andamento: {{ activeBlock.name }}
              </span>
              <button
                type="button"
                @click="focusStore.openFocusModal()"
                class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-medium bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border border-zinc-700 transition-all touch-target sm:min-h-0 sm:min-w-0"
                title="Abrir Modo Foco & Timer Pomodoro"
              >
                <Timer class="w-3.5 h-3.5 text-zinc-400" />
                <span>Modo Foco</span>
              </button>
              <button
                type="button"
                @click="isRoutineModalOpen = true"
                class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold bg-zinc-100 hover:bg-white text-zinc-950 shadow-sm transition-all touch-target sm:min-h-0 sm:min-w-0"
                title="Criar ou reconfigurar rotina"
              >
                <Plus class="w-3.5 h-3.5" />
                <span>Configurar Rotina</span>
              </button>
            </div>
          </div>

          <RoutineTimeline
            :blocks="routineStore.blocks"
            :active-block-id="activeBlock?.id"
            :completed-block-ids="completedBlockIds"
            :skipped-block-ids="skippedBlockIds"
            :loading="routineStore.loading"
            @complete="handleCompleteBlock"
            @open-create="isRoutineModalOpen = true"
            @open-focus="(b) => focusStore.openFocusModal(b)"
          />
        </div>
      </section>

      <!-- Right Column: Today's Habits -->
      <section class="lg:col-span-5 bg-surface-card border border-surface-border rounded-2xl p-4 sm:p-6 shadow-sm glow-card flex flex-col justify-between">
        <div>
          <div class="flex items-center justify-between mb-5">
            <h2 class="text-base sm:text-lg font-bold text-white font-outfit">Hábitos de Hoje</h2>
            <span class="text-xs text-slate-400 font-mono bg-surface-dark px-2.5 py-1 rounded-full border border-surface-border">
              {{ habitStore.habits.filter((h) => h.streak.is_active_today).length }}/{{ habitStore.habits.length }} feitos
            </span>
          </div>

          <HabitChecklist
            :habits="habitStore.habits"
            :loading="habitStore.loading"
            @checkin="handleHabitCheckin"
          />
        </div>
      </section>
    </div>

    <!-- Routine Manage Modal -->
    <RoutineManageModal
      :is-open="isRoutineModalOpen"
      :loading="routineStore.loading"
      @close="isRoutineModalOpen = false"
      @save="handleCreateRoutine"
    />
  </div>
</template>
