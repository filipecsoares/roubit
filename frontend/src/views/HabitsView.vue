<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { Flame, Plus, Sparkles, Trophy, Check, AlertCircle } from 'lucide-vue-next'
import { useHabitStore } from '@/stores/habit'
import StreakBadge from '@/components/habits/StreakBadge.vue'
import HeatmapCalendar from '@/components/habits/HeatmapCalendar.vue'
import HabitManageModal from '@/components/habits/HabitManageModal.vue'
import type { HabitKind } from '@/types'

const habitStore = useHabitStore()

const isModalOpen = ref(false)
const toastMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)

function showToast(text: string, type: 'success' | 'error' = 'success') {
  toastMessage.value = { text, type }
  setTimeout(() => {
    if (toastMessage.value?.text === text) {
      toastMessage.value = null
    }
  }, 4000)
}

// Stats metrics
const totalActiveHabits = computed(() => habitStore.habits.length)
const maxCurrentStreak = computed(() => {
  return Math.max(0, ...habitStore.habits.map((h) => h.streak.current_streak))
})
const maxHistoricalStreak = computed(() => {
  return Math.max(0, ...habitStore.habits.map((h) => h.streak.longest_streak))
})
const completedTodayCount = computed(() => {
  return habitStore.habits.filter((h) => h.streak.is_active_today).length
})

// Build sample heatmap activity map from habits' streak data
const aggregatedActivityMap = computed<Record<string, number>>(() => {
  const map: Record<string, number> = {}
  const today = new Date()

  // Generate heatmap marks for active streaks
  habitStore.habits.forEach((h) => {
    const streak = h.streak.current_streak
    for (let i = 0; i < streak; i++) {
      const d = new Date(today)
      d.setDate(today.getDate() - i)
      const dStr = d.toISOString().split('T')[0]
      map[dStr] = (map[dStr] || 0) + 1
    }
  })

  return map
})

async function handleCreateHabit(payload: {
  name: string
  description?: string
  habit_type: HabitKind
  target_value: number
  unit?: string
  frequency_days?: number
  color_hex?: string
}) {
  try {
    await habitStore.createHabit(payload)
    isModalOpen.value = false
    showToast('Hábito criado com sucesso!')
  } catch (err: any) {
    showToast(err.message || 'Erro ao criar hábito', 'error')
  }
}

async function handleCheckin(habitId: string, isCompletedCurrently: boolean) {
  const habitItem = habitStore.habits.find((h) => h.habit.id === habitId)
  if (!habitItem) return

  const prevActive = habitItem.streak.is_active_today
  const prevStreak = habitItem.streak.current_streak

  const willBeActive = !isCompletedCurrently
  habitItem.streak.is_active_today = willBeActive
  habitItem.streak.current_streak = willBeActive ? prevStreak + 1 : Math.max(0, prevStreak - 1)

  const todayStr = new Date().toISOString().split('T')[0]
  try {
    await habitStore.checkin(habitId, todayStr, willBeActive ? habitItem.habit.target_value : 0, false)
    showToast(willBeActive ? 'Check-in concluído!' : 'Check-in desfeito')
  } catch (err: any) {
    habitItem.streak.is_active_today = prevActive
    habitItem.streak.current_streak = prevStreak
    showToast(err.message || 'Erro ao registrar check-in', 'error')
  }
}

