<script setup lang="ts">
import { ref } from 'vue'
import { X, Clock, Plus, Trash2 } from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    isOpen: boolean
    loading?: boolean
  }>(),
  {
    loading: false,
  }
)

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'save', payload: {
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
  }): void
}>()

const routineName = ref('Rotina Diária de Foco')
const routineDescription = ref('')

interface BlockDraft {
  id: string
  name: string
  start_time: string
  end_time: string
  category: string
  color_hex: string
}

const blocks = ref<BlockDraft[]>([
  {
    id: 'b-1',
    name: 'Foco Profundo / Deep Work',
    start_time: '08:00',
    end_time: '11:00',
    category: 'Trabalho',
    color_hex: '#6366f1',
  },
  {
    id: 'b-2',
    name: 'Almoço & Descanso',
    start_time: '12:00',
    end_time: '13:00',
    category: 'Saúde',
    color_hex: '#10b981',
  },
  {
    id: 'b-3',
    name: 'Bloco de Execução & Reuniões',
    start_time: '14:00',
    end_time: '17:00',
    category: 'Trabalho',
    color_hex: '#3b82f6',
  },
])

const validationError = ref<string | null>(null)

function addBlock() {
  const lastBlock = blocks.value[blocks.value.length - 1]
  let newStart = '18:00'
  let newEnd = '19:00'
  if (lastBlock) {
    newStart = lastBlock.end_time
    const [h, m] = newStart.split(':').map(Number)
    const endH = Math.min(23, h + 1)
    newEnd = `${String(endH).padStart(2, '0')}:${String(m).padStart(2, '0')}`
  }

  blocks.value.push({
    id: `b-${Date.now()}`,
    name: '',
    start_time: newStart,
    end_time: newEnd,
    category: 'Foco',
    color_hex: '#6366f1',
  })
}

function removeBlock(index: number) {
  if (blocks.value.length > 1) {
    blocks.value.splice(index, 1)
  }
}

