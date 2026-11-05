import { describe, it, expect } from 'vitest'
import { formatLocalDate, parseLocalDate, addDaysLocal } from '../date'

describe('Date Utilities (IMP-04.4)', () => {
  it('formats local date correctly as YYYY-MM-DD', () => {
    const fixedDate = new Date(2026, 10, 4, 22, 30, 0) // Month index 10 = November
    expect(formatLocalDate(fixedDate)).toBe('2026-11-04')
  })

  it('correctly pads single-digit months and days', () => {
    const fixedDate = new Date(2026, 0, 5, 12, 0, 0) // Month index 0 = January 5
    expect(formatLocalDate(fixedDate)).toBe('2026-01-05')
  })

  it('parses YYYY-MM-DD string into a local Date instance', () => {
    const parsed = parseLocalDate('2026-11-04')
    expect(parsed.getFullYear()).toBe(2026)
    expect(parsed.getMonth()).toBe(10) // November
    expect(parsed.getDate()).toBe(4)
  })

  it('adds and subtracts days crossing month and year boundaries correctly', () => {
    expect(addDaysLocal('2026-11-04', 1)).toBe('2026-11-05')
    expect(addDaysLocal('2026-11-04', -1)).toBe('2026-11-03')
    expect(addDaysLocal('2026-11-01', -1)).toBe('2026-10-31')
    expect(addDaysLocal('2026-12-31', 1)).toBe('2027-01-01')
    expect(addDaysLocal('2026-01-01', -1)).toBe('2025-12-31')
  })

  it('handles leap years correctly', () => {
    // 2024 is a leap year (Feb 29 exists)
    expect(addDaysLocal('2024-02-28', 1)).toBe('2024-02-29')
    expect(addDaysLocal('2024-02-29', 1)).toBe('2024-03-01')

    // 2026 is not a leap year (Feb 28 -> Mar 1)
    expect(addDaysLocal('2026-02-28', 1)).toBe('2026-03-01')
  })
})
