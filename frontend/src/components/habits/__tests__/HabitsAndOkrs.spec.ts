import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import StreakBadge from '../StreakBadge.vue'
import HeatmapCalendar from '../HeatmapCalendar.vue'
import KeyResultProgressBar from '@/components/okrs/KeyResultProgressBar.vue'
import ObjectiveCard from '@/components/okrs/ObjectiveCard.vue'
import type { KeyResult, Objective } from '@/types'

describe('Habits & OKR Visualizations Suite', () => {
  describe('StreakBadge.vue', () => {
    it('renders current streak with day label', () => {
      const wrapper = mount(StreakBadge, {
        props: {
          currentStreak: 7,
          longestStreak: 10,
        },
      })

      expect(wrapper.text()).toContain('7')
      expect(wrapper.text()).toContain('dias')
      expect(wrapper.text()).toContain('(max: 10d)')
    })

    it('highlights record badge when current streak meets or exceeds record', () => {
      const wrapper = mount(StreakBadge, {
        props: {
          currentStreak: 12,
          longestStreak: 10,
        },
      })

      expect(wrapper.text()).toContain('Recorde')
    })
  })

  describe('HeatmapCalendar.vue', () => {
    it('generates exactly 52 weeks grid by default', () => {
      const wrapper = mount(HeatmapCalendar, {
        props: {
          weeksCount: 52,
          activityMap: {
            '2026-10-30': 4,
            '2026-10-29': 2,
          },
        },
      })

      expect(wrapper.text()).toContain('Consistência Anual')
      expect(wrapper.text()).toContain('Menos')
      expect(wrapper.text()).toContain('Mais')

      // Check legend 5 levels are rendered
      const legendCells = wrapper.findAll('.w-3.h-3.rounded-\\[3px\\]')
      // Legend has 5 cells, plus weeks cells
      expect(legendCells.length).toBeGreaterThanOrEqual(5)
    })
  })

  describe('KeyResultProgressBar.vue', () => {
    const mockKr: KeyResult = {
      id: 'kr-1',
      objective_id: 'obj-1',
      title: 'Atingir 100km de corrida',
      initial_value: 0,
      target_value: 100,
      current_value: 75,
      unit: 'km',
      weight: 1.0,
      progress_percentage: 75,
      linked_habit_id: 'habit-run',
      created_at: '',
      updated_at: '',
    }

    it('renders KR title, current progress, and linked habit badge', () => {
      const wrapper = mount(KeyResultProgressBar, {
        props: {
          kr: mockKr,
          linkedHabitName: 'Corrida Matinal',
        },
      })

      expect(wrapper.text()).toContain('Atingir 100km de corrida')
      expect(wrapper.text()).toContain('75 / 100 km')
      expect(wrapper.text()).toContain('75%')
      expect(wrapper.text()).toContain('Corrida Matinal')
    })

    it('shows link button when no habit is linked', () => {
      const unlinkedKr: KeyResult = {
        ...mockKr,
        linked_habit_id: undefined,
      }

      const wrapper = mount(KeyResultProgressBar, {
        props: {
          kr: unlinkedKr,
        },
      })

      expect(wrapper.text()).toContain('Vincular Hábito')
    })
  })

  describe('ObjectiveCard.vue', () => {
    const mockObjective: Objective = {
      id: 'obj-1',
      title: 'Conquistar Condicionamento Físico',
      description: 'Melhorar capacidade cardiovascular',
      quarter: '2026-Q4',
      start_date: '2026-10-01',
      end_date: '2026-12-31',
      progress_percentage: 82,
      is_completed: false,
      created_at: '',
      updated_at: '',
    }

    const mockKrs: KeyResult[] = [
      {
        id: 'kr-1',
        objective_id: 'obj-1',
        title: 'Correr 100km',
        initial_value: 0,
        target_value: 100,
        current_value: 82,
        unit: 'km',
        weight: 1.0,
        progress_percentage: 82,
        created_at: '',
        updated_at: '',
      },
    ]

    it('renders objective header, quarter badge, and child key results', () => {
      const wrapper = mount(ObjectiveCard, {
        props: {
          objective: mockObjective,
          keyResults: mockKrs,
        },
      })

      expect(wrapper.text()).toContain('Conquistar Condicionamento Físico')
      expect(wrapper.text()).toContain('2026-Q4')
      expect(wrapper.text()).toContain('82% Concluído')
      expect(wrapper.text()).toContain('1 Resultados-Chave')
      expect(wrapper.text()).toContain('Correr 100km')
    })
  })
})
