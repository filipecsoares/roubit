import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import OkrsView from '../OkrsView.vue'
import { useOkrStore } from '@/stores/okr'
import { useHabitStore } from '@/stores/habit'
import type { ObjectiveWithKeyResults } from '@/types'

describe('OkrsView.vue Integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    const okrStore = useOkrStore()
    const habitStore = useHabitStore()
    vi.spyOn(okrStore, 'fetchOkrs').mockImplementation(async () => {})
    vi.spyOn(habitStore, 'fetchHabits').mockImplementation(async () => {})
    vi.clearAllMocks()
  })

  const mockObjectives: ObjectiveWithKeyResults[] = [
    {
      objective: {
        id: 'obj-1',
        user_id: 'user-1',
        title: 'Lançar MVP do Roubit com Alta Qualidade',
        description: 'Entrega de todas as 6 fases',
        quarter: '2026-Q4',
        progress_percentage: 80,
        status: 'active',
        created_at: '2026-10-30T00:00:00Z',
        updated_at: '2026-10-30T00:00:00Z',
      },
      key_results: [
        {
          id: 'kr-1',
          objective_id: 'obj-1',
          title: 'Passar 100% dos testes E2E e unitários',
          initial_value: 0,
          current_value: 100,
          target_value: 100,
          unit: '%',
          weight: 1,
          progress_percentage: 100,
          linked_habit_id: 'habit-1',
          created_at: '2026-10-30T00:00:00Z',
          updated_at: '2026-10-30T00:00:00Z',
        },
        {
          id: 'kr-2',
          objective_id: 'obj-1',
          title: 'Manter latência P99 abaixo de 50ms',
          initial_value: 100,
          current_value: 60,
          target_value: 50,
          unit: 'ms',
          weight: 1,
          progress_percentage: 60,
          linked_habit_id: null,
          created_at: '2026-10-30T00:00:00Z',
          updated_at: '2026-10-30T00:00:00Z',
        },
      ],
    },
  ]

  it('mounts, fetches OKRs for active quarter and habits, and renders stats', async () => {
    const okrStore = useOkrStore()
    okrStore.objectives = mockObjectives
    const fetchOkrsSpy = vi.spyOn(okrStore, 'fetchOkrs')

    const wrapper = mount(OkrsView)
    await flushPromises()

    expect(fetchOkrsSpy).toHaveBeenCalled()

    // Stats: 1 objective, average progress 80%, 2 KRs, 1 KR with habit
    expect(wrapper.text()).toContain('Objetivos')
    expect(wrapper.text()).toContain('80%')
    expect(wrapper.text()).toContain('Resultados-Chave')
    expect(wrapper.text()).toContain('1 / 2')
    expect(wrapper.text()).toContain('Lançar MVP do Roubit com Alta Qualidade')
  })

  it('switches quarter when quarter button is clicked', async () => {
    const okrStore = useOkrStore()
    const fetchOkrsSpy = vi.spyOn(okrStore, 'fetchOkrs').mockResolvedValue(undefined)

    const wrapper = mount(OkrsView)
    await flushPromises()

    const q3Btn = wrapper.findAll('button').find((b) => b.text() === '2026-Q3')
    expect(q3Btn).toBeDefined()

    await q3Btn!.trigger('click')
    await flushPromises()

    expect(fetchOkrsSpy).toHaveBeenCalledWith('2026-Q3')
  })

  it('opens OkrManageModal on "Novo Objetivo" button click and creates objective', async () => {
    const okrStore = useOkrStore()
    const createSpy = vi.spyOn(okrStore, 'createObjective').mockResolvedValue({} as any)

    const wrapper = mount(OkrsView)
    await flushPromises()

    const newBtn = wrapper.findAll('button').find((b) => b.text().includes('Novo Objetivo'))
    expect(newBtn).toBeDefined()
    await newBtn!.trigger('click')

    const modal = wrapper.findComponent({ name: 'OkrManageModal' })
    expect(modal.exists()).toBe(true)
    expect(modal.props('isOpen')).toBe(true)

    const payload = {
      title: 'Aumentar retenção de usuários',
      quarter: '2026-Q4',
      key_results: [
        {
          title: '30 dias de uso ativo contínuo',
          initial_value: 0,
          target_value: 30,
          unit: 'dias',
          weight: 1,
        },
      ],
    }

    await modal.vm.$emit('save', payload)
    await flushPromises()

    expect(createSpy).toHaveBeenCalledWith(payload)
  })

  it('opens LinkHabitModal and links habit to key result', async () => {
    const okrStore = useOkrStore()
    const habitStore = useHabitStore()
    okrStore.objectives = JSON.parse(JSON.stringify(mockObjectives))
    habitStore.habits = [
      {
        habit: {
          id: 'habit-2',
          user_id: 'user-1',
          name: 'Escrever Testes Automatizados',
          habit_type: 'numeric',
          target_value: 5,
          unit: 'testes',
          frequency_days: 1,
          color_hex: '#10b981',
          is_archived: false,
          created_at: '2026-10-30T00:00:00Z',
          updated_at: '2026-10-30T00:00:00Z',
        },
        streak: { habit_id: 'habit-2', current_streak: 2, longest_streak: 5, is_active_today: false },
        today_value: 0,
      },
    ]

    const linkSpy = vi.spyOn(okrStore, 'linkHabitToKeyResult').mockResolvedValue({} as any)

    const wrapper = mount(OkrsView)
    await flushPromises()

    // Trigger link-habit from ObjectiveCard
    const objCard = wrapper.findComponent({ name: 'ObjectiveCard' })
    expect(objCard.exists()).toBe(true)

    await objCard.vm.$emit('link-habit', 'kr-2')
    await flushPromises()

    const linkModal = wrapper.findComponent({ name: 'LinkHabitModal' })
    expect(linkModal.exists()).toBe(true)
    expect(linkModal.props('isOpen')).toBe(true)
    expect(linkModal.props('krTitle')).toBe('Manter latência P99 abaixo de 50ms')

    // Confirm linking
    await linkModal.vm.$emit('link', 'habit-2')
    await flushPromises()

    expect(linkSpy).toHaveBeenCalledWith('obj-1', 'kr-2', 'habit-2')
  })

  it('deletes objective after user confirmation', async () => {
    const okrStore = useOkrStore()
    okrStore.objectives = JSON.parse(JSON.stringify(mockObjectives))
    const deleteSpy = vi.spyOn(okrStore, 'deleteObjective').mockResolvedValue(undefined)
    vi.spyOn(window, 'confirm').mockReturnValue(true)

    const wrapper = mount(OkrsView)
    await flushPromises()

    const objCard = wrapper.findComponent({ name: 'ObjectiveCard' })
    await objCard.vm.$emit('delete', 'obj-1')
    await flushPromises()

    expect(window.confirm).toHaveBeenCalledWith(
      'Tem certeza que deseja excluir "Lançar MVP do Roubit com Alta Qualidade" e todos os seus Resultados-Chave?'
    )
    expect(deleteSpy).toHaveBeenCalledWith('obj-1')
  })
})
