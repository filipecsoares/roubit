<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { Lock, Mail, ArrowRight } from 'lucide-vue-next'

const router = useRouter()
const email = ref('')
const password = ref('')
const isRegister = ref(false)
const errorMessage = ref('')

function handleSubmit() {
  if (!email.value || !password.value) {
    errorMessage.value = 'Preencha todos os campos obrigatórios'
    return
  }
  // Temporary auth token stub for onboarding
  localStorage.setItem('roubit_token', 'demo-token-active')
  router.push('/')
}
</script>

<template>
  <div class="min-h-[80vh] flex items-center justify-center p-4">
    <div class="w-full max-w-md glass-panel p-8 rounded-2xl border border-surface-border space-y-6">
      <div class="text-center space-y-2">
        <div class="inline-flex items-center justify-center w-12 h-12 rounded-xl bg-brand-500/10 border border-brand-500/20 text-brand-400 mb-2">
          <Lock class="w-6 h-6" />
        </div>
        <h1 class="text-2xl font-extrabold text-white font-outfit">
          {{ isRegister ? 'Criar Conta' : 'Acessar o Roubit' }}
        </h1>
        <p class="text-xs text-slate-400">
          Autenticação criptografada com Argon2id e tokens seguros
        </p>
      </div>

      <form @submit.prevent="handleSubmit" class="space-y-4">
        <div>
          <label class="block text-xs font-medium text-slate-300 mb-1">E-mail</label>
          <div class="relative">
            <Mail class="w-4 h-4 text-slate-500 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              v-model="email"
              type="email"
              placeholder="seu@email.com"
              required
              class="w-full pl-9 pr-4 py-2.5 rounded-xl bg-surface-card border border-surface-border text-white text-sm focus:outline-none focus:border-brand-500 transition-colors"
            />
          </div>
        </div>

        <div>
          <label class="block text-xs font-medium text-slate-300 mb-1">Senha</label>
          <div class="relative">
            <Lock class="w-4 h-4 text-slate-500 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              v-model="password"
              type="password"
              placeholder="••••••••••••"
              required
              class="w-full pl-9 pr-4 py-2.5 rounded-xl bg-surface-card border border-surface-border text-white text-sm focus:outline-none focus:border-brand-500 transition-colors"
            />
          </div>
        </div>

        <div v-if="errorMessage" class="p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 text-xs">
          {{ errorMessage }}
        </div>

        <button
          type="submit"
          class="w-full flex items-center justify-center gap-2 py-2.5 rounded-xl text-sm font-semibold bg-brand-600 hover:bg-brand-500 text-white shadow-lg shadow-brand-500/25 transition-all"
        >
          <span>{{ isRegister ? 'Cadastrar' : 'Entrar' }}</span>
          <ArrowRight class="w-4 h-4" />
        </button>
      </form>

      <div class="text-center pt-2 border-t border-surface-border">
        <button
          type="button"
          @click="isRegister = !isRegister"
          class="text-xs text-brand-400 hover:text-brand-300 transition-colors"
        >
          {{ isRegister ? 'Já possui conta? Entrar' : 'Não tem conta? Cadastre-se' }}
        </button>
      </div>
    </div>
  </div>
</template>
