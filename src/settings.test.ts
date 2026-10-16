import { describe, expect, it } from 'vitest'
import { resolveLanguage, resolveTheme, translate } from './settings'

describe('user interface preferences', () => {
  it('follows Chinese system languages and falls back to English', () => {
    expect(resolveLanguage('system', 'zh-Hans-CN')).toBe('zh-CN')
    expect(resolveLanguage('system', 'zh-TW')).toBe('zh-CN')
    expect(resolveLanguage('system', 'fr-FR')).toBe('en')
  })

  it('keeps an explicit language choice when the system language differs', () => {
    expect(resolveLanguage('en', 'zh-CN')).toBe('en')
    expect(resolveLanguage('zh-CN', 'en-US')).toBe('zh-CN')
  })

  it('follows system appearance unless the user selects a theme', () => {
    expect(resolveTheme('system', true)).toBe('dark')
    expect(resolveTheme('system', false)).toBe('light')
    expect(resolveTheme('light', true)).toBe('light')
    expect(resolveTheme('dark', false)).toBe('dark')
  })

  it('uses translated interface strings and interpolates progress values', () => {
    expect(translate('zh-CN', 'app.startRecording')).toBe('开始录制')
    expect(translate('en', 'app.speechModelDownload', { percent: 42 })).toBe('Speech model download: 42%')
  })
})
