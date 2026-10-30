<script setup lang="ts">
import { ref, computed } from 'vue'
import { Sparkles, Trophy } from 'lucide-vue-next'
import HabitCheckinRow from './HabitCheckinRow.vue'
import type { HabitWithStreakResponse } from '@/types'

const props = withDefaults(
  defineProps<{
    habits: HabitWithStreakResponse[]
    loading?: boolean
  }>(),
  {
    loading: false,
  }
)

const emit = defineEmits<{
  (e: 'checkin', habitId: string, value: number, isIncrement: boolean): void
}>()

type FilterTab = 'all' | 'pending' | 'completed'
const activeTab = ref<FilterTab>('all')

const pendingHabits = computed(() => {
  return props.habits.filter((h) => !h.streak.is_active_today)
})

const completedHabits = computed(() => {
  return props.habits.filter((h) => h.streak.is_active_today)
})

const filteredHabits = computed(() => {
  if (activeTab.value === 'pending') return pendingHabits.value
  if (activeTab.value === 'completed') return completedHabits.value
  return props.habits
})
</script>

<template>
  <div class="space-y-4">
    <!-- Filter Tabs -->
    <div class="flex items-center gap-1.5 p-1 bg-surface-card/60 rounded-xl border border-surface-border/60">
      <button
        type="button"
        @click="activeTab = 'all'"
        class="flex-1 py-1.5 px-3 rounded-lg text-xs font-medium transition-all"
        :class="activeTab === 'all' ? 'bg-surface-border text-white shadow-sm' : 'text-slate-400 hover:text-slate-200'"
      >
        Todos ({{ habits.length }})
      </button>

      <button
        type="button"
        @click="activeTab = 'pending'"
        class="flex-1 py-1.5 px-3 rounded-lg text-xs font-medium transition-all"
        :class="activeTab === 'pending' ? 'bg-surface-border text-white shadow-sm' : 'text-slate-400 hover:text-slate-200'"
      >
        Pendentes ({{ pendingHabits.length }})
      </button>

      <button
        type="button"
        @click="activeTab = 'completed'"
        class="flex-1 py-1.5 px-3 rounded-lg text-xs font-medium transition-all"
        :class="activeTab === 'completed' ? 'bg-surface-border text-white shadow-sm' : 'text-slate-400 hover:text-slate-200'"
      >
        Concluídos ({{ completedHabits.length }})
      </button>
    </div>

    <!-- Empty state: No habits at all -->
    <div
      v-if="habits.length === 0"
      class="text-center py-12 px-4 rounded-xl border border-dashed border-surface-border"
    >
      <div class="inline-flex p-3 rounded-2xl bg-surface-border/40 text-slate-400 mb-3">
        <Sparkles class="w-6 h-6" />
      </div>
      <h3 class="text-sm font-medium text-white">Nenhum hábito cadastrado</h3>
      <p class="text-xs text-slate-400 mt-1 max-w-xs mx-auto">
        Crie hábitos diários para acompanhar seu progresso e construir sequências vitoriosas.
      </p>
    </div>

    <!-- Empty state: All completed for pending tab -->
    <div
      v-else-if="activeTab === 'pending' && pendingHabits.length === 0"
      class="text-center py-10 px-4 rounded-xl border border-emerald-500/20 bg-emerald-950/10"
    >
      <div class="inline-flex p-3 rounded-2xl bg-emerald-500/20 text-emerald-400 mb-2 animate-bounce-short">
        <Trophy class="w-6 h-6" />
      </div>
      <h3 class="text-sm font-semibold text-emerald-300">Todos os hábitos de hoje foram concluídos!</h3>
      <p class="text-xs text-slate-400 mt-1">
        Parabéns pela dedicação. Mantenha a sequência ativa amanhã!
      </p>
    </div>

    <!-- Empty state: None completed for completed tab -->
    <div
      v-else-if="activeTab === 'completed' && completedHabits.length === 0"
      class="text-center py-8 px-4 text-xs text-slate-400"
    >
      Nenhum hábito concluído ainda hoje. Comece dando o primeiro check-in!
    </div>

    <!-- Habits List with TransitionGroup -->
    <TransitionGroup
      v-else
      name="list"
      tag="div"
      class="space-y-2.5"
    >
      <HabitCheckinRow
        v-for="item in filteredHabits"
        :key="item.habit.id"
        :habit="item.habit"
        :streak="item.streak"
        :is-completed-today="item.streak.is_active_today"
        :loading="loading"
        @checkin="(id, val, inc) => emit('checkin', id, val, inc)"
      />
    </TransitionGroup>
  </div>
</template>

<style scoped>
.list-enter-active,
.list-leave-active {
  transition: all 0.25s ease;
}
.list-enter-from,
.list-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
