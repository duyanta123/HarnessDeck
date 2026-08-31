import { describe, expect, it, vi } from 'vitest'

import {
  HARNESSDECK_PROTOCOL,
  HARNESSDECK_HOST_PROTOCOL,
  getHarnessDeck,
  getHarnessDeckHost,
  hasHarnessDeckCapability,
  onHarnessDeckWorkspaceDrop,
  requireHarnessDeck,
  requireHarnessDeckHost,
} from './index.js'

function desktop() {
  const dispose = vi.fn()
  const onDrop = vi.fn(() => dispose)
  return {
    contract: { protocol: HARNESSDECK_PROTOCOL, workspace: { onDrop } },
    dispose,
    onDrop,
  }
}

describe('HarnessDeck SDK', () => {
  it('detects exactly Protocol 3 and rejects missing or malformed scopes', () => {
    const value = desktop().contract
    expect(getHarnessDeck({ harnessDeck: value })).toBe(value)
    expect(getHarnessDeck({ harnessDeck: { protocol: 2 } })).toBeUndefined()
    expect(getHarnessDeck({ harnessDeck: null })).toBeUndefined()
    expect(getHarnessDeck(null)).toBeUndefined()
  })

  it('reports absent and incompatible hosts without leaking implementation details', () => {
    expect(() => requireHarnessDeck({})).toThrow('Protocol 3 is not available')
    expect(() => requireHarnessDeck({ harnessDeck: { protocol: 4 } })).toThrow(
      'Protocol 4 is not supported',
    )
  })

  it('checks only a readable non-empty advertised capability', () => {
    expect(hasHarnessDeckCapability({ capabilities: ['workspace'] }, 'workspace')).toBe(true)
    expect(hasHarnessDeckCapability({ capabilities: ['workspace'] }, '')).toBe(false)
    expect(hasHarnessDeckCapability({ capabilities: null }, 'workspace')).toBe(false)
    expect(hasHarnessDeckCapability(undefined, 'workspace')).toBe(false)
  })

  it('registers workspace drops and returns the native disposer', () => {
    const value = desktop()
    const handler = vi.fn()
    const dispose = onHarnessDeckWorkspaceDrop(handler, { harnessDeck: value.contract })

    expect(value.onDrop).toHaveBeenCalledWith(handler)
    expect(dispose).toBe(value.dispose)
  })

  it('rejects an invalid drop handler before consulting the host', () => {
    const value = desktop()
    // @ts-expect-error Runtime callers can still arrive from untyped JavaScript.
    expect(() => onHarnessDeckWorkspaceDrop(null, { harnessDeck: value.contract })).toThrow(TypeError)
    expect(value.onDrop).not.toHaveBeenCalled()
  })

  it('detects the read-only Host contract through Cordis or direct properties', () => {
    const host = { protocol: HARNESSDECK_HOST_PROTOCOL }
    expect(getHarnessDeckHost({ get: vi.fn(() => host) })).toBe(host)
    expect(getHarnessDeckHost({ harnessDeckHost: host })).toBe(host)
    expect(getHarnessDeckHost({ harnessDeckHost: { protocol: 2 } })).toBeUndefined()
    expect(getHarnessDeckHost(null)).toBeUndefined()
  })

  it('reports absent and incompatible Host contracts', () => {
    expect(() => requireHarnessDeckHost({})).toThrow('Host Protocol 1 is not available')
    expect(() => requireHarnessDeckHost({ harnessDeckHost: { protocol: 2 } })).toThrow(
      'Host Protocol 2 is not supported',
    )
  })
})
