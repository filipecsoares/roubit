<script setup lang="ts">
import { ref, computed } from 'vue'

const props = withDefaults(
  defineProps<{
    activityMap?: Record<string, number> // 'YYYY-MM-DD' -> count
    endDate?: Date
    weeksCount?: number
  }>(),
  {
    activityMap: () => ({}),
    endDate: () => new Date(),
    weeksCount: 52,
  }
)

interface HeatmapDay {
  date: string // 'YYYY-MM-DD'
  formattedDate: string
  count: number
  level: number
  dayOfWeek: number // 0 = Mon, 6 = Sun
}

interface HeatmapWeek {
  weekIndex: number
  days: (HeatmapDay | null)[]
}

const hoveredCell = ref<{
  date: string
  formattedDate: string
  count: number
  x: number
  y: number
} | null>(null)

const dateFormatter = new Intl.DateTimeFormat('pt-BR', {
  weekday: 'short',
  day: 'numeric',
  month: 'short',
  year: 'numeric',
})

// Generate weeks structure ending on the week of props.endDate
const weeks = computed<HeatmapWeek[]>(() => {
  const result: HeatmapWeek[] = []
  const end = new Date(props.endDate)
  // Align end to Sunday of the current week (or today)
  // JavaScript getDay(): 0 is Sunday, 1 is Monday ... 6 is Saturday
  const currentDayOfWeek = (end.getDay() + 6) % 7 // 0 = Mon, 6 = Sun

  // Total days to generate = weeksCount * 7
  const totalDays = props.weeksCount * 7
  const start = new Date(end)
  start.setDate(end.getDate() - totalDays + (6 - currentDayOfWeek) + 1)

  const cursor = new Date(start)

  for (let w = 0; w < props.weeksCount; w++) {
    const days: (HeatmapDay | null)[] = []
    for (let d = 0; d < 7; d++) {
      const year = cursor.getFullYear()
      const month = String(cursor.getMonth() + 1).padStart(2, '0')
      const day = String(cursor.getDate()).padStart(2, '0')
      const dateStr = `${year}-${month}-${day}`

      // Check if cursor is in the future relative to endDate
      if (cursor > end) {
        days.push(null)
      } else {
        const count = props.activityMap[dateStr] || 0
        let level = 0
        if (count >= 4) level = 4
        else if (count >= 3) level = 3
        else if (count >= 2) level = 2
        else if (count >= 1) level = 1

        const formattedDate = dateFormatter.format(cursor)

        days.push({
          date: dateStr,
          formattedDate,
          count,
          level,
          dayOfWeek: d,
        })
      }
      cursor.setDate(cursor.getDate() + 1)
    }
    result.push({ weekIndex: w, days })
  }

  return result
})

function getLevelColor(level: number): string {
  switch (level) {
    case 1:
      return '#064e3b' // emerald-900
    case 2:
      return '#047857' // emerald-700
    case 3:
      return '#10b981' // emerald-500
    case 4:
      return '#34d399' // emerald-400
    case 0:
    default:
      return '#1e293b' // slate-800
  }
}

function showTooltip(event: UIEvent, day: HeatmapDay) {
  const target = event.target as HTMLElement
  if (!target || typeof target.getBoundingClientRect !== 'function') return
  const rect = target.getBoundingClientRect()
  hoveredCell.value = {
    date: day.date,
    formattedDate: day.formattedDate,
    count: day.count,
    x: rect.left + rect.width / 2,
    y: rect.top - 8,
  }
}

function hideTooltip() {
  hoveredCell.value = null
}
</script>

<template>
  <div class="relative p-4 sm:p-5 rounded-2xl bg-surface-card border border-surface-border overflow-hidden glow-card">
    <!-- Header -->
    <div class="flex flex-col sm:flex-row sm:items-center justify-between mb-4 gap-2">
      <div class="space-y-0.5">
        <h3 class="text-sm font-semibold text-white">Consistência Anual</h3>
        <p class="text-xs text-slate-400">Mapa de calor dos últimos 365 dias de check-ins</p>
      </div>

      <!-- Legend -->
      <div class="flex items-center gap-2 text-xs text-slate-400 self-start sm:self-auto">
        <span class="text-[11px] text-slate-500 sm:hidden mr-1">Deslize para ver</span>
        <span>Menos</span>
        <div class="flex items-center gap-1">
          <span
            v-for="lvl in [0, 1, 2, 3, 4]"
            :key="lvl"
            class="w-3 h-3 rounded-[3px] border border-black/20"
            :style="{ backgroundColor: getLevelColor(lvl) }"
          />
        </div>
        <span>Mais</span>
      </div>
    </div>

    <!-- Heatmap Grid Container with touch momentum horizontal scrolling -->
    <div class="overflow-x-auto pb-2 scrollbar-thin scrollbar-thumb-surface-border overscroll-x-contain" style="-webkit-overflow-scrolling: touch;">
      <div class="inline-flex gap-1.5 min-w-max py-1">
        <!-- Day labels column (Mon, Wed, Fri) -->
        <div class="flex flex-col justify-between py-0.5 text-[10px] text-slate-500 font-mono pr-1 select-none">
          <span class="h-3 flex items-center">Seg</span>
          <span class="h-3 flex items-center"></span>
          <span class="h-3 flex items-center">Qua</span>
          <span class="h-3 flex items-center"></span>
          <span class="h-3 flex items-center">Sex</span>
          <span class="h-3 flex items-center"></span>
          <span class="h-3 flex items-center"></span>
        </div>

        <!-- Weeks columns -->
        <div
          v-for="week in weeks"
          :key="week.weekIndex"
          class="flex flex-col gap-1"
        >
          <div
            v-for="(day, dayIdx) in week.days"
            :key="dayIdx"
            :role="day ? 'gridcell' : undefined"
            :tabindex="day ? 0 : undefined"
            :aria-label="day ? `${day.count} ${day.count === 1 ? 'check-in' : 'check-ins'} em ${day.formattedDate}` : undefined"
            class="w-3 h-3 rounded-[3px] transition-transform duration-150 focus:outline-none focus:ring-1 focus:ring-emerald-400"
            :class="day ? 'hover:scale-125 focus:scale-125 cursor-pointer border border-black/20' : 'opacity-0 pointer-events-none'"
            :style="day ? { backgroundColor: getLevelColor(day.level) } : {}"
            @mouseenter="day && showTooltip($event, day)"
            @mouseleave="hideTooltip"
            @focus="day && showTooltip($event, day)"
            @blur="hideTooltip"
          />
        </div>
      </div>
    </div>

    <!-- Floating Tooltip -->
    <Teleport to="body">
      <div
        v-if="hoveredCell"
        class="fixed z-50 pointer-events-none transform -translate-x-1/2 -translate-y-full px-2.5 py-1.5 rounded-lg bg-slate-900 border border-slate-700 shadow-xl text-xs whitespace-nowrap"
        :style="{ left: `${hoveredCell.x}px`, top: `${hoveredCell.y}px` }"
      >
        <span class="font-semibold text-white font-mono">
          {{ hoveredCell.count }} {{ hoveredCell.count === 1 ? 'check-in' : 'check-ins' }}
        </span>
        <span class="text-slate-400 ml-1.5 font-sans">em {{ hoveredCell.formattedDate }}</span>
      </div>
    </Teleport>
  </div>
</template>
