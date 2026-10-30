import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import DailyProgressGauge from '../DailyProgressGauge.vue'
import TimeBlockCard from '../TimeBlockCard.vue'
import RoutineTimeline from '../RoutineTimeline.vue'
import HabitCheckinRow from '../HabitCheckinRow.vue'
import HabitChecklist from '../HabitChecklist.vue'
import type { TimeBlock, Habit, StreakMetrics, HabitWithStreakResponse } from '@/types'

describe('Dashboard Components Suite', () => {
  describe('DailyProgressGauge.vue', () => {
    it('renders the calculated percentage and breakdown', () => {
      const wrapper = mount(DailyProgressGauge, {
        props: {
          percentage: 75,
          routinePercentage: 50,
          habitPercentage: 100,
        },
      })

      expect(wrapper.text()).toContain('75%')
      expect(wrapper.text()).toContain('Score')
      expect(wrapper.text()).toContain('Rotinas:')
      expect(wrapper.text()).toContain('50%')
      expect(wrapper.text()).toContain('Hábitos:')
      expect(wrapper.text()).toContain('100%')
    })
  })

  describe('TimeBlockCard.vue', () => {
    const mockBlock: TimeBlock = {
      id: 'block-1',
      routine_id: 'routine-1',
      name: 'Foco Profundo',
      start_time: '08:00:00',
      end_time: '10:00:00',
      category: 'Trabalho',
      color_hex: '#6366f1',
      order_index: 0,
      created_at: '2026-10-30T00:00:00Z',
    }

    it('formats time and displays active indicator when active', () => {
      const wrapper = mount(TimeBlockCard, {
        props: {
          block: mockBlock,
          isActive: true,
        },
      })

      expect(wrapper.text()).toContain('08:00 - 10:00')
      expect(wrapper.text()).toContain('Agora')
      expect(wrapper.text()).toContain('Foco Profundo')
      expect(wrapper.text()).toContain('Trabalho')
    })

    it('emits complete event when action buttons are clicked', async () => {
      const wrapper = mount(TimeBlockCard, {
        props: {
          block: mockBlock,
          isActive: false,
        },
      })

      const buttons = wrapper.findAll('button')
      expect(buttons.length).toBe(2)

      // Click complete button
      await buttons[0].trigger('click')
      expect(wrapper.emitted('complete')).toBeTruthy()
      expect(wrapper.emitted('complete')![0]).toEqual(['block-1', 'completed'])

      // Click skip button
      await buttons[1].trigger('click')
      expect(wrapper.emitted('complete')![1]).toEqual(['block-1', 'skipped'])
    })
  })

  describe('RoutineTimeline.vue', () => {
    it('renders empty state when no blocks are provided', () => {
      const wrapper = mount(RoutineTimeline, {
        props: {
          blocks: [],
        },
      })

      expect(wrapper.text()).toContain('Nenhum bloco de rotina ativo')
    })

    it('renders timeline with chronological order', () => {
      const blocks: TimeBlock[] = [
        {
          id: 'b-2',
          routine_id: 'r-1',
          name: 'Almoço',
          start_time: '12:00:00',
          end_time: '13:00:00',
          category: 'Saúde',
          color_hex: '#10b981',
          order_index: 1,
          created_at: '',
        },
        {
          id: 'b-1',
          routine_id: 'r-1',
          name: 'Manhã',
          start_time: '08:00:00',
          end_time: '10:00:00',
          category: 'Foco',
          color_hex: '#6366f1',
          order_index: 0,
          created_at: '',
        },
      ]

      const wrapper = mount(RoutineTimeline, {
        props: {
          blocks,
          activeBlockId: 'b-1',
        },
      })

      const blockCards = wrapper.findAllComponents(TimeBlockCard)
      expect(blockCards.length).toBe(2)
      // Check chronological sorting
      expect(blockCards[0].props('block').name).toBe('Manhã')
      expect(blockCards[1].props('block').name).toBe('Almoço')
    })
  })

  describe('HabitCheckinRow.vue', () => {
    const mockHabit: Habit = {
      id: 'h-1',
      name: 'Beber 2L de água',
      description: 'Manter hidratação',
      habit_type: 'numeric',
      target_value: 2000,
      unit: 'ml',
      frequency_days: 127,
      is_archived: false,
      color_hex: '#3b82f6',
      created_at: '',
      updated_at: '',
    }

    const mockStreak: StreakMetrics = {
      current_streak: 5,
      longest_streak: 12,
      is_active_today: false,
    }

    it('renders habit info, streak count, and numeric increment button', async () => {
      const wrapper = mount(HabitCheckinRow, {
        props: {
          habit: mockHabit,
          streak: mockStreak,
          currentValue: 500,
          isCompletedToday: false,
        },
      })

      expect(wrapper.text()).toContain('Beber 2L de água')
      expect(wrapper.text()).toContain('5 dias')
      expect(wrapper.text()).toContain('+250 ml')
      expect(wrapper.text()).toContain('500 / 2000 ml')

      // Click increment button
      const incrementBtn = wrapper.find('button[title="Adicionar +250"]')
      await incrementBtn.trigger('click')

      expect(wrapper.emitted('checkin')).toBeTruthy()
      expect(wrapper.emitted('checkin')![0]).toEqual(['h-1', 250, true])
    })

    it('emits binary toggle when check button is clicked', async () => {
      const binaryHabit: Habit = {
        ...mockHabit,
        habit_type: 'binary',
        target_value: 1,
      }

      const wrapper = mount(HabitCheckinRow, {
        props: {
          habit: binaryHabit,
          streak: mockStreak,
          isCompletedToday: false,
        },
      })

      const toggleBtn = wrapper.find('button[title="Concluir hábito"]')
      await toggleBtn.trigger('click')

      expect(wrapper.emitted('checkin')![0]).toEqual(['h-1', 1, false])
    })
  })

  describe('HabitChecklist.vue', () => {
    const mockHabits: HabitWithStreakResponse[] = [
      {
        habit: {
          id: 'h-1',
          name: 'Meditação',
          habit_type: 'binary',
          target_value: 1,
          frequency_days: 127,
          is_archived: false,
          color_hex: '#6366f1',
          created_at: '',
          updated_at: '',
        },
        streak: {
          current_streak: 3,
          longest_streak: 5,
          is_active_today: true,
        },
      },
      {
        habit: {
          id: 'h-2',
          name: 'Leitura',
          habit_type: 'binary',
          target_value: 1,
          frequency_days: 127,
          is_archived: false,
          color_hex: '#10b981',
          created_at: '',
          updated_at: '',
        },
        streak: {
          current_streak: 0,
          longest_streak: 2,
          is_active_today: false,
        },
      },
    ]

    it('filters habits by pending and completed tabs', async () => {
      const wrapper = mount(HabitChecklist, {
        props: {
          habits: mockHabits,
        },
      })

      // Default: "Todos"
      expect(wrapper.findAllComponents(HabitCheckinRow).length).toBe(2)

      // Switch to "Pendentes"
      const buttons = wrapper.findAll('button')
      await buttons[1].trigger('click') // Tab "Pendentes"
      expect(wrapper.findAllComponents(HabitCheckinRow).length).toBe(1)
      expect(wrapper.text()).toContain('Leitura')
      expect(wrapper.text()).not.toContain('Meditação')

      // Switch to "Concluídos"
      await buttons[2].trigger('click') // Tab "Concluídos"
      expect(wrapper.findAllComponents(HabitCheckinRow).length).toBe(1)
      expect(wrapper.text()).toContain('Meditação')
      expect(wrapper.text()).not.toContain('Leitura')
    })
  })
})
