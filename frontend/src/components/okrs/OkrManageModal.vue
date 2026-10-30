<script setup lang="ts">
import { ref, watch } from 'vue'
import { X, Target, Plus, Trash2, Link2 } from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    isOpen: boolean
    defaultQuarter?: string
    habits: Array<{ id: string; name: string }>
    loading?: boolean
  }>(),
  {
    defaultQuarter: '2026-Q4',
    loading: false,
  }
)

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'save', payload: {
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
  }): void
}>()

const title = ref('')
const description = ref('')
const quarter = ref(props.defaultQuarter)
const startDate = ref('')
const endDate = ref('')

interface KrDraft {
  id: string
  title: string
  initial_value: number
  target_value: number
  unit: string
  weight: number
  linked_habit_id?: string
}

const keyResults = ref<KrDraft[]>([
  {
    id: 'kr-init',
    title: '',
    initial_value: 0,
    target_value: 100,
    unit: '%',
    weight: 1.0,
  },
])

watch(
  () => props.defaultQuarter,
  (newQuarter) => {
    if (newQuarter) quarter.value = newQuarter
  }
)

function addKeyResult() {
  keyResults.value.push({
    id: `kr-${Date.now()}`,
    title: '',
    initial_value: 0,
    target_value: 10,
    unit: 'unidades',
    weight: 1.0,
  })
}

function removeKeyResult(index: number) {
  if (keyResults.value.length > 1) {
    keyResults.value.splice(index, 1)
  }
}

function resetForm() {
  title.value = ''
  description.value = ''
  quarter.value = props.defaultQuarter
  startDate.value = ''
  endDate.value = ''
  keyResults.value = [
    {
      id: 'kr-init',
      title: '',
      initial_value: 0,
      target_value: 100,
      unit: '%',
      weight: 1.0,
    },
  ]
}

