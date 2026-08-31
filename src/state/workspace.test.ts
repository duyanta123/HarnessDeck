import { beforeEach, describe, expect, it, vi } from 'vitest'

import * as ipc from '@/lib/ipc'
import { useDialog } from '@/state/dialog'
import { useHarness } from '@/state/harness'
import { useProjects } from '@/state/projects'
import { switchWorkspace } from '@/state/workspace'

vi.mock('@/lib/ipc')

const answer = async (taken: boolean): Promise<void> => {
  await vi.waitFor(() => expect(useDialog.getState().pending).not.toBeNull())
  useDialog.getState().settle(taken)
}

const roster = {
  projects: [
    { id: 'new-project', name: 'new-project', path: 'D:\\work', profile: 'web', lastOpenedAt: 1 },
  ],
  selected: 'new-project',
}

beforeEach(() => {
  vi.clearAllMocks()
  useDialog.setState({ pending: null })
  useProjects.setState({ roster: null, working: null, error: null })
  useHarness.setState({
    status: { phase: 'stopped' },
    error: null,
    inspect: vi.fn(async () => {}),
    stop: vi.fn(async () => {}),
    start: vi.fn(async () => {}),
  })
  vi.mocked(ipc.projectsAdd).mockResolvedValue(roster)
  vi.mocked(ipc.announce).mockResolvedValue()
})

describe('selecting a native workspace', () => {
  it('persists and refreshes it without restarting a stopped Harness', async () => {
    expect(await switchWorkspace('D:\\work')).toBe(true)
    expect(ipc.projectsAdd).toHaveBeenCalledWith('D:\\work')
    expect(useHarness.getState().inspect).toHaveBeenCalledTimes(1)
    expect(useHarness.getState().stop).not.toHaveBeenCalled()
    expect(useProjects.getState().working).toBeNull()
  })

  it('offers and performs the restart needed by a running Harness', async () => {
    useHarness.setState({
      status: { phase: 'ready', origin: 'http://127.0.0.1:8100', pid: 42 },
    })

    const changing = switchWorkspace('D:\\next')
    await answer(true)
    expect(await changing).toBe(true)
    expect(useHarness.getState().stop).toHaveBeenCalledTimes(1)
    expect(useHarness.getState().start).toHaveBeenCalledTimes(1)
    expect(useProjects.getState().working).toBeNull()
  })

  it('shows the backend refusal and leaves the current Harness alone', async () => {
    vi.mocked(ipc.projectsAdd).mockRejectedValueOnce('removable workspaces are blocked')

    expect(await switchWorkspace('E:\\unsafe')).toBe(false)
    expect(useProjects.getState().error).toBe('removable workspaces are blocked')
    expect(useHarness.getState().inspect).not.toHaveBeenCalled()
    expect(useProjects.getState().working).toBeNull()
    expect(useDialog.getState().pending).toMatchObject({
      kind: 'error',
      details: 'removable workspaces are blocked',
    })
  })

  it('releases the project write slot when the environment check fails', async () => {
    vi.mocked(useHarness.getState().inspect).mockRejectedValueOnce(new Error('probe failed'))

    expect(await switchWorkspace('D:\\broken')).toBe(false)
    expect(ipc.projectsAdd).toHaveBeenCalledWith('D:\\broken')
    expect(useProjects.getState().roster).toEqual(roster)
    expect(useProjects.getState().working).toBeNull()
    expect(useHarness.getState().stop).not.toHaveBeenCalled()
  })

  it('turns away a second folder while the first admission is in flight', async () => {
    let finish!: (value: typeof roster) => void
    vi.mocked(ipc.projectsAdd).mockReturnValueOnce(
      new Promise<typeof roster>((resolve) => {
        finish = resolve
      }),
    )

    const first = switchWorkspace('D:\\first')
    expect(useProjects.getState().working).toBe('D:\\first')
    expect(await switchWorkspace('D:\\second')).toBe(false)
    expect(ipc.projectsAdd).toHaveBeenCalledTimes(1)

    finish(roster)
    expect(await first).toBe(true)
    expect(useProjects.getState().working).toBeNull()
  })
})
