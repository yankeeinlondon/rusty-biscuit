import { describe, expect, it } from 'vitest'
import { statusToDotColor } from './types'

describe('statusToDotColor', () => {
  it('maps active to green', () => {
    expect(statusToDotColor('active')).toBe('green')
  })

  it('maps standby to amber', () => {
    expect(statusToDotColor('standby')).toBe('amber')
  })

  it('maps error to red', () => {
    expect(statusToDotColor('error')).toBe('red')
  })

  it('maps not_configured to grey', () => {
    expect(statusToDotColor('not_configured')).toBe('grey')
  })
})