function handleSave() {
  if (!title.value.trim()) return

  // Filter out empty KRs
  const validKrs = keyResults.value
    .filter((k) => k.title.trim().length > 0)
    .map((k) => ({
      title: k.title.trim(),
      initial_value: k.initial_value || 0,
      target_value: k.target_value > 0 ? k.target_value : 100,
      unit: k.unit.trim() || 'pts',
      weight: k.weight > 0 ? k.weight : 1.0,
      linked_habit_id: k.linked_habit_id || undefined,
    }))

  emit('save', {
    title: title.value.trim(),
    description: description.value.trim() || undefined,
    quarter: quarter.value.trim() || props.defaultQuarter,
    start_date: startDate.value || undefined,
    end_date: endDate.value || undefined,
    key_results: validKrs,
  })

  resetForm()
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="isOpen"
      class="fixed inset-0 z-50 flex items-center justify-center p-2.5 sm:p-4 bg-black/75 backdrop-blur-sm animate-fade-in"
      @click="emit('close')"
    >
      <div
        class="w-full max-w-2xl bg-surface-card border border-surface-border rounded-2xl shadow-2xl p-4 sm:p-6 space-y-5 sm:space-y-6 max-h-[92vh] overflow-y-auto"
        @click.stop
      >
        <!-- Modal Header -->
        <div class="flex items-center justify-between border-b border-surface-border/60 pb-4">
          <div class="flex items-center gap-3">
            <div class="p-2 sm:p-2.5 rounded-xl bg-brand-500/10 text-brand-400 border border-brand-500/20">
              <Target class="w-4 h-4 sm:w-5 sm:h-5" />
            </div>
            <div>
              <h3 class="text-base sm:text-lg font-bold text-white font-outfit">Novo Objetivo Estratégico (OKR)</h3>
              <p class="text-xs text-slate-400">Defina o objetivo e seus Resultados-Chave mensuráveis</p>
            </div>
          </div>
          <button
            type="button"
            @click="emit('close')"
            class="p-2 rounded-xl text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors touch-target sm:min-h-0 sm:min-w-0"
          >
            <X class="w-5 h-5" />
          </button>
        </div>

        <!-- Form -->
        <form @submit.prevent="handleSave" class="space-y-4 sm:space-y-5">
          <!-- Title & Quarter -->
          <div class="grid grid-cols-1 sm:grid-cols-3 gap-3 sm:gap-4">
            <div class="sm:col-span-2 space-y-1.5">
              <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider">
                Título do Objetivo *
              </label>
              <input
                v-model="title"
                type="text"
                required
                placeholder="Ex: Conquistar Alta Produtividade e Foco"
                class="w-full px-3.5 py-2.5 rounded-xl bg-surface-dark border border-surface-border focus:border-brand-500 text-sm text-white placeholder-slate-500 outline-none transition-all"
              />
            </div>

            <div class="space-y-1.5">
              <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider">
                Trimestre *
              </label>
              <input
                v-model="quarter"
                type="text"
                required
                placeholder="2026-Q4"
                class="w-full px-3.5 py-2.5 rounded-xl bg-surface-dark border border-surface-border focus:border-brand-500 text-sm text-white font-mono outline-none"
              />
            </div>
          </div>

          <!-- Description -->
          <div class="space-y-1.5">
            <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider">
              Descrição do Objetivo (opcional)
            </label>
            <textarea
              v-model="description"
              rows="2"
              placeholder="Explique o propósito e o impacto esperado deste objetivo..."
              class="w-full px-3.5 py-2 rounded-xl bg-surface-dark border border-surface-border focus:border-brand-500 text-sm text-white placeholder-slate-500 outline-none"
            ></textarea>
          </div>

          <!-- Key Results Section -->
          <div class="space-y-3 pt-2 border-t border-surface-border/60">
            <div class="flex items-center justify-between flex-wrap gap-2">
              <div>
                <h4 class="text-xs sm:text-sm font-bold text-white font-outfit">Resultados-Chave (Key Results)</h4>
                <p class="text-[11px] sm:text-xs text-slate-400">Métricas específicas e mensuráveis para atingir o objetivo</p>
              </div>

              <button
                type="button"
                @click="addKeyResult"
                class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold bg-brand-600/20 hover:bg-brand-600/30 text-brand-300 border border-brand-500/30 transition-colors touch-target sm:min-h-0 sm:min-w-0"
              >
                <Plus class="w-3.5 h-3.5" />
                <span>Adicionar KR</span>
              </button>
            </div>

            <!-- List of Key Result Drafts -->
            <div class="space-y-3">
              <div
                v-for="(kr, idx) in keyResults"
                :key="kr.id"
                class="p-3 sm:p-4 rounded-xl bg-surface-dark/70 border border-surface-border space-y-3 relative group"
              >
                <div class="flex items-center justify-between gap-2">
                  <span class="text-xs font-bold text-slate-400 font-mono">KR #{{ idx + 1 }}</span>
                  <button
                    v-if="keyResults.length > 1"
                    type="button"
                    @click="removeKeyResult(idx)"
                    class="p-1.5 rounded-lg text-slate-500 hover:text-rose-400 transition-colors touch-target sm:min-h-0 sm:min-w-0"
                    title="Remover KR"
                  >
                    <Trash2 class="w-4 h-4" />
                  </button>
                </div>

                <div class="grid grid-cols-2 sm:grid-cols-12 gap-2.5 sm:gap-3">
                  <!-- Title -->
                  <div class="col-span-2 sm:col-span-6">
                    <input
                      v-model="kr.title"
                      type="text"
                      placeholder="Ex: Ler 12 livros técnicos..."
                      class="w-full px-3 py-2 rounded-lg bg-surface-card border border-surface-border focus:border-brand-500 text-xs text-white placeholder-slate-500 outline-none"
                    />
                  </div>

                  <!-- Target Value -->
                  <div class="col-span-1 sm:col-span-3">
                    <input
                      v-model.number="kr.target_value"
                      type="number"
                      min="1"
                      placeholder="Meta (ex: 12)"
                      class="w-full px-3 py-2 rounded-lg bg-surface-card border border-surface-border focus:border-brand-500 text-xs text-white font-mono outline-none"
                    />
                  </div>

                  <!-- Unit -->
                  <div class="col-span-1 sm:col-span-3">
                    <input
                      v-model="kr.unit"
                      type="text"
                      placeholder="Unidade (ex: livros, %)"
                      class="w-full px-3 py-2 rounded-lg bg-surface-card border border-surface-border focus:border-brand-500 text-xs text-white outline-none"
                    />
                  </div>
                </div>

                <!-- Link to habit option -->
                <div class="flex flex-wrap items-center gap-2 pt-1 text-xs">
                  <div class="flex items-center gap-1.5 text-slate-400">
                    <Link2 class="w-3.5 h-3.5 text-slate-500" />
                    <span>Vincular a hábito:</span>
                  </div>
                  <select
                    v-model="kr.linked_habit_id"
                    class="px-2.5 py-1.5 rounded-lg bg-surface-card border border-surface-border text-xs text-slate-200 outline-none flex-1 min-w-[140px]"
                  >
                    <option :value="undefined">Nenhum (progresso manual)</option>
                    <option v-for="h in habits" :key="h.id" :value="h.id">
                      {{ h.name }}
                    </option>
                  </select>
                </div>
              </div>
            </div>
          </div>

          <!-- Actions -->
          <div class="flex items-center justify-end gap-2.5 sm:gap-3 pt-4 border-t border-surface-border/60">
            <button
              type="button"
              @click="emit('close')"
              class="px-4 py-2.5 rounded-xl border border-surface-border text-sm font-medium text-slate-300 hover:text-white transition-colors touch-target sm:min-h-0 sm:min-w-0"
            >
              Cancelar
            </button>
            <button
              type="submit"
              :disabled="loading || !title.trim()"
              class="px-5 py-2.5 rounded-xl bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-sm font-semibold text-white shadow-lg shadow-brand-500/25 transition-all touch-target sm:min-h-0 sm:min-w-0"
            >
              {{ loading ? 'Salvando...' : 'Criar Objetivo' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
