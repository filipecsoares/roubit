import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import HabitsView from '../HabitsView.vue'
import { useHabitStore } from '@/stores/habit'
import type { HabitWithStreakResponse } from '@/types'

describe('HabitsView.vue Integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    const habitStore = useHabitStore()
    vi.spyOn(habitStore, 'fetchHabits').mockImplementation(async () => {})
    vi.spyOn(habitStore, 'fetchAnalytics').mockImplementation(async () => {})
    vi.clearAllMocks()
  })

  const mockHabits: HabitWithStreakResponse[] = [
    {
      habit: {
        id: 'habit-1',
        user_id: 'user-1',
        name: 'Meditação Matinal',
        description: '15 minutos de mindfulness',
        habit_type: 'binary',
        target_value: 1,
        frequency_days: 1,
        color_hex: '#10b981',
        is_archived: false,
        created_at: '2026-10-30T00:00:00Z',
        updated_at: '2026-10-30T00:00:00Z',
      },
      streak: {
        habit_id: 'habit-1',
        current_streak: 7,
        longest_streak: 14,
        is_active_today: true,
      },
      today_value: 1,
    },
    {
      habit: {
        id: 'habit-2',
        user_id: 'user-1',
        name: 'Exercício Físico',
        description: 'Musculação ou corrida',
        habit_type: 'numeric',
        target_value: 45,
        unit: 'min',
        frequency_days: 1,
        color_hex: '#f59e0b',
        is_archived: false,
        created_at: '2026-10-30T00:00:00Z',
        updated_at: '2026-10-30T00:00:00Z',
      },
      streak: {
        habit_id: 'habit-2',
        current_streak: 3,
        longest_streak: 21,
        is_active_today: false,
      },
      today_value: 0,
    },
  ]

  it('mounts, fetches habits and analytics, and renders calculated stats', async () => {
    const habitStore = useHabitStore()
    const fetchHabitsSpy = vi.spyOn(habitStore, 'fetchHabits').mockImplementation(async () => {
      habitStore.habits = mockHabits
    })
    const fetchAnalyticsSpy = vi.spyOn(habitStore, 'fetchAnalytics').mockResolvedValue(undefined)

    const wrapper = mount(HabitsView)
    await flushPromises()

    expect(fetchHabitsSpy).toHaveBeenCalled()
    expect(fetchAnalyticsSpy).toHaveBeenCalled()

    // Active habits: 2
    expect(wrapper.text()).toContain('Hábitos Ativos')
    expect(wrapper.text()).toContain('2')

    // Max current streak: 7 dias
    expect(wrapper.text()).toContain('7 dias')

    // Max historical streak: 21 dias
    expect(wrapper.text()).toContain('21 dias')

    // Completed today: 1 / 2
    expect(wrapper.text()).toContain('1 / 2')
  })

  it('passes aggregated heatmap data to HeatmapCalendar', async () => {
    const habitStore = useHabitStore()
    habitStore.habits = mockHabits
    habitStore.analytics = {
      aggregated_activity_map: {
        '2026-10-30': 3,
        '2026-10-29': 2,
      },
    } as any

    const wrapper = mount(HabitsView)
    await flushPromises()

    const heatmap = wrapper.findComponent({ name: 'HeatmapCalendar' })
    expect(heatmap.exists()).toBe(true)
    expect(heatmap.props('activityMap')).toEqual({
      '2026-10-30': 3,
      '2026-10-29': 2,
    })
  })

  it('opens HabitManageModal on "Novo Hábito" button click and creates habit', async () => {
    const habitStore = useHabitStore()
    const createSpy = vi.spyOn(habitStore, 'createHabit').mockResolvedValue({
      id: 'habit-3',
      user_id: 'user-1',
      name: 'Leitura',
      habit_type: 'numeric',
      target_value: 20,
      unit: 'páginas',
      frequency_days: 1,
      color_hex: '#6366f1',
      is_archived: false,
      created_at: '2026-10-30T00:00:00Z',
      updated_at: '2026-10-30T00:00:00Z',
    })

    const wrapper = mount(HabitsView)
    await flushPromises()

    const openBtn = wrapper.findAll('button').find((b) => b.text().includes('Novo Hábito'))
    expect(openBtn).toBeDefined()
    await openBtn!.trigger('click')

    const modal = wrapper.findComponent({ name: 'HabitManageModal' })
    expect(modal.exists()).toBe(true)
    expect(modal.props('isOpen')).toBe(true)

    // Emit save from modal
    await modal.vm.$emit('save', {
      name: 'Leitura',
      habit_type: 'numeric',
      target_value: 20,
      unit: 'páginas',
    })
    await flushPromises()

    expect(createSpy).toHaveBeenCalledWith({
      name: 'Leitura',
      habit_type: 'numeric',
      target_value: 20,
      unit: 'páginas',
    })
  })

  it('toggles check-in state when clicking checkin button', async () => {
    const habitStore = useHabitStore()
    habitStore.habits = JSON.parse(JSON.stringify(mockHabits))
    const checkinSpy = vi.spyOn(habitStore, 'checkin').mockResolvedValue({} as any)

    const wrapper = mount(HabitsView)
    await flushPromises()

    // Check-in on habit-2 (currently not active)
    const checkinButtons = wrapper.findAll('button').filter((b) => b.text().includes('Check-in'))
    expect(checkinButtons.length).toBeGreaterThan(0)

    await checkinButtons[0].trigger('click')
    await flushPromises()

    expect(checkinSpy).toHaveBeenCalledWith('habit-2', expect.any(String), 45, false)
  })

  it('archives habit after user confirmation', async () => {
    const habitStore = useHabitStore()
    habitStore.habits = JSON.parse(JSON.stringify(mockHabits))
    const archiveSpy = vi.spyOn(habitStore, 'archiveHabit').mockResolvedValue(undefined)
    vi.spyOn(window, 'confirm').mockReturnValue(true)

    const wrapper = mount(HabitsView)
    await flushPromises()

    const archiveBtn = wrapper.find('button[title="Arquivar hábito"]')
    expect(archiveBtn.exists()).toBe(true)

    await archiveBtn.trigger('click')
    await flushPromises()

    expect(window.confirm).toHaveBeenCalledWith('Deseja arquivar o hábito "Meditação Matinal"?')
    expect(archiveSpy).toHaveBeenCalledWith('habit-1')
  })

  it('deletes habit after user confirmation', async () => {
    const habitStore = useHabitStore()
    habitStore.habits = JSON.parse(JSON.stringify(mockHabits))
    const deleteSpy = vi.spyOn(habitStore, 'deleteHabit').mockResolvedValue(undefined)
    vi.spyOn(window, 'confirm').mockReturnValue(true)

    const wrapper = mount(HabitsView)
    await flushPromises()

    const deleteBtn = wrapper.find('button[title="Excluir hábito"]')
    expect(deleteBtn.exists()).toBe(true)

    await deleteBtn.trigger('click')
    await flushPromises()

    expect(window.confirm).toHaveBeenCalledWith('Tem certeza que deseja excluir o hábito "Meditação Matinal"?')
    expect(deleteSpy).toHaveBeenCalledWith('habit-1')
  })
})
