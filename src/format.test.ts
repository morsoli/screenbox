import { describe, expect, it } from 'vitest'
import { clock, validTrim } from './format'

describe('edit validation', () => {
  it('keeps trim endpoints inside the source and rejects empty spans', () => {
    expect(validTrim(500, 2500, 3000)).toBe(true)
    expect(validTrim(3000, 3000, 3000)).toBe(false)
    expect(validTrim(-1, 2000, 3000)).toBe(false)
    expect(validTrim(0, 4000, 3000)).toBe(false)
  })
  it('formats elapsed recording time', () => { expect(clock(61000)).toBe('01:01') })
})
