<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { Target, TrendingUp, Sparkles, AlertCircle, Plus } from 'lucide-vue-next'
import { useOkrStore } from '@/stores/okr'
import { useHabitStore } from '@/stores/habit'
import ObjectiveCard from '@/components/okrs/ObjectiveCard.vue'
import LinkHabitModal from '@/components/okrs/LinkHabitModal.vue'
import OkrManageModal from '@/components/okrs/OkrManageModal.vue'

const okrStore = useOkrStore()
const habitStore = useHabitStore()

const quarters = ['2026-Q4', '2026-Q3', '2027-Q1', '2027-Q2']
const selectedQuarter = ref('2026-Q4')

// Modals state
const isLinkModalOpen = ref(false)
const selectedKrId = ref<string | null>(null)
const selectedKrTitle = ref('')

const isCreateModalOpen = ref(false)

// Toast notifications
const toastMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)

function showToast(text: string, type: 'success' | 'error' = 'success') {
  toastMessage.value = { text, type }
  setTimeout(() => {
    if (toastMessage.value?.text === text) {
      toastMessage.value = null
    }
  }, 4000)
}

// Habits map: habitId -> habitName
const habitsMap = computed<Record<string, string>>(() => {
  const map: Record<string, string> = {}
  habitStore.habits.forEach((h) => {
    map[h.habit.id] = h.habit.name
  })
  return map
})

const habitsList = computed(() => {
  return habitStore.habits.map((h) => ({
    id: h.habit.id,
    name: h.habit.name,
  }))
})

// Analytics metrics
const totalObjectives = computed(() => okrStore.objectives.length)

const averageProgress = computed(() => {
  if (okrStore.objectives.length === 0) return 0
  const sum = okrStore.objectives.reduce((acc, curr) => acc + curr.objective.progress_percentage, 0)
  return Math.round(sum / okrStore.objectives.length)
})

const totalKeyResults = computed(() => {
  return okrStore.objectives.reduce((acc, curr) => acc + curr.key_results.length, 0)
})

const linkedKeyResultsCount = computed(() => {
  return okrStore.objectives.reduce((acc, curr) => {
    return acc + curr.key_results.filter((kr) => !!kr.linked_habit_id).length
  }, 0)
})

function openLinkModal(krId: string) {
  for (const obj of okrStore.objectives) {
    const kr = obj.key_results.find((k) => k.id === krId)
    if (kr) {
      selectedKrId.value = kr.id
      selectedKrTitle.value = kr.title
      isLinkModalOpen.value = true
      return
    }
  }
}

function handleLinkConfirm(habitId: string) {
  if (!selectedKrId.value) return

  // Update in local store
  for (const obj of okrStore.objectives) {
    const kr = obj.key_results.find((k) => k.id === selectedKrId.value)
    if (kr) {
      kr.linked_habit_id = habitId
      const habitName = habitsMap.value[habitId] || 'Hábito'
      showToast(`Resultado-Chave vinculado ao hábito "${habitName}"!`)
      break
    }
  }
  isLinkModalOpen.value = false
  selectedKrId.value = null
}

async function handleQuarterChange(quarter: string) {
  selectedQuarter.value = quarter
  try {
    await okrStore.fetchOkrs(quarter)
  } catch (err: any) {
    showToast(err.message || 'Erro ao carregar OKRs do trimestre', 'error')
  }
}

async function handleCreateObjective(payload: {
  title: string
  description?: string
  quarter: string
  start_date?: string
  end_date?: string
  key_results: Array<{
    title: string
    initial_value: number
    target_value: number
    unit: string
    weight: number
    linked_habit_id?: string
  }>
}) {
  try {
    await okrStore.createObjective(payload)
    isCreateModalOpen.value = false
    showToast('Objetivo e Resultados-Chave cadastrados com sucesso!')
  } catch (err: any) {
    showToast(err.message || 'Erro ao criar objetivo', 'error')
  }
}

async function handleDeleteObjective(objectiveId: string) {
  const obj = okrStore.objectives.find((o) => o.objective.id === objectiveId)
  const title = obj?.objective.title || 'este objetivo'
  if (!confirm(`Tem certeza que deseja excluir "${title}" e todos os seus Resultados-Chave?`)) return
  try {
    await okrStore.deleteObjective(objectiveId)
    showToast(`Objetivo "${title}" excluído com sucesso.`)
  } catch (err: any) {
    showToast(err.message || 'Erro ao excluir objetivo', 'error')
  }
}