onMounted(() => {
  habitStore.fetchHabits().catch(() => {})
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

    <!-- Header -->
    <header class="flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div>
        <h1 class="text-3xl font-extrabold tracking-tight text-white font-outfit">
          Rastreamento de Hábitos & Streaks
        </h1>
        <p class="text-sm text-slate-400 mt-1">
          Mantenha a consistência diária e acompanhe seu histórico no mapa de calor anual.
        </p>
      </div>

      <button
        type="button"
        @click="isModalOpen = true"
        class="inline-flex items-center gap-2 px-4 py-2.5 rounded-xl text-sm font-semibold bg-brand-600 hover:bg-brand-500 text-white shadow-lg shadow-brand-500/20 transition-all"
      >
        <Plus class="w-4 h-4" />
        Novo Hábito
      </button>
    </header>

    <!-- Stat Cards Grid -->
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
      <div class="p-4 rounded-xl bg-surface-card border border-surface-border">
        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Hábitos Ativos</span>
        <p class="text-2xl font-bold text-white mt-1">{{ totalActiveHabits }}</p>
      </div>

      <div class="p-4 rounded-xl bg-surface-card border border-surface-border">
        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Maior Sequência</span>
        <div class="flex items-center gap-1.5 mt-1">
          <Flame class="w-5 h-5 text-amber-400 fill-amber-400/20" />
          <p class="text-2xl font-bold text-white">{{ maxCurrentStreak }} <span class="text-xs font-normal text-slate-400">dias</span></p>
        </div>
      </div>

      <div class="p-4 rounded-xl bg-surface-card border border-surface-border">
        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Recorde Histórico</span>
        <div class="flex items-center gap-1.5 mt-1">
          <Trophy class="w-5 h-5 text-amber-400" />
          <p class="text-2xl font-bold text-white">{{ maxHistoricalStreak }} <span class="text-xs font-normal text-slate-400">dias</span></p>
        </div>
      </div>

      <div class="p-4 rounded-xl bg-surface-card border border-surface-border">
        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Concluídos Hoje</span>
        <p class="text-2xl font-bold text-emerald-400 mt-1">
          {{ completedTodayCount }} <span class="text-xs font-normal text-slate-400">/ {{ totalActiveHabits }}</span>
        </p>
      </div>
    </div>

    <!-- Annual 365-Day Contribution Heatmap -->
    <HeatmapCalendar :activity-map="aggregatedActivityMap" />

    <!-- Habits Grid -->
    <section class="space-y-4">
      <h2 class="text-lg font-bold text-white">Todos os Hábitos</h2>

      <div
        v-if="habitStore.habits.length === 0"
        class="text-center py-16 px-4 rounded-2xl border border-dashed border-surface-border"
      >
        <Sparkles class="w-8 h-8 text-slate-500 mx-auto mb-2" />
        <h3 class="text-sm font-semibold text-white">Nenhum hábito cadastrado</h3>
        <p class="text-xs text-slate-400 mt-1 max-w-sm mx-auto">
          Crie seu primeiro hábito diário para desbloquear sequências e visualizações no mapa de calor.
        </p>
      </div>

      <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div
          v-for="item in habitStore.habits"
          :key="item.habit.id"
          class="p-5 rounded-2xl bg-surface-card border transition-all duration-200 flex flex-col justify-between space-y-4 group"
          :class="
            item.streak.is_active_today
              ? 'border-emerald-500/40 bg-emerald-950/10'
              : 'border-surface-border hover:border-slate-700'
          "
        >
          <!-- Top Row -->
          <div class="flex items-start justify-between gap-3">
            <div class="space-y-1 min-w-0">
              <h3 class="text-base font-bold text-white truncate group-hover:text-brand-300 transition-colors">
                {{ item.habit.name }}
              </h3>
              <p v-if="item.habit.description" class="text-xs text-slate-400 line-clamp-2">
                {{ item.habit.description }}
              </p>
            </div>
            <StreakBadge
              :current-streak="item.streak.current_streak"
              :longest-streak="item.streak.longest_streak"
              :is-active-today="item.streak.is_active_today"
              size="sm"
            />
          </div>

          <!-- Target & Action Row -->
          <div class="flex items-center justify-between pt-3 border-t border-surface-border/60">
            <span class="text-xs text-slate-400 font-mono">
              Meta: {{ item.habit.target_value }} {{ item.habit.unit || (item.habit.habit_type === 'binary' ? 'check' : '') }}
            </span>

            <button
              type="button"
              @click="handleCheckin(item.habit.id, item.streak.is_active_today)"
              :disabled="habitStore.loading"
              class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold transition-all duration-200 active:scale-95"
              :class="
                item.streak.is_active_today
                  ? 'bg-emerald-500 text-white shadow-md shadow-emerald-500/20'
                  : 'bg-surface-border/60 hover:bg-slate-700 text-slate-300 hover:text-white'
              "
            >
              <Check class="w-3.5 h-3.5" />
              {{ item.streak.is_active_today ? 'Concluído' : 'Check-in' }}
            </button>
          </div>
        </div>
      </div>
    </section>

    <!-- Modal for adding habit -->
    <HabitManageModal
      :is-open="isModalOpen"
      :loading="habitStore.loading"
      @close="isModalOpen = false"
      @save="handleCreateHabit"
    />
  </div>
</template>
