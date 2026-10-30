import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'dashboard',
    component: () => import('@/views/DashboardView.vue'),
    meta: {
      title: 'Dashboard — Roubit',
      requiresAuth: true,
    },
  },
  {
    path: '/habits',
    name: 'habits',
    component: () => import('@/views/HabitsView.vue'),
    meta: {
      title: 'Hábitos — Roubit',
      requiresAuth: true,
    },
  },
  {
    path: '/okrs',
    name: 'okrs',
    component: () => import('@/views/OkrsView.vue'),
    meta: {
      title: 'OKRs & Objetivos — Roubit',
      requiresAuth: true,
    },
  },
  {
    path: '/journal',
    name: 'journal',
    component: () => import('@/views/JournalView.vue'),
    meta: {
      title: 'Diário Criptografado — Roubit',
      requiresAuth: true,
    },
  },
  {
    path: '/login',
    name: 'login',
    component: () => import('@/views/LoginView.vue'),
    meta: {
      title: 'Entrar — Roubit',
      public: true,
    },
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/',
  },
]

export const router = createRouter({
  history: createWebHistory(),
  routes,
})

// Navigation Guard protecting authenticated views
router.beforeEach((to, _from, next) => {
  if (to.meta.title) {
    document.title = to.meta.title as string
  }

  const token = localStorage.getItem('roubit_token')
  const requiresAuth = to.meta.requiresAuth === true

  if (requiresAuth && !token) {
    next({ name: 'login', query: { redirect: to.fullPath } })
  } else if (to.name === 'login' && token) {
    next({ name: 'dashboard' })
  } else {
    next()
  }
})

export default router
