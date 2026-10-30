<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    modelValue?: number
    disabled?: boolean
  }>(),
  {
    modelValue: 3,
    disabled: false,
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: number): void
}>()

const moods = [
  { score: 1, emoji: '😫', label: 'Muito Difícil' },
  { score: 2, emoji: '🙁', label: 'Desafiador' },
  { score: 3, emoji: '😐', label: 'Neutro' },
  { score: 4, emoji: '🙂', label: 'Produtivo' },
  { score: 5, emoji: '🚀', label: 'Excelente' },
]
</script>

<template>
  <div class="space-y-1.5">
    <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider">
      Como foi o seu dia?
    </label>
    <div class="flex items-center gap-2">
      <button
        v-for="m in moods"
        :key="m.score"
        type="button"
        :disabled="disabled"
        @click="emit('update:modelValue', m.score)"
        class="flex-1 py-2 px-1.5 rounded-xl border flex flex-col items-center gap-1 transition-all"
        :class="
          modelValue === m.score
            ? 'bg-brand-500/20 border-brand-500 shadow-sm shadow-brand-500/20 scale-105 text-white'
            : 'border-surface-border bg-surface-dark/50 text-slate-400 hover:border-slate-700 hover:text-slate-200'
        "
        :title="m.label"
      >
        <span class="text-xl leading-none">{{ m.emoji }}</span>
        <span class="text-[10px] font-medium truncate hidden sm:block">{{ m.label }}</span>
      </button>
    </div>
  </div>
</template>
