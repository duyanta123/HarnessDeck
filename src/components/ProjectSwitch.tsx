import { useEffect, type MouseEvent } from 'react'
import { ChevronDown, FolderKanban, Settings2 } from 'lucide-react'

import { t } from '@/lib/i18n'
import { useHarness } from '@/state/harness'
import { SEPARATOR, useMenu, type MenuEntry } from '@/state/menu'
import { switchProject, useProjects } from '@/state/projects'

interface ProjectSwitchProps {
  /** Open the settings pane, where projects are renamed and rebound. */
  onManage: () => void
}

/**
 * Which project this window works in, in the title bar.
 *
 * A project is the working directory plus the profile whose credentials and
 * plugins belong to it. It is a property of the window, not of a pane, so this
 * lives in the chrome next to the profile chip — and once the harness fills the
 * window, the title bar is the only place left that can still say which
 * project is running.
 */
export function ProjectSwitch({ onManage }: ProjectSwitchProps) {
  const roster = useProjects((state) => state.roster)
  const refresh = useProjects((state) => state.refresh)
  const phase = useHarness((state) => state.status.phase)

  // Read on arrival and whenever the harness settles. A fresh registry is
  // written on first run, and a project created in another window has to reach
  // this one through the shared-change subscription anyway; this first read
  // makes the chip true before any project has ever been selected here.
  useEffect(() => {
    if (phase === 'starting' || phase === 'restarting') return
    void refresh()
  }, [phase, refresh])

  const selected = roster?.selected ?? ''
  if (!selected || !roster) return null

  const current = roster.projects.find((project) => project.id === selected)

  const open = (event: MouseEvent<HTMLButtonElement>) => {
    const entries: MenuEntry[] = roster.projects.map((project) => ({
      label: project.name,
      checked: project.id === selected,
      run: () => void switchProject(project.id),
    }))
    entries.push(SEPARATOR, { label: t('project.manage'), icon: Settings2, run: onManage })

    const box = event.currentTarget.getBoundingClientRect()
    useMenu.getState().show(box.left, box.bottom + 4, entries)
  }

  return (
    <button
      type="button"
      aria-haspopup="menu"
      aria-label={t('project.switch')}
      data-hint={t('project.switch')}
      onClick={open}
      className="ml-1 inline-flex h-[22px] max-w-[164px] shrink-0 items-center gap-1.5 rounded-control border border-line px-1.5 text-[11.5px] text-muted transition-colors duration-100 hover:border-line-strong hover:bg-surface-2 hover:text-text"
    >
      <FolderKanban
        size={11}
        strokeWidth={2.1}
        className="shrink-0 text-brand"
        aria-hidden="true"
      />
      <span className="truncate">{current?.name ?? selected}</span>
      <ChevronDown size={10} strokeWidth={2.4} className="shrink-0 opacity-55" aria-hidden="true" />
    </button>
  )
}