function handleSave() {
  validationError.value = null

  if (!routineName.value.trim()) {
    validationError.value = 'Informe o nome da rotina.'
    return
  }

  const validBlocks = blocks.value.filter((b) => b.name.trim().length > 0)
  if (validBlocks.length === 0) {
    validationError.value = 'Cadastre pelo menos um bloco com nome válido.'
    return
  }

  // Validate start_time < end_time
  for (const b of validBlocks) {
    if (b.start_time >= b.end_time) {
      validationError.value = `O bloco "${b.name}" tem horário inicial maior ou igual ao final (${b.start_time} - ${b.end_time}).`
      return
    }
  }

  // Sort blocks chronologically
  validBlocks.sort((a, b) => a.start_time.localeCompare(b.start_time))

  emit('save', {
    name: routineName.value.trim(),
    description: routineDescription.value.trim() || undefined,
    is_active: true,
    blocks: validBlocks.map((b, idx) => ({
      name: b.name.trim(),
      start_time: b.start_time.length === 5 ? `${b.start_time}:00` : b.start_time,
      end_time: b.end_time.length === 5 ? `${b.end_time}:00` : b.end_time,
      category: b.category || undefined,
      color_hex: b.color_hex || '#6366f1',
      order_index: idx,
    })),
  })
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="isOpen"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm animate-fade-in"
      @click="emit('close')"
    >
      <div
        class="w-full max-w-2xl bg-surface-card border border-surface-border rounded-2xl shadow-2xl p-6 space-y-6 max-h-[90vh] overflow-y-auto"
        @click.stop
      >
        <!-- Modal Header -->
        <div class="flex items-center justify-between border-b border-surface-border/60 pb-4">
          <div class="flex items-center gap-3">
            <div class="p-2.5 rounded-xl bg-brand-500/10 text-brand-400 border border-brand-500/20">
              <Clock class="w-5 h-5" />
            </div>
            <div>
              <h3 class="text-lg font-bold text-white">Criar / Configurar Rotina</h3>
              <p class="text-xs text-slate-400">Estruture seus blocos de tempo diários e rotina ativa</p>
            </div>
          </div>
          <button
            type="button"
            @click="emit('close')"
            class="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors"
          >
            <X class="w-5 h-5" />
          </button>
        </div>

        <!-- Validation Error Message -->
        <div
          v-if="validationError"
          class="p-3 rounded-xl bg-rose-950/60 border border-rose-500/40 text-xs text-rose-300"
        >
          {{ validationError }}
        </div>

        <!-- Form -->
        <form @submit.prevent="handleSave" class="space-y-5">
          <!-- Routine Info -->
          <div class="space-y-3">
            <div>
              <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                Nome da Rotina *
              </label>
              <input
                v-model="routineName"
                type="text"
                required
                placeholder="Ex: Rotina Semanal de Foco"
                class="w-full px-3.5 py-2.5 rounded-xl bg-surface-dark border border-surface-border focus:border-brand-500 text-sm text-white placeholder-slate-500 outline-none transition-all"
              />
            </div>

            <div>
              <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                Descrição (opcional)
              </label>
              <input
                v-model="routineDescription"
                type="text"
                placeholder="Ex: Cronograma otimizado para dias úteis"
                class="w-full px-3.5 py-2 rounded-xl bg-surface-dark border border-surface-border focus:border-brand-500 text-sm text-white placeholder-slate-500 outline-none"
              />
            </div>
          </div>

          <!-- Time Blocks Builder -->
          <div class="space-y-3 pt-2 border-t border-surface-border/60">
            <div class="flex items-center justify-between">
              <div>
                <h4 class="text-sm font-bold text-white">Blocos de Tempo (Time Blocks)</h4>
                <p class="text-xs text-slate-400">Defina os horários de início e término de cada atividade</p>
              </div>

              <button
                type="button"
                @click="addBlock"
                class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold bg-brand-600/20 hover:bg-brand-600/30 text-brand-300 transition-colors"
              >
                <Plus class="w-3.5 h-3.5" />
                Adicionar Bloco
              </button>
            </div>

            <!-- Blocks List -->
            <div class="space-y-3">
              <div
                v-for="(block, idx) in blocks"
                :key="block.id"
                class="p-4 rounded-xl bg-surface-dark/70 border border-surface-border space-y-3"
              >
                <div class="flex items-center justify-between gap-2">
                  <span class="text-xs font-bold text-slate-400 font-mono">Bloco #{{ idx + 1 }}</span>
                  <button
                    v-if="blocks.length > 1"
                    type="button"
                    @click="removeBlock(idx)"
                    class="p-1 rounded text-slate-500 hover:text-rose-400 transition-colors"
                    title="Remover bloco"
                  >
                    <Trash2 class="w-4 h-4" />
                  </button>
                </div>

                <div class="grid grid-cols-1 md:grid-cols-12 gap-3">
                  <!-- Name -->
                  <div class="md:col-span-5">
                    <label class="block text-[11px] text-slate-400 mb-1">Nome do Bloco</label>
                    <input
                      v-model="block.name"
                      type="text"
                      placeholder="Ex: Foco Profundo"
                      class="w-full px-3 py-2 rounded-lg bg-surface-card border border-surface-border focus:border-brand-500 text-xs text-white placeholder-slate-500 outline-none"
                    />
                  </div>

                  <!-- Start Time -->
                  <div class="md:col-span-2">
                    <label class="block text-[11px] text-slate-400 mb-1">Início</label>
                    <input
                      v-model="block.start_time"
                      type="time"
                      required
                      class="w-full px-2.5 py-2 rounded-lg bg-surface-card border border-surface-border focus:border-brand-500 text-xs text-white font-mono outline-none"
                    />
                  </div>

                  <!-- End Time -->
                  <div class="md:col-span-2">
                    <label class="block text-[11px] text-slate-400 mb-1">Fim</label>
                    <input
                      v-model="block.end_time"
                      type="time"
                      required
                      class="w-full px-2.5 py-2 rounded-lg bg-surface-card border border-surface-border focus:border-brand-500 text-xs text-white font-mono outline-none"
                    />
                  </div>

                  <!-- Category -->
                  <div class="md:col-span-3">
                    <label class="block text-[11px] text-slate-400 mb-1">Categoria</label>
                    <input
                      v-model="block.category"
                      type="text"
                      placeholder="Trabalho, Saúde..."
                      class="w-full px-2.5 py-2 rounded-lg bg-surface-card border border-surface-border focus:border-brand-500 text-xs text-white outline-none"
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Actions -->
          <div class="flex items-center justify-end gap-3 pt-4 border-t border-surface-border/60">
            <button
              type="button"
              @click="emit('close')"
              class="px-4 py-2.5 rounded-xl border border-surface-border text-sm font-medium text-slate-300 hover:text-white transition-colors"
            >
              Cancelar
            </button>
            <button
              type="submit"
              :disabled="loading || !routineName.trim()"
              class="px-5 py-2.5 rounded-xl bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-sm font-semibold text-white shadow-lg shadow-brand-500/25 transition-all"
            >
              {{ loading ? 'Salvando...' : 'Salvar Rotina' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
