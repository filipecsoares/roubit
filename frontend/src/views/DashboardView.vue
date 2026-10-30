<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { Clock, AlertCircle, RefreshCw, Sparkles, Plus } from 'lucide-vue-next'
import { useRoutineStore } from '@/stores/routine'
import { useHabitStore } from '@/stores/habit'
import RoutineTimeline from '@/components/dashboard/RoutineTimeline.vue'
import HabitChecklist from '@/components/dashboard/HabitChecklist.vue'
import DailyProgressGauge from '@/components/dashboard/DailyProgressGauge.vue'
import RoutineManageModal from '@/components/dashboard/RoutineManageModal.vue'
import type { BlockStatusType } from '@/types'

const routineStore = useRoutineStore()
const habitStore = useHabitStore()

const now = ref(new Date())
let timerId: ReturnType<typeof setInterval> | null = null

const completedBlockIds = ref<string[]>([])
const skippedBlockIds = ref<string[]>([])
const isRoutineModalOpen = ref(false)
const toastMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)

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

function showToast(text: string, type: 'success' | 'error' = 'success') {
  toastMessage.value = { text, type }
  setTimeout(() => {
    if (toastMessage.value?.text === text) {
      toastMessage.value = null
    }
  }, 4000)
}

async function handleCompleteBlock(blockId: string, status: BlockStatusType) {
  try {
    if (status === 'completed') {
      if (!completedBlockIds.value.includes(blockId)) {
        completedBlockIds.value.push(blockId)
      }
      skippedBlockIds.value = skippedBlockIds.value.filter((id) => id !== blockId)
    } else if (status === 'skipped') {
      if (!skippedBlockIds.value.includes(blockId)) {
        skippedBlockIds.value.push(blockId)
      }
      completedBlockIds.value = completedBlockIds.value.filter((id) => id !== blockId)
    }
    await routineStore.completeBlock(blockId, status)
    showToast(`Bloco atualizado como ${status === 'completed' ? 'concluído' : 'pulado'}`)
  } catch (err: any) {
    // Revert optimistic update
    completedBlockIds.value = completedBlockIds.value.filter((id) => id !== blockId)
    skippedBlockIds.value = skippedBlockIds.value.filter((id) => id !== blockId)
    showToast(err.message || 'Erro ao registrar progresso do bloco', 'error')
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
    showToast(err.message || 'Erro ao criar rotina', 'error')
  }
}

async function handleHabitCheckin(habitId: string, value: number, isIncrement: boolean) {
  const habitItem = habitStore.habits.find((h) => h.habit.id === habitId)
  if (!habitItem) return

  // Save previous state for optimistic rollback
  const prevActive = habitItem.streak.is_active_today
  const prevStreak = habitItem.streak.current_streak

  // Optimistic update
  const willBeActive = isIncrement ? true : value > 0
  habitItem.streak.is_active_today = willBeActive
  if (willBeActive && !prevActive) {
    habitItem.streak.current_streak += 1
  } else if (!willBeActive && prevActive) {
    habitItem.streak.current_streak = Math.max(0, habitItem.streak.current_streak - 1)
  }

  const todayStr = new Date().toISOString().split('T')[0]

  try {
    await habitStore.checkin(habitId, todayStr, value, isIncrement)
    showToast(willBeActive ? 'Hábito concluído!' : 'Check-in desfeito')
  } catch (err: any) {
    // Rollback on failure
    habitItem.streak.is_active_today = prevActive
    habitItem.streak.current_streak = prevStreak
    showToast(err.message || 'Falha ao registrar check-in', 'error')
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
  <div class="max-w-6xl mx-auto px-4 py-8 space-y-8 animate-fade-in">
    <!-- Toast Notification -->
    <div
      v-if="toastMessage"
      class="fixed bottom-6 right-6 z-50 flex items-center gap-2 px-4 py-3 rounded-xl border shadow-xl text-sm transition-all"
      :class="
        toastMessage.type === 'success'
          ? 'bg-emerald-950/90 border-emerald-500/50 text-emerald-200'
          : 'bg-rose-950/90 border-rose-500/50 text-rose-200'
      "
    >
      <AlertCircle v-if="toastMessage.type === 'error'" class="w-4 h-4 shrink-0" />
      <Sparkles v-else class="w-4 h-4 shrink-0 text-emerald-400" />
      <span>{{ toastMessage.text }}</span>
    </div>

    <!-- Header & Gauge Banner -->
    <header class="flex flex-col md:flex-row justify-between items-start md:items-center gap-6">
      <div>
        <div class="flex items-center gap-2">
          <h1 class="text-3xl font-extrabold tracking-tight text-white font-outfit">
            Daily Focus
          </h1>
          <button
            type="button"
            @click="loadData"
            :disabled="routineStore.loading || habitStore.loading"
            class="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors"
            title="Atualizar dados"
          >
            <RefreshCw
              class="w-4 h-4"
              :class="{ 'animate-spin': routineStore.loading || habitStore.loading }"
            />
          </button>
        </div>
        <p class="text-sm text-slate-400 capitalize mt-1">{{ currentDateFormatted }}</p>
      </div>

      <DailyProgressGauge
        :percentage="combinedProgressPercentage"
        :routine-percentage="routinePercentage"
        :habit-percentage="habitPercentage"
      />
    </header>

    <!-- Main Grid: Routines (Left) & Habits (Right) -->
    <div class="grid grid-cols-1 lg:grid-cols-12 gap-8">
      <!-- Left Column: Routine Timeline -->
      <section class="lg:col-span-7 bg-surface-card border border-surface-border rounded-2xl p-6 shadow-sm">
        <div class="flex items-center justify-between mb-5">
          <div class="flex items-center gap-2">
            <Clock class="w-5 h-5 text-brand-400" />
            <h2 class="text-lg font-bold text-white">Cronograma de Rotina</h2>
          </div>
          <div class="flex items-center gap-2">
            <span
              v-if="activeBlock"
              class="text-xs font-medium text-brand-300 bg-brand-500/10 border border-brand-500/20 px-2.5 py-1 rounded-full hidden sm:inline"
            >
              Em andamento: {{ activeBlock.name }}
            </span>
            <button
              type="button"
              @click="isRoutineModalOpen = true"
              class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold bg-brand-600/20 hover:bg-brand-600/30 text-brand-300 transition-colors"
              title="Criar ou reconfigurar rotina"
            >
              <Plus class="w-3.5 h-3.5" />
              Configurar Rotina
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
        />
      </section>

      <!-- Right Column: Today's Habits -->
      <section class="lg:col-span-5 bg-surface-card border border-surface-border rounded-2xl p-6 shadow-sm">
        <div class="flex items-center justify-between mb-5">
          <h2 class="text-lg font-bold text-white">Hábitos de Hoje</h2>
          <span class="text-xs text-slate-400">
            {{ habitStore.habits.filter((h) => h.streak.is_active_today).length }} de {{ habitStore.habits.length }} concluídos
          </span>
        </div>

        <HabitChecklist
          :habits="habitStore.habits"
          :loading="habitStore.loading"
          @checkin="handleHabitCheckin"
        />
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
