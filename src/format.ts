export function clock(ms: number): string {
  const seconds = Math.max(0, Math.floor(ms / 1000))
  return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`
}
export function dateLabel(iso: string, locale: 'zh-CN' | 'en' = 'en'): string {
  return new Intl.DateTimeFormat(locale === 'zh-CN' ? 'zh-CN' : 'en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }).format(new Date(iso))
}
export function clamp(value: number, min: number, max: number): number { return Math.min(max, Math.max(min, value)) }
export function validTrim(start: number, end: number, duration: number): boolean {
  return Number.isFinite(start) && Number.isFinite(end) && start >= 0 && end <= duration && end - start >= 100
}
