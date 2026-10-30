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
  (e: 'update:moodScore', value: number): void
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
  <div class="space-y-6">
    <!-- Mood and Tags Controls -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 p-5 rounded-2xl bg-surface-card border border-surface-border">
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
    <div class="rounded-2xl bg-surface-card border border-surface-border overflow-hidden shadow-sm focus-within:border-brand-500/80 transition-all">
      <!-- Toolbar / Meta info -->
      <div class="flex items-center justify-between px-5 py-3 border-b border-surface-border/60 bg-surface-dark/40 text-xs text-slate-400">
        <div class="flex items-center gap-2">
          <Sparkles class="w-3.5 h-3.5 text-brand-400" />
          <span>Reflexão Diária & Retrospectiva</span>
        </div>
        <div class="flex items-center gap-3 font-mono text-[11px]">
          <span>{{ wordCount }} palavras</span>
          <span>•</span>
          <span>{{ charCount }} caracteres</span>
        </div>
      </div>

      <!-- Textarea -->
      <div class="p-5">
        <textarea
          :value="plaintextReflection"
          :disabled="isReadOnly || isSaving"
          @input="emit('update:plaintextReflection', ($event.target as HTMLTextAreaElement).value)"
          rows="14"
          placeholder="Escreva suas reflexões, intenções do dia, aprendizados e retrospectiva noturna... Seus pensamentos serão cifrados com AES-256-GCM antes de sair do navegador."
          class="w-full bg-transparent text-slate-100 placeholder-slate-500 text-sm leading-relaxed outline-none resize-y font-sans min-h-[220px]"
        ></textarea>
      </div>

      <!-- Footer Action -->
      <div class="flex items-center justify-between px-5 py-3.5 border-t border-surface-border/60 bg-surface-dark/30">
        <div class="flex items-center gap-2 text-xs text-slate-400">
          <Lock class="w-3.5 h-3.5 text-emerald-400" />
          <span class="hidden sm:inline">Criptografia AES-256 ponta a ponta ativa</span>
        </div>

        <button
          type="button"
          :disabled="isSaving || isReadOnly || !plaintextReflection.trim()"
          @click="emit('save')"
          class="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-sm font-semibold text-white shadow-lg shadow-brand-500/25 transition-all active:scale-95"
        >
          <Lock class="w-4 h-4" />
          <span>{{ isSaving ? 'Cifrando & Salvando...' : 'Salvar & Criptografar' }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
