import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import DashboardView from '../DashboardView.vue'
import { useRoutineStore } from '@/stores/routine'
import { useHabitStore } from '@/stores/habit'
import { useFocusStore } from '@/stores/focus'
import type { Routine, TimeBlock, HabitWithStreakResponse } from '@/types'

describe('DashboardView.vue Integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.clearAllMocks()
  })

  const mockRoutine: Routine = {
    id: 'routine-1',
    user_id: 'user-1',
    name: 'Rotina Produtiva',
    description: 'Foco e consistência diária',
    is_active: true,
    created_at: '2026-10-30T00:00:00Z',
    updated_at: '2026-10-30T00:00:00Z',
  }

  const mockBlocks: TimeBlock[] = [
    {
      id: 'block-1',
      routine_id: 'routine-1',
      name: 'Deep Work Manhã',
      start_time: '08:00',
      end_time: '11:00',
      category: 'Trabalho',
      color_hex: '#6366f1',
      order_index: 0,
      created_at: '2026-10-30T00:00:00Z',
    },
    {
      id: 'block-2',
      routine_id: 'routine-1',
      name: 'Reunião de Alinhamento',
      start_time: '11:00',
      end_time: '12:00',
      category: 'Comunicação',
      color_hex: '#10b981',
      order_index: 1,
      created_at: '2026-10-30T00:00:00Z',
    },
  ]

  const mockHabits: HabitWithStreakResponse[] = [
    {
      habit: {
        id: 'habit-1',
        user_id: 'user-1',
        name: 'Beber 2L de Água',
        habit_type: 'numeric',
        target_value: 2000,
        unit: 'ml',
        frequency_days: 1,
        color_hex: '#3b82f6',
        is_archived: false,
        created_at: '2026-10-30T00:00:00Z',
        updated_at: '2026-10-30T00:00:00Z',
      },
      streak: {
        habit_id: 'habit-1',
        current_streak: 5,
        longest_streak: 10,
        is_active_today: false,
      },
      today_value: 500,
    },
  ]

  it('mounts, fetches active routine and habits on initial load', async () => {
    const routineStore = useRoutineStore()
    const habitStore = useHabitStore()

    const fetchRoutineSpy = vi.spyOn(routineStore, 'fetchActiveRoutine').mockImplementation(async () => {
      routineStore.activeRoutine = mockRoutine
      routineStore.blocks = mockBlocks
    })
    const fetchHabitsSpy = vi.spyOn(habitStore, 'fetchHabits').mockImplementation(async () => {
      habitStore.habits = mockHabits
    })

    const wrapper = mount(DashboardView)
    await flushPromises()

    expect(fetchRoutineSpy).toHaveBeenCalled()
    expect(fetchHabitsSpy).toHaveBeenCalled()
    expect(wrapper.text()).toContain('Daily Focus')
    expect(wrapper.text()).toContain('Deep Work Manhã')
    expect(wrapper.text()).toContain('Beber 2L de Água')
  })

  it('correctly tracks completed block IDs and triggers block completion', async () => {
    const routineStore = useRoutineStore()
    routineStore.activeRoutine = mockRoutine
    routineStore.blocks = mockBlocks
    routineStore.completedBlockIds = ['block-1']
    const completeBlockSpy = vi.spyOn(routineStore, 'completeBlock').mockResolvedValue(undefined)

    const wrapper = mount(DashboardView)
    await flushPromises()

    expect(wrapper.text()).toContain('Deep Work Manhã')

    // Find timeline and emit complete event
    const timeline = wrapper.findComponent({ name: 'RoutineTimeline' })
    expect(timeline.exists()).toBe(true)
    expect(timeline.props('completedBlockIds')).toEqual(['block-1'])

    await timeline.vm.$emit('complete', 'block-2', 'completed')
    await flushPromises()

    expect(completeBlockSpy).toHaveBeenCalledWith('block-2', 'completed')
  })

  it('handles habit check-in with optimistic state update', async () => {
    const habitStore = useHabitStore()
    habitStore.habits = JSON.parse(JSON.stringify(mockHabits))
    const checkinSpy = vi.spyOn(habitStore, 'checkin').mockResolvedValue({
      id: 'log-1',
      habit_id: 'habit-1',
      date: '2026-10-30',
      value: 1500,
      completed: true,
      created_at: '2026-10-30T00:00:00Z',
    })

    const wrapper = mount(DashboardView)
    await flushPromises()

    const checklist = wrapper.findComponent({ name: 'HabitChecklist' })
    expect(checklist.exists()).toBe(true)

    // Emit increment check-in (e.g. +1500ml)
    await checklist.vm.$emit('checkin', 'habit-1', 1500, true)
    await flushPromises()

    expect(checkinSpy).toHaveBeenCalledWith('habit-1', expect.any(String), 1500, true)
    expect(habitStore.habits[0].today_value).toBe(2000)
    expect(habitStore.habits[0].streak.is_active_today).toBe(true)
    expect(habitStore.habits[0].streak.current_streak).toBe(6)
  })

  it('rolls back habit state on checkin API failure', async () => {
    const habitStore = useHabitStore()
    habitStore.habits = JSON.parse(JSON.stringify(mockHabits))
    vi.spyOn(habitStore, 'checkin').mockRejectedValue(new Error('Network error'))

    const wrapper = mount(DashboardView)
    await flushPromises()

    const checklist = wrapper.findComponent({ name: 'HabitChecklist' })
    await checklist.vm.$emit('checkin', 'habit-1', 1500, true)
    await flushPromises()

    // Restores original values
    expect(habitStore.habits[0].today_value).toBe(500)
    expect(habitStore.habits[0].streak.is_active_today).toBe(false)
    expect(habitStore.habits[0].streak.current_streak).toBe(5)
  })

  it('opens RoutineManageModal when clicking "Configurar Rotina"', async () => {
    const wrapper = mount(DashboardView)
    await flushPromises()

    const configureBtn = wrapper.findAll('button').find((b) => b.text().includes('Configurar Rotina'))
    expect(configureBtn).toBeDefined()

    await configureBtn!.trigger('click')
    await flushPromises()

    const modal = wrapper.findComponent({ name: 'RoutineManageModal' })
    expect(modal.exists()).toBe(true)
    expect(modal.props('isOpen')).toBe(true)
  })

  it('opens FocusMode modal when focus button is clicked', async () => {
    const focusStore = useFocusStore()
    const openFocusSpy = vi.spyOn(focusStore, 'openFocusModal')

    const wrapper = mount(DashboardView)
    await flushPromises()

    const focusBtn = wrapper.findAll('button').find((b) => b.text().includes('Modo Foco'))
    expect(focusBtn).toBeDefined()
    await focusBtn!.trigger('click')

    expect(openFocusSpy).toHaveBeenCalled()
  })
})
