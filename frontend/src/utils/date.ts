/**
 * Utility functions for local timezone-aware date formatting and manipulation.
 * Avoids UTC shifts caused by .toISOString().split('T')[0] in timezones behind UTC (e.g., UTC-3).
 */

/**
 * Returns a date string formatted as `YYYY-MM-DD` based on the local timezone.
 */
export function formatLocalDate(date: Date = new Date()): string {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

/**
 * Parses a `YYYY-MM-DD` date string into a local Date object (at local midnight 00:00:00).
 */
export function parseLocalDate(dateStr: string): Date {
  const [year, month, day] = dateStr.split('-').map(Number)
  return new Date(year, month - 1, day)
}

/**
 * Adds or subtracts days from a `YYYY-MM-DD` date string and returns a new `YYYY-MM-DD` string in local time.
 */
export function addDaysLocal(dateStr: string, offset: number): string {
  const [year, month, day] = dateStr.split('-').map(Number)
  const d = new Date(year, month - 1, day + offset)
  return formatLocalDate(d)
}
