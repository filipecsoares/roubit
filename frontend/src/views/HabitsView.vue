<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { Flame, Plus, Sparkles, Trophy, Check, AlertCircle, Trash2, Archive } from 'lucide-vue-next'
import { useHabitStore } from '@/stores/habit'
import StreakBadge from '@/components/habits/StreakBadge.vue'
import HeatmapCalendar from '@/components/habits/HeatmapCalendar.vue'
import HabitManageModal from '@/components/habits/HabitManageModal.vue'
import type { HabitKind } from '@/types'
import { formatLocalDate, addDaysLocal } from '@/utils/date'
import { useToast } from '@/composables/useToast'

const habitStore = useHabitStore()
const { toastMessage, showToast } = useToast()

const isModalOpen = ref(false)

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

// Real 365-day heatmap activity map from backend analytics (with fallback to active streaks)
const aggregatedActivityMap = computed<Record<string, number>>(() => {
  if (habitStore.analytics?.aggregated_activity_map) {
    return habitStore.analytics.aggregated_activity_map
  }
  const map: Record<string, number> = {}
  const todayStr = formatLocalDate()

  // Fallback: Generate heatmap marks from active streaks if analytics not yet loaded
  habitStore.habits.forEach((h) => {
    const streak = h.streak.current_streak
    for (let i = 0; i < streak; i++) {
      const dStr = addDaysLocal(todayStr, -i)
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
    await habitStore.fetchAnalytics().catch(() => {})
    showToast('Hábito criado com sucesso!')
  } catch (err: any) {
    showToast(err, 'error')
  }
}

async function handleCheckin(habitId: string, isCompletedCurrently: boolean) {
  const habitItem = habitStore.habits.find((h) => h.habit.id === habitId)
  if (!habitItem) return

  const prevActive = habitItem.streak.is_active_today
  const prevStreak = habitItem.streak.current_streak
  const prevValue = habitItem.today_value ?? 0

  const willBeActive = !isCompletedCurrently
  const nextValue = willBeActive ? habitItem.habit.target_value : 0
  habitItem.today_value = nextValue
  habitItem.streak.is_active_today = willBeActive
  habitItem.streak.current_streak = willBeActive ? prevStreak + 1 : Math.max(0, prevStreak - 1)

  const todayStr = formatLocalDate()
  try {
    await habitStore.checkin(habitId, todayStr, nextValue, false)
    await habitStore.fetchAnalytics().catch(() => {})
    showToast(willBeActive ? 'Check-in concluído!' : 'Check-in desfeito')
  } catch (err: any) {
    habitItem.today_value = prevValue
    habitItem.streak.is_active_today = prevActive
    habitItem.streak.current_streak = prevStreak
    showToast(err, 'error')
  }
}

async function handleArchiveHabit(habitId: string, habitName: string) {
  if (!confirm(`Deseja arquivar o hábito "${habitName}"?`)) return
  try {
    await habitStore.archiveHabit(habitId)
    await habitStore.fetchAnalytics().catch(() => {})
    showToast(`Hábito "${habitName}" arquivado com sucesso.`)
  } catch (err: any) {
    showToast(err, 'error')
  }
}

async function handleDeleteHabit(habitId: string, habitName: string) {
  if (!confirm(`Tem certeza que deseja excluir o hábito "${habitName}"?`)) return
  try {
    await habitStore.deleteHabit(habitId)
    await habitStore.fetchAnalytics().catch(() => {})
    showToast(`Hábito "${habitName}" excluído com sucesso.`)
  } catch (err: any) {
    showToast(err, 'error')
  }
}

onMounted(() => {
  habitStore.fetchHabits().catch(() => {})
  habitStore.fetchAnalytics().catch(() => {})
})
</script>

<template>
  <div class="space-y-6 sm:space-y-8 animate-fade-in">
    <!-- Toast Notification -->
    <div
      v-if="toastMessage"
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

    <!-- Header -->
    <header class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
      <div>
        <h1 class="text-2xl sm:text-3xl font-extrabold tracking-tight text-white font-outfit">
          Rastreamento de Hábitos & Streaks
        </h1>
        <p class="text-xs sm:text-sm text-slate-400 mt-1">
          Mantenha a consistência diária e acompanhe seu histórico no mapa de calor anual.
        </p>
      </div>

      <button
        type="button"
        @click="isModalOpen = true"
        class="inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-xs sm:text-sm font-semibold bg-zinc-100 hover:bg-white text-zinc-950 shadow-sm transition-all touch-target sm:min-h-0 sm:min-w-0"
      >
        <Plus class="w-4 h-4" />
        <span>Novo Hábito</span>
      </button>
    </header>

    <!-- Stat Cards Grid (2 cols mobile, 4 cols desktop) -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-3 sm:gap-4">
      <div class="p-3.5 sm:p-4 rounded-2xl bg-surface-card border border-surface-border shadow-sm glow-card">
        <span class="text-[11px] sm:text-xs font-semibold text-slate-400 uppercase tracking-wider">Hábitos Ativos</span>
        <p class="text-xl sm:text-2xl font-extrabold text-white mt-1 font-mono">{{ totalActiveHabits }}</p>
      </div>

      <div class="p-3.5 sm:p-4 rounded-2xl bg-surface-card border border-surface-border shadow-sm glow-card">
        <span class="text-[11px] sm:text-xs font-semibold text-slate-400 uppercase tracking-wider">Maior Sequência</span>
        <div class="flex items-center gap-1.5 mt-1">
          <Flame class="w-5 h-5 text-amber-400 fill-amber-400/20" />
          <p class="text-xl sm:text-2xl font-extrabold text-white font-mono">
            {{ maxCurrentStreak }} <span class="text-xs font-normal text-slate-400 font-sans">dias</span>
          </p>
        </div>
      </div>

      <div class="p-3.5 sm:p-4 rounded-2xl bg-surface-card border border-surface-border shadow-sm glow-card">
        <span class="text-[11px] sm:text-xs font-semibold text-slate-400 uppercase tracking-wider">Recorde Histórico</span>
        <div class="flex items-center gap-1.5 mt-1">
          <Trophy class="w-5 h-5 text-amber-400" />
          <p class="text-xl sm:text-2xl font-extrabold text-white font-mono">
            {{ maxHistoricalStreak }} <span class="text-xs font-normal text-slate-400 font-sans">dias</span>
          </p>
        </div>
      </div>

      <div class="p-3.5 sm:p-4 rounded-2xl bg-surface-card border border-surface-border shadow-sm glow-card">
        <span class="text-[11px] sm:text-xs font-semibold text-slate-400 uppercase tracking-wider">Concluídos Hoje</span>
        <p class="text-xl sm:text-2xl font-extrabold text-emerald-400 mt-1 font-mono">
          {{ completedTodayCount }} <span class="text-xs font-normal text-slate-400 font-sans">/ {{ totalActiveHabits }}</span>
        </p>
      </div>
    </div>

    <!-- Annual 365-Day Contribution Heatmap -->
    <HeatmapCalendar :activity-map="aggregatedActivityMap" />

    <!-- Habits Grid -->
    <section class="space-y-4">
      <div class="flex items-center justify-between">
        <h2 class="text-base sm:text-lg font-bold text-white font-outfit">Todos os Hábitos</h2>
        <span class="text-xs text-slate-400 font-mono">{{ habitStore.habits.length }} cadastrados</span>
      </div>

      <!-- Loading state -->
      <div
        v-if="habitStore.loading && habitStore.habits.length === 0"
        class="text-center py-16 px-4 rounded-2xl border border-dashed border-surface-border/40 animate-pulse space-y-2"
      >
        <Sparkles class="w-8 h-8 text-slate-500 mx-auto mb-2 animate-spin" />
        <h3 class="text-sm font-semibold text-slate-300">Carregando hábitos...</h3>
      </div>

      <div
        v-else-if="habitStore.habits.length === 0"
        class="text-center py-16 px-4 rounded-2xl border border-dashed border-surface-border"
      >
        <Sparkles class="w-8 h-8 text-slate-500 mx-auto mb-2" />
        <h3 class="text-sm font-semibold text-white">Nenhum hábito cadastrado</h3>
        <p class="text-xs text-slate-400 mt-1 max-w-sm mx-auto">
          Crie seu primeiro hábito diário para desbloquear sequências e visualizações no mapa de calor.
        </p>
      </div>

      <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3.5 sm:gap-4">
        <div
          v-for="item in habitStore.habits"
          :key="item.habit.id"
          class="p-4 sm:p-5 rounded-2xl bg-surface-card border transition-all duration-200 flex flex-col justify-between space-y-4 group glow-card"
          :class="
            item.streak.is_active_today
              ? 'border-emerald-500/40 bg-emerald-950/10'
              : 'border-surface-border hover:border-slate-700'
          "
        >
          <!-- Top Row -->
          <div class="flex items-start justify-between gap-3">
            <div class="space-y-1 min-w-0">
              <h3 class="text-sm sm:text-base font-bold text-white truncate group-hover:text-brand-300 transition-colors">
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

            <div class="flex items-center gap-1.5 sm:gap-2">
              <button
                type="button"
                @click.stop="handleArchiveHabit(item.habit.id, item.habit.name)"
                title="Arquivar hábito"
                class="p-2 rounded-xl text-slate-500 hover:text-amber-400 hover:bg-amber-500/10 border border-transparent hover:border-amber-500/20 transition-all touch-target sm:min-h-0 sm:min-w-0"
              >
                <Archive class="w-4 h-4" />
              </button>

              <button
                type="button"
                @click.stop="handleDeleteHabit(item.habit.id, item.habit.name)"
                title="Excluir hábito"
                class="p-2 rounded-xl text-slate-500 hover:text-rose-400 hover:bg-rose-500/10 border border-transparent hover:border-rose-500/20 transition-all touch-target sm:min-h-0 sm:min-w-0"
              >
                <Trash2 class="w-4 h-4" />
              </button>

              <button
                type="button"
                @click="handleCheckin(item.habit.id, item.streak.is_active_today)"
                :disabled="habitStore.loading"
                class="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-semibold transition-all duration-200 active:scale-95 touch-target sm:min-h-0 sm:min-w-0"
                :class="
                  item.streak.is_active_today
                    ? 'bg-emerald-500 text-white shadow-md shadow-emerald-500/20'
                    : 'bg-surface-border/60 hover:bg-slate-700 text-slate-300 hover:text-white border border-surface-border'
                "
              >
                <Check class="w-3.5 h-3.5" />
                <span>{{ item.streak.is_active_today ? 'Concluído' : 'Check-in' }}</span>
              </button>
            </div>
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
