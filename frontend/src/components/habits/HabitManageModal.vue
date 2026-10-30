<script setup lang="ts">
import { ref, watch } from 'vue'
import { X, Sparkles } from 'lucide-vue-next'
import type { HabitKind } from '@/types'

const props = defineProps<{
  isOpen: boolean
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'save', payload: {
    name: string
    description?: string
    habit_type: HabitKind
    target_value: number
    unit?: string
    frequency_days?: number
    color_hex?: string
  }): void
}>()

const name = ref('')
const description = ref('')
const habitType = ref<HabitKind>('binary')
const targetValue = ref(1)
const unit = ref('')
const colorHex = ref('#6366f1')

const presetColors = ['#6366f1', '#10b981', '#f59e0b', '#3b82f6', '#ec4899', '#8b5cf6']

watch(habitType, (newType) => {
  if (newType === 'binary') {
    targetValue.value = 1
    unit.value = ''
  } else if (newType === 'numeric') {
    targetValue.value = 10
    unit.value = 'unidades'
  } else if (newType === 'timer') {
    targetValue.value = 30
    unit.value = 'minutos'
  }
})

function resetForm() {
  name.value = ''
  description.value = ''
  habitType.value = 'binary'
  targetValue.value = 1
  unit.value = ''
  colorHex.value = '#6366f1'
}

function handleSave() {
  if (!name.value.trim()) return
  emit('save', {
    name: name.value.trim(),
    description: description.value.trim() || undefined,
    habit_type: habitType.value,
    target_value: targetValue.value,
    unit: unit.value.trim() || undefined,
    frequency_days: 127, // All days of the week by default (binary 1111111)
    color_hex: colorHex.value,
  })
  resetForm()
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="isOpen"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-fade-in"
    >
      <div
        class="w-full max-w-lg bg-surface-card border border-surface-border rounded-2xl shadow-2xl p-6 space-y-5"
        @click.stop
      >
        <!-- Modal Header -->
        <div class="flex items-center justify-between border-b border-surface-border/60 pb-4">
          <div class="flex items-center gap-2">
            <div class="p-2 rounded-xl bg-brand-500/10 text-brand-400">
              <Sparkles class="w-5 h-5" />
            </div>
            <h3 class="text-lg font-bold text-white">Novo Hábito</h3>
          </div>
          <button
            type="button"
            @click="emit('close')"
            class="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors"
          >
            <X class="w-5 h-5" />
          </button>
        </div>

        <!-- Form Fields -->
        <form @submit.prevent="handleSave" class="space-y-4">
          <div>
            <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
              Nome do Hábito *
            </label>
            <input
              v-model="name"
              type="text"
              required
              placeholder="Ex: Meditação Mindfulness, Beber Água..."
              class="w-full px-3.5 py-2.5 rounded-xl bg-surface-dark border border-surface-border focus:border-brand-500 focus:ring-1 focus:ring-brand-500 text-sm text-white placeholder-slate-500 outline-none transition-all"
            />
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
              Descrição (opcional)
            </label>
            <input
              v-model="description"
              type="text"
              placeholder="Ex: Pelo menos 15 minutos ao acordar"
              class="w-full px-3.5 py-2.5 rounded-xl bg-surface-dark border border-surface-border focus:border-brand-500 focus:ring-1 focus:ring-brand-500 text-sm text-white placeholder-slate-500 outline-none transition-all"
            />
          </div>

          <!-- Type Selector -->
          <div>
            <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
              Tipo de Meta
            </label>
            <div class="grid grid-cols-3 gap-2">
              <button
                type="button"
                @click="habitType = 'binary'"
                class="py-2 px-3 rounded-xl border text-xs font-semibold transition-all"
                :class="habitType === 'binary' ? 'bg-brand-500/20 border-brand-500 text-brand-300' : 'border-surface-border text-slate-400 hover:text-white'"
              >
                Sim / Não
              </button>
              <button
                type="button"
                @click="habitType = 'numeric'"
                class="py-2 px-3 rounded-xl border text-xs font-semibold transition-all"
                :class="habitType === 'numeric' ? 'bg-brand-500/20 border-brand-500 text-brand-300' : 'border-surface-border text-slate-400 hover:text-white'"
              >
                Numérico
              </button>
              <button
                type="button"
                @click="habitType = 'timer'"
                class="py-2 px-3 rounded-xl border text-xs font-semibold transition-all"
                :class="habitType === 'timer' ? 'bg-brand-500/20 border-brand-500 text-brand-300' : 'border-surface-border text-slate-400 hover:text-white'"
              >
                Cronômetro
              </button>
            </div>
          </div>

          <!-- Numeric / Timer Details -->
          <div v-if="habitType !== 'binary'" class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                Meta Diária
              </label>
              <input
                v-model.number="targetValue"
                type="number"
                min="1"
                required
                class="w-full px-3.5 py-2.5 rounded-xl bg-surface-dark border border-surface-border focus:border-brand-500 text-sm text-white outline-none"
              />
            </div>
            <div>
              <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                Unidade
              </label>
              <input
                v-model="unit"
                type="text"
                placeholder="Ex: ml, páginas, min"
                class="w-full px-3.5 py-2.5 rounded-xl bg-surface-dark border border-surface-border focus:border-brand-500 text-sm text-white outline-none"
              />
            </div>
          </div>

          <!-- Color palette picker -->
          <div>
            <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
              Cor do Hábito
            </label>
            <div class="flex items-center gap-2">
              <button
                v-for="color in presetColors"
                :key="color"
                type="button"
                @click="colorHex = color"
                class="w-7 h-7 rounded-full border-2 transition-transform"
                :class="colorHex === color ? 'border-white scale-110 shadow-md' : 'border-transparent hover:scale-105'"
                :style="{ backgroundColor: color }"
              />
            </div>
          </div>

          <!-- Modal Actions -->
          <div class="flex items-center justify-end gap-3 pt-3 border-t border-surface-border/60">
            <button
              type="button"
              @click="emit('close')"
              class="px-4 py-2.5 rounded-xl border border-surface-border text-sm font-medium text-slate-300 hover:text-white hover:bg-surface-border/40 transition-colors"
            >
              Cancelar
            </button>
            <button
              type="submit"
              :disabled="loading || !name.trim()"
              class="px-5 py-2.5 rounded-xl bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-sm font-semibold text-white shadow-lg shadow-brand-500/25 transition-all"
            >
              Criar Hábito
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