onMounted(async () => {
  try {
    await Promise.allSettled([
      okrStore.fetchOkrs(selectedQuarter.value),
      habitStore.fetchHabits(),
    ])
  } catch (e) {}
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

    <!-- Header & Actions -->
    <header class="flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div>
        <h1 class="text-3xl font-extrabold tracking-tight text-white font-outfit">
          Objetivos & Resultados-Chave (OKRs)
        </h1>
        <p class="text-sm text-slate-400 mt-1">
          Alinhamento estratégico conectando metas trimestrais diretamente à cadência dos seus hábitos diários.
        </p>
      </div>

      <div class="flex items-center gap-3">
        <!-- Quarter Pills -->
        <div class="flex items-center gap-1.5 p-1 bg-surface-card rounded-xl border border-surface-border">
          <button
            v-for="q in quarters"
            :key="q"
            type="button"
            @click="handleQuarterChange(q)"
            class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-all"
            :class="
              selectedQuarter === q
                ? 'bg-brand-600 text-white shadow-sm'
                : 'text-slate-400 hover:text-white'
            "
          >
            {{ q }}
          </button>
        </div>

        <!-- New Objective Button -->
        <button
          type="button"
          @click="isCreateModalOpen = true"
          class="inline-flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold bg-brand-600 hover:bg-brand-500 text-white shadow-lg shadow-brand-500/20 transition-all shrink-0"
        >
          <Plus class="w-4 h-4" />
          Novo Objetivo
        </button>
      </div>
    </header>

    <!-- Overview Stats Grid -->
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
      <div class="p-4 rounded-xl bg-surface-card border border-surface-border">
        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Objetivos</span>
        <p class="text-2xl font-bold text-white mt-1">{{ totalObjectives }}</p>
      </div>

      <div class="p-4 rounded-xl bg-surface-card border border-surface-border">
        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Progresso Médio</span>
        <div class="flex items-center gap-1.5 mt-1">
          <TrendingUp class="w-5 h-5 text-brand-400" />
          <p class="text-2xl font-bold text-white">{{ averageProgress }}%</p>
        </div>
      </div>

      <div class="p-4 rounded-xl bg-surface-card border border-surface-border">
        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Resultados-Chave</span>
        <p class="text-2xl font-bold text-white mt-1">{{ totalKeyResults }}</p>
      </div>

      <div class="p-4 rounded-xl bg-surface-card border border-surface-border">
        <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">KRs com Hábitos</span>
        <p class="text-2xl font-bold text-emerald-400 mt-1">
          {{ linkedKeyResultsCount }} <span class="text-xs font-normal text-slate-400">/ {{ totalKeyResults }}</span>
        </p>
      </div>
    </div>

    <!-- Objectives Tree Section -->
    <section class="space-y-4">
      <div class="flex items-center justify-between">
        <h2 class="text-lg font-bold text-white">Objetivos em {{ selectedQuarter }}</h2>
      </div>

      <!-- Empty state with CTA button -->
      <div
        v-if="okrStore.objectives.length === 0"
        class="text-center py-16 px-4 rounded-2xl border border-dashed border-surface-border space-y-4"
      >
        <div class="inline-flex p-3 rounded-2xl bg-surface-border/40 text-slate-400">
          <Target class="w-8 h-8 text-slate-400 mx-auto" />
        </div>
        <div class="max-w-sm mx-auto">
          <h3 class="text-sm font-semibold text-white">Nenhum objetivo cadastrado para este trimestre</h3>
          <p class="text-xs text-slate-400 mt-1">
            Defina seus objetivos estratégicos e adicione Resultados-Chave mensuráveis para impulsionar suas metas.
          </p>
        </div>
        <button
          type="button"
          @click="isCreateModalOpen = true"
          class="inline-flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold bg-brand-600 hover:bg-brand-500 text-white shadow-lg shadow-brand-500/20 transition-all"
        >
          <Plus class="w-4 h-4" />
          Cadastrar Primeiro Objetivo
        </button>
      </div>

      <div v-else class="space-y-4">
        <ObjectiveCard
          v-for="item in okrStore.objectives"
          :key="item.objective.id"
          :objective="item.objective"
          :key-results="item.key_results"
          :habits-map="habitsMap"
          @link-habit="openLinkModal"
          @delete="handleDeleteObjective"
        />
      </div>
    </section>

    <!-- Link Habit Modal -->
    <LinkHabitModal
      :is-open="isLinkModalOpen"
      :kr-title="selectedKrTitle"
      :habits="habitsList"
      @close="isLinkModalOpen = false"
      @link="handleLinkConfirm"
    />

    <!-- Create Objective Modal -->
    <OkrManageModal
      :is-open="isCreateModalOpen"
      :default-quarter="selectedQuarter"
      :habits="habitsList"
      :loading="okrStore.loading"
      @close="isCreateModalOpen = false"
      @save="handleCreateObjective"
    />
  </div>
</template>
