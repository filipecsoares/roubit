import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import LoginView from '../LoginView.vue'
import { useAuthStore } from '@/stores/auth'

const pushMock = vi.fn()
let currentRouteQuery: Record<string, string> = {}

vi.mock('vue-router', () => ({
  useRouter: () => ({
    push: pushMock,
  }),
  useRoute: () => ({
    query: currentRouteQuery,
  }),
}))

describe('LoginView.vue Integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    pushMock.mockClear()
    currentRouteQuery = {}
    vi.clearAllMocks()
  })

  it('renders login mode by default and toggles to register mode', async () => {
    const wrapper = mount(LoginView)

    expect(wrapper.find('h1').text()).toBe('Acessar o Roubit')
    expect(wrapper.find('button[type="submit"]').text()).toContain('Entrar')

    const toggleButton = wrapper.findAll('button').find((b) => b.text().includes('Cadastre-se'))
    expect(toggleButton).toBeDefined()
    await toggleButton!.trigger('click')

    expect(wrapper.find('h1').text()).toBe('Criar Conta')
    expect(wrapper.find('button[type="submit"]').text()).toContain('Cadastrar')

    const toggleBack = wrapper.findAll('button').find((b) => b.text().includes('Já possui conta?'))
    expect(toggleBack).toBeDefined()
    await toggleBack!.trigger('click')

    expect(wrapper.find('h1').text()).toBe('Acessar o Roubit')
  })

  it('validates required fields before submitting', async () => {
    const wrapper = mount(LoginView)
    const authStore = useAuthStore()
    const loginSpy = vi.spyOn(authStore, 'login')

    await wrapper.find('form').trigger('submit.prevent')
    await flushPromises()

    expect(loginSpy).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('Preencha todos os campos obrigatórios')
  })

  it('calls authStore.login and redirects to "/" by default', async () => {
    const wrapper = mount(LoginView)
    const authStore = useAuthStore()
    const loginSpy = vi.spyOn(authStore, 'login').mockResolvedValue(undefined)

    const inputs = wrapper.findAll('input')
    await inputs[0].setValue('user@roubit.app')
    await inputs[1].setValue('SecurePassword123!')

    await wrapper.find('form').trigger('submit.prevent')
    await flushPromises()

    expect(loginSpy).toHaveBeenCalledWith('user@roubit.app', 'SecurePassword123!')
    expect(pushMock).toHaveBeenCalledWith('/')
  })

  it('calls authStore.register when in register mode', async () => {
    const wrapper = mount(LoginView)
    const authStore = useAuthStore()
    const registerSpy = vi.spyOn(authStore, 'register').mockResolvedValue(undefined)

    // Switch to register mode
    const toggleButton = wrapper.findAll('button').find((b) => b.text().includes('Cadastre-se'))
    await toggleButton!.trigger('click')

    const inputs = wrapper.findAll('input')
    await inputs[0].setValue('newuser@roubit.app')
    await inputs[1].setValue('SuperSecret123!')

    await wrapper.find('form').trigger('submit.prevent')
    await flushPromises()

    expect(registerSpy).toHaveBeenCalledWith('newuser@roubit.app', 'SuperSecret123!')
    expect(pushMock).toHaveBeenCalledWith('/')
  })

  it('sanitizes redirect parameter to prevent open redirect vulnerabilities', async () => {
    // Malicious open redirect
    currentRouteQuery = { redirect: '//evil.com/hack' }
    let wrapper = mount(LoginView)
    const authStore = useAuthStore()
    vi.spyOn(authStore, 'login').mockResolvedValue(undefined)

    const inputs = wrapper.findAll('input')
    await inputs[0].setValue('user@roubit.app')
    await inputs[1].setValue('pass')
    await wrapper.find('form').trigger('submit.prevent')
    await flushPromises()

    // Should fallback to '/'
    expect(pushMock).toHaveBeenCalledWith('/')

    // Valid internal redirect
    pushMock.mockClear()
    currentRouteQuery = { redirect: '/journal' }
    wrapper = mount(LoginView)
    const inputs2 = wrapper.findAll('input')
    await inputs2[0].setValue('user@roubit.app')
    await inputs2[1].setValue('pass')
    await wrapper.find('form').trigger('submit.prevent')
    await flushPromises()

    expect(pushMock).toHaveBeenCalledWith('/journal')
  })

  it('renders error banner when authentication fails', async () => {
    const wrapper = mount(LoginView)
    const authStore = useAuthStore()
    authStore.error = 'Credenciais inválidas'
    vi.spyOn(authStore, 'login').mockRejectedValue(new Error('Unauthorized'))

    const inputs = wrapper.findAll('input')
    await inputs[0].setValue('user@roubit.app')
    await inputs[1].setValue('wrongpassword')

    await wrapper.find('form').trigger('submit.prevent')
    await flushPromises()

    expect(wrapper.text()).toContain('Credenciais inválidas')
  })
})
