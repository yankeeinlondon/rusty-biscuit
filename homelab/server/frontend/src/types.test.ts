import { describe, expect, it } from 'vitest'
import { statusToDotColor } from './types'

describe('statusToDotColor', () => {
  it('maps active to green', () => {
    expect(statusToDotColor('active')).toBe('green')
  })

  it('maps standby to amber', () => {
    expect(statusToDotColor('standby')).toBe('amber')
  })

  it('maps off to grey', () => {
    expect(statusToDotColor('off')).toBe('grey')
  })

  it('maps not_configured to grey', () => {
    expect(statusToDotColor('not_configured')).toBe('grey')
  })
})
