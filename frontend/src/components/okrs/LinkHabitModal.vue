<script setup lang="ts">
import { ref } from 'vue'
import { X, Link2 } from 'lucide-vue-next'
import { useModal } from '@/composables/useModal'

const props = defineProps<{
  isOpen: boolean
  krTitle: string
  habits: Array<{ id: string; name: string }>
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'link', habitId: string): void
}>()

useModal(() => props.isOpen, () => emit('close'))

const selectedHabitId = ref('')

function handleLink() {
  if (!selectedHabitId.value) return
  emit('link', selectedHabitId.value)
  selectedHabitId.value = ''
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="isOpen"
      class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-black/75 backdrop-blur-sm animate-fade-in"
      @click="emit('close')"
    >
      <div
        class="w-full max-w-md bg-surface-card border border-surface-border rounded-2xl shadow-2xl p-4 sm:p-6 space-y-5 max-h-[92vh] overflow-y-auto"
        @click.stop
      >
        <!-- Modal Header -->
        <div class="flex items-center justify-between border-b border-surface-border/60 pb-4">
          <div class="flex items-center gap-2.5">
            <div class="p-2 sm:p-2.5 rounded-xl bg-zinc-800 text-zinc-100 border border-zinc-700">
              <Link2 class="w-4 h-4 sm:w-5 sm:h-5" />
            </div>
            <h3 class="text-base sm:text-lg font-bold text-white font-outfit">Vincular Hábito ao KR</h3>
          </div>
          <button
            type="button"
            @click="emit('close')"
            class="p-2 rounded-xl text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors touch-target sm:min-h-0 sm:min-w-0"
          >
            <X class="w-5 h-5" />
          </button>
        </div>

        <p class="text-xs text-slate-400 leading-relaxed">
          Vincule o progresso diário de um hábito para atualizar automaticamente o Resultado-Chave:
          <strong class="text-white block mt-1.5 p-2 rounded-xl bg-surface-dark border border-surface-border">"{{ krTitle }}"</strong>
        </p>

        <!-- Habit Select -->
        <div class="space-y-2">
          <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider">
            Selecione o Hábito
          </label>
          <select
            v-model="selectedHabitId"
            class="w-full px-3.5 py-2.5 rounded-xl bg-surface-dark border border-surface-border focus:border-brand-500 text-sm text-white outline-none"
          >
            <option value="" disabled selected>Escolha um hábito...</option>
            <option v-for="h in habits" :key="h.id" :value="h.id">
              {{ h.name }}
            </option>
          </select>
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
            type="button"
            :disabled="!selectedHabitId || loading"
            @click="handleLink"
            class="px-5 py-2.5 rounded-xl bg-zinc-100 hover:bg-white disabled:opacity-50 text-sm font-semibold text-zinc-950 shadow-sm transition-all touch-target sm:min-h-0 sm:min-w-0"
          >
            Confirmar Vínculo
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
