<script setup lang="ts">
import { ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { Lock, Mail, ArrowRight, Loader2, ShieldCheck } from 'lucide-vue-next'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()

const email = ref('')
const password = ref('')
const isRegister = ref(false)
const errorMessage = ref('')

async function handleSubmit() {
  errorMessage.value = ''
  if (!email.value || !password.value) {
    errorMessage.value = 'Preencha todos os campos obrigatórios'
    return
  }

  try {
    if (isRegister.value) {
      await authStore.register(email.value, password.value)
    } else {
      await authStore.login(email.value, password.value)
    }
    const redirectQuery = route.query.redirect
    const redirect =
      typeof redirectQuery === 'string' &&
      redirectQuery.startsWith('/') &&
      !redirectQuery.startsWith('//')
        ? redirectQuery
        : '/'
    router.push(redirect)
  } catch (_err: any) {
    errorMessage.value = authStore.error || 'Erro na autenticação'
  }
}
</script>

<template>
  <div class="min-h-[82vh] flex items-center justify-center p-3 sm:p-4">
    <div class="w-full max-w-md glass-panel p-6 sm:p-8 rounded-2xl border border-surface-border space-y-6 glow-card">
      <div class="text-center space-y-2">
        <div class="inline-flex items-center justify-center w-12 h-12 rounded-2xl bg-zinc-800 border border-zinc-700 text-zinc-100 shadow-sm mb-2">
          <ShieldCheck class="w-6 h-6" />
        </div>
        <h1 class="text-2xl sm:text-3xl font-extrabold text-white font-outfit">
          {{ isRegister ? 'Criar Conta' : 'Acessar o Roubit' }}
        </h1>
        <p class="text-xs text-zinc-400">
          Autenticação criptografada com Argon2id e tokens seguros
        </p>
      </div>

      <form @submit.prevent="handleSubmit" class="space-y-4">
        <div>
          <label for="login-email" class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1.5">E-mail</label>
          <div class="relative">
            <Mail class="w-4 h-4 text-zinc-500 absolute left-3.5 top-1/2 -translate-y-1/2 pointer-events-none" />
            <input
              id="login-email"
              v-model="email"
              type="email"
              placeholder="seu@email.com"
              required
              class="w-full pl-10 pr-4 py-2.5 sm:py-3 rounded-xl bg-surface-card border border-surface-border text-white text-sm focus:outline-none focus:border-zinc-500 transition-colors"
            />
          </div>
        </div>

        <div>
          <label for="login-password" class="block text-xs font-semibold text-zinc-300 uppercase tracking-wider mb-1.5">Senha</label>
          <div class="relative">
            <Lock class="w-4 h-4 text-zinc-500 absolute left-3.5 top-1/2 -translate-y-1/2 pointer-events-none" />
            <input
              id="login-password"
              v-model="password"
              type="password"
              placeholder="••••••••••••"
              required
              class="w-full pl-10 pr-4 py-2.5 sm:py-3 rounded-xl bg-surface-card border border-surface-border text-white text-sm focus:outline-none focus:border-zinc-500 transition-colors"
            />
          </div>
        </div>

        <div v-if="errorMessage" class="p-3 rounded-xl bg-rose-950/60 border border-rose-500/40 text-rose-300 text-xs">
          {{ errorMessage }}
        </div>

        <button
          type="submit"
          :disabled="authStore.loading"
          class="w-full flex items-center justify-center gap-2 py-3 px-4 rounded-xl text-sm font-semibold bg-zinc-100 hover:bg-white disabled:opacity-60 text-zinc-950 shadow-sm transition-all touch-target"
        >
          <Loader2 v-if="authStore.loading" class="w-4 h-4 animate-spin text-zinc-950" />
          <template v-else>
            <span>{{ isRegister ? 'Cadastrar' : 'Entrar' }}</span>
            <ArrowRight class="w-4 h-4" />
          </template>
        </button>
      </form>

      <div class="text-center pt-3 border-t border-surface-border/60">
        <button
          type="button"
          @click="isRegister = !isRegister"
          class="text-xs text-brand-400 hover:text-brand-300 transition-colors p-2"
        >
          {{ isRegister ? 'Já possui conta? Entrar' : 'Não tem conta? Cadastre-se' }}
        </button>
      </div>
    </div>
  </div>
</template>
