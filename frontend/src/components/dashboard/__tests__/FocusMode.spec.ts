import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import FocusModeModal from '../FocusModeModal.vue'
import { useFocusStore } from '@/stores/focus'

describe('FocusModeModal.vue', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  const mountModal = () =>
    mount(FocusModeModal, {
      global: {
        stubs: {
          Teleport: true,
        },
      },
    })

  it('renders modal when isModalOpen is true', () => {
    const focusStore = useFocusStore()
    focusStore.openFocusModal({ id: 'block-1', name: 'Deep Work Session' })

    const wrapper = mountModal()

    expect(wrapper.text()).toContain('Modo Foco')
    expect(wrapper.text()).toContain('Deep Work Session')
    expect(wrapper.text()).toContain('25:00')
    expect(wrapper.text()).toContain('Pausar')
  })

  it('triggers togglePlayPause on button click and space keydown', async () => {
    const focusStore = useFocusStore()
    focusStore.openFocusModal({ id: 'block-1', name: 'Coding' })
    const toggleSpy = vi.spyOn(focusStore, 'togglePlayPause')

    const wrapper = mountModal()

    const playPauseBtn = wrapper.find('button[title*="Pausar"]')
    expect(playPauseBtn.exists()).toBe(true)
    await playPauseBtn.trigger('click')
    expect(toggleSpy).toHaveBeenCalledTimes(1)

    // Keyboard Space trigger
    window.dispatchEvent(new KeyboardEvent('keydown', { code: 'Space' }))
    expect(toggleSpy).toHaveBeenCalledTimes(2)
  })

  it('triggers reset and skip controls', async () => {
    const focusStore = useFocusStore()
    focusStore.openFocusModal({ id: 'block-1', name: 'Task' })
    const resetSpy = vi.spyOn(focusStore, 'resetTimer')
    const skipSpy = vi.spyOn(focusStore, 'skipToBreak')

    const wrapper = mountModal()

    const resetBtn = wrapper.find('button[title="Reiniciar sessão"]')
    await resetBtn.trigger('click')
    expect(resetSpy).toHaveBeenCalledTimes(1)

    const skipBtn = wrapper.find('button[title="Pular para Pausa"]')
    await skipBtn.trigger('click')
    expect(skipSpy).toHaveBeenCalledTimes(1)
  })

  it('emits completeBlock when associated block completion is triggered', async () => {
    const focusStore = useFocusStore()
    focusStore.openFocusModal({ id: 'block-task-99', name: 'Refactoring' })

    const wrapper = mountModal()

    const completeBlockBtn = wrapper.find('button[title="Marcar bloco da rotina como concluído"]')
    expect(completeBlockBtn.exists()).toBe(true)
    await completeBlockBtn.trigger('click')

    expect(wrapper.emitted('completeBlock')).toBeTruthy()
    expect(wrapper.emitted('completeBlock')![0]).toEqual(['block-task-99'])
  })

  it('closes modal on close button click and escape key', async () => {
    const focusStore = useFocusStore()
    focusStore.openFocusModal({ id: 'block-1', name: 'Task' })
    const closeSpy = vi.spyOn(focusStore, 'closeFocusModal')

    const wrapper = mountModal()

    const closeBtn = wrapper.find('button[title*="Minimizar"]')
    await closeBtn.trigger('click')
    expect(closeSpy).toHaveBeenCalledTimes(1)

    // Reopen modal to verify Escape key shortcut
    focusStore.isModalOpen = true
    window.dispatchEvent(new KeyboardEvent('keydown', { code: 'Escape' }))
    expect(closeSpy).toHaveBeenCalledTimes(2)
  })

  it('renders and supports standalone focus mode without routines', async () => {
    const focusStore = useFocusStore()
    focusStore.openFocusModal() // Standalone, no block

    const wrapper = mountModal()

    // Standalone custom input should be present
    const taskInput = wrapper.find('input[placeholder*="Foco Livre"]')
    expect(taskInput.exists()).toBe(true)
    expect((taskInput.element as HTMLInputElement).value).toBe('Foco Livre')

    // Preset pills should be rendered
    const deepFocusBtn = wrapper.find('button[title*="50 minutos"]')
    expect(deepFocusBtn.exists()).toBe(true)
    await deepFocusBtn.trigger('click')

    expect(focusStore.durationSeconds).toBe(3000)
    expect(wrapper.text()).toContain('50:00')

    // Preset short break pill
    const shortBreakBtn = wrapper.find('button[title*="Pausa curta"]')
    expect(shortBreakBtn.exists()).toBe(true)
    await shortBreakBtn.trigger('click')

    expect(focusStore.mode).toBe('short_break')
    expect(focusStore.durationSeconds).toBe(300)
    expect(wrapper.text()).toContain('05:00')
  })
})
