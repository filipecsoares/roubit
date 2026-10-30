<script setup lang="ts">
import { ref } from 'vue'
import { Tag, X } from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    modelValue: string[]
    disabled?: boolean
  }>(),
  {
    modelValue: () => [],
    disabled: false,
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', tags: string[]): void
}>()

const newTag = ref('')

function addTag() {
  const clean = newTag.value.trim().toLowerCase().replace(/^#/, '')
  if (clean && !props.modelValue.includes(clean)) {
    emit('update:modelValue', [...props.modelValue, clean])
  }
  newTag.value = ''
}

function removeTag(tagToRemove: string) {
  emit(
    'update:modelValue',
    props.modelValue.filter((t) => t !== tagToRemove)
  )
}

function handleKeyDown(event: KeyboardEvent) {
  if (event.key === 'Enter' || event.key === ',') {
    event.preventDefault()
    addTag()
  } else if (event.key === 'Backspace' && !newTag.value && props.modelValue.length > 0) {
    removeTag(props.modelValue[props.modelValue.length - 1])
  }
}
</script>

<template>
  <div class="space-y-1.5">
    <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider">
      Tags & Tópicos
    </label>

    <div
      class="flex flex-wrap items-center gap-1.5 p-2 rounded-xl bg-surface-dark border border-surface-border focus-within:border-brand-500 transition-all min-h-[42px]"
      :class="{ 'opacity-60 cursor-not-allowed': disabled }"
    >
      <span
        v-for="tag in modelValue"
        :key="tag"
        class="inline-flex items-center gap-1 px-2 py-0.5 rounded-lg text-xs font-medium bg-brand-500/10 text-brand-300 border border-brand-500/20"
      >
        <Tag class="w-3 h-3" />
        {{ tag }}
        <button
          v-if="!disabled"
          type="button"
          @click="removeTag(tag)"
          class="text-brand-300 hover:text-white transition-colors"
        >
          <X class="w-3 h-3" />
        </button>
      </span>

      <input
        v-model="newTag"
        type="text"
        :disabled="disabled"
        placeholder="Adicionar tag (Enter)..."
        @keydown="handleKeyDown"
        @blur="addTag"
        class="flex-1 min-w-[120px] bg-transparent text-xs text-white placeholder-slate-500 outline-none px-1"
      />
    </div>
  </div>
</template>
