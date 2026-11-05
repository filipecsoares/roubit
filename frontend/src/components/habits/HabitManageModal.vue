<script setup lang="ts">
import { ref, watch } from 'vue'
import { X, Sparkles } from 'lucide-vue-next'
import { useModal } from '@/composables/useModal'
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

useModal(() => props.isOpen, () => emit('close'))

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

watch(
  () => props.isOpen,
  (open, wasOpen) => {
    if (!open && wasOpen) {
      resetForm()
    }
  }
)

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
        class="w-full max-w-lg bg-surface-card border border-surface-border rounded-2xl shadow-2xl p-4 sm:p-6 space-y-5 max-h-[92vh] overflow-y-auto"
        @click.stop
      >
        <!-- Modal Header -->
        <div class="flex items-center justify-between border-b border-surface-border/60 pb-4">
          <div class="flex items-center gap-2.5">
            <div class="p-2 sm:p-2.5 rounded-xl bg-zinc-800 text-zinc-100 border border-zinc-700">
              <Sparkles class="w-4 h-4 sm:w-5 sm:h-5" />
            </div>
            <div>
              <h3 class="text-base sm:text-lg font-bold text-white font-outfit">Novo Hábito</h3>
              <p class="text-xs text-zinc-400">Defina a cadência e meta diária do hábito</p>
            </div>
          </div>
          <button
            type="button"
            @click="emit('close')"
            class="p-2 rounded-xl text-zinc-400 hover:text-white hover:bg-surface-border/60 transition-colors touch-target sm:min-h-0 sm:min-w-0"
          >
            <X class="w-5 h-5" />
          </button>
        </div>

        <!-- Form Fields -->
        <form @submit.prevent="handleSave" class="space-y-4">
          <div>
            <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1.5">
              Nome do Hábito *
            </label>
            <input
              v-model="name"
              type="text"
              required
              placeholder="Ex: Meditação Mindfulness, Beber Água..."
              class="w-full px-3.5 py-2.5 rounded-xl bg-surface-dark border border-surface-border focus:border-zinc-500 text-sm text-white placeholder-zinc-500 outline-none transition-all"
            />
          </div>

          <div>
            <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1.5">
              Descrição (opcional)
            </label>
            <input
              v-model="description"
              type="text"
              placeholder="Ex: Pelo menos 15 minutos ao acordar"
              class="w-full px-3.5 py-2.5 rounded-xl bg-surface-dark border border-surface-border focus:border-zinc-500 text-sm text-white placeholder-zinc-500 outline-none transition-all"
            />
          </div>

          <!-- Type Selector -->
          <div>
            <label class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1.5">
              Tipo de Meta
            </label>
            <div class="grid grid-cols-3 gap-2">
              <button
                type="button"
                @click="habitType = 'binary'"
                class="py-2.5 px-2 rounded-xl border text-xs font-semibold transition-all touch-target sm:min-h-0 sm:min-w-0"
                :class="habitType === 'binary' ? 'bg-zinc-800 border-zinc-500 text-white shadow-sm' : 'border-surface-border text-zinc-400 hover:text-white bg-surface-dark/40'"
              >
                Sim / Não
              </button>
              <button
                type="button"
                @click="habitType = 'numeric'"
                class="py-2.5 px-2 rounded-xl border text-xs font-semibold transition-all touch-target sm:min-h-0 sm:min-w-0"
                :class="habitType === 'numeric' ? 'bg-zinc-800 border-zinc-500 text-white shadow-sm' : 'border-surface-border text-zinc-400 hover:text-white bg-surface-dark/40'"
              >
                Numérico
              </button>
              <button
                type="button"
                @click="habitType = 'timer'"
                class="py-2.5 px-2 rounded-xl border text-xs font-semibold transition-all touch-target sm:min-h-0 sm:min-w-0"
                :class="habitType === 'timer' ? 'bg-zinc-800 border-zinc-500 text-white shadow-sm' : 'border-surface-border text-zinc-400 hover:text-white bg-surface-dark/40'"
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
                class="w-full px-3.5 py-2.5 rounded-xl bg-surface-dark border border-surface-border focus:border-brand-500 text-sm text-white font-mono outline-none"
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
            <div class="flex items-center gap-2.5">
              <button
                v-for="color in presetColors"
                :key="color"
                type="button"
                @click="colorHex = color"
                class="w-8 h-8 rounded-full border-2 transition-transform"
                :class="colorHex === color ? 'border-white scale-110 shadow-md' : 'border-transparent hover:scale-105'"
                :style="{ backgroundColor: color }"
              />
            </div>
          </div>

          <!-- Modal Actions -->
          <div class="flex items-center justify-end gap-2.5 sm:gap-3 pt-3 border-t border-surface-border/60">
            <button
              type="button"
              @click="emit('close')"
              class="px-4 py-2.5 rounded-xl border border-surface-border text-sm font-medium text-zinc-400 hover:text-white transition-colors touch-target sm:min-h-0 sm:min-w-0"
            >
              Cancelar
            </button>
            <button
              type="submit"
              :disabled="loading || !name.trim()"
              class="px-5 py-2.5 rounded-xl bg-zinc-100 hover:bg-white disabled:opacity-50 text-sm font-semibold text-zinc-950 shadow-sm transition-all touch-target sm:min-h-0 sm:min-w-0"
            >
              Criar Hábito
            </button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
