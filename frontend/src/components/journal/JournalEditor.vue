<script setup lang="ts">
import { computed } from 'vue'
import { Lock, Sparkles } from 'lucide-vue-next'
import MoodSelector from './MoodSelector.vue'
import TagInput from './TagInput.vue'

const props = withDefaults(
  defineProps<{
    plaintextReflection: string
    moodScore?: number
    tags: string[]
    isSaving?: boolean
    entryDate: string
    isReadOnly?: boolean
  }>(),
  {
    moodScore: 3,
    isSaving: false,
    isReadOnly: false,
  }
)

const emit = defineEmits<{
  (e: 'update:plaintextReflection', value: string): void
  (e: 'update:plaintext-reflection', value: string): void
  (e: 'update:moodScore', value: number): void
  (e: 'update:mood-score', value: number): void
  (e: 'update:tags', value: string[]): void
  (e: 'save'): void
}>()

const wordCount = computed(() => {
  const trimmed = props.plaintextReflection.trim()
  return trimmed ? trimmed.split(/\s+/).length : 0
})

const charCount = computed(() => props.plaintextReflection.length)
</script>

<template>
  <div class="space-y-5 sm:space-y-6">
    <!-- Mood and Tags Controls -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 p-4 sm:p-5 rounded-2xl bg-surface-card border border-surface-border glow-card">
      <MoodSelector
        :model-value="moodScore"
        :disabled="isReadOnly || isSaving"
        @update:model-value="(val) => emit('update:moodScore', val)"
      />

      <TagInput
        :model-value="tags"
        :disabled="isReadOnly || isSaving"
        @update:model-value="(val) => emit('update:tags', val)"
      />
    </div>

    <!-- Main Markdown Writing Area -->
    <div class="rounded-2xl bg-surface-card border border-surface-border overflow-hidden shadow-sm focus-within:border-brand-500/80 transition-all glow-card">
      <!-- Toolbar / Meta info -->
      <div class="flex items-center justify-between px-4 sm:px-5 py-3 border-b border-surface-border/60 bg-surface-dark/50 text-xs text-slate-400">
        <div class="flex items-center gap-2">
          <Sparkles class="w-3.5 h-3.5 text-brand-400" />
          <span class="font-medium">Reflexão Diária & Retrospectiva</span>
        </div>
        <div class="flex items-center gap-2.5 font-mono text-[11px]">
          <span>{{ wordCount }} palavras</span>
          <span>•</span>
          <span>{{ charCount }} caracteres</span>
        </div>
      </div>

      <!-- Textarea -->
      <div class="p-4 sm:p-5">
        <textarea
          :value="plaintextReflection"
          :disabled="isReadOnly || isSaving"
          @input="emit('update:plaintextReflection', ($event.target as HTMLTextAreaElement).value)"
          rows="12"
          placeholder="Escreva suas reflexões, intenções do dia, aprendizados e retrospectiva noturna... Seus pensamentos serão cifrados com AES-256-GCM antes de sair do navegador."
          class="w-full bg-transparent text-slate-100 placeholder-slate-500 text-sm leading-relaxed outline-none resize-y font-sans min-h-[180px] sm:min-h-[240px]"
        ></textarea>
      </div>

      <!-- Footer Action -->
      <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3 px-4 sm:px-5 py-3.5 border-t border-surface-border/60 bg-surface-dark/40">
        <div class="flex items-center gap-2 text-xs text-slate-400">
          <Lock class="w-3.5 h-3.5 text-emerald-400 shrink-0" />
          <span>Criptografia AES-256 ponta a ponta ativa</span>
        </div>

        <button
          type="button"
          :disabled="isSaving || isReadOnly || !plaintextReflection.trim()"
          @click="emit('save')"
          class="inline-flex items-center justify-center gap-2 px-5 py-2.5 rounded-xl bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-sm font-semibold text-white shadow-sm transition-all active:scale-95 touch-target sm:min-h-0 sm:min-w-0"
        >
          <Lock class="w-4 h-4" />
          <span>{{ isSaving ? 'Cifrando & Salvando...' : 'Salvar & Criptografar' }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
